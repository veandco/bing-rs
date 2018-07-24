// std
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// serde_json
use serde_json;

// ws
use ws::{self, CloseCode, Error, ErrorKind, Handshake, Message, Request, Result};

// url
use url::Url;

// uuid
use uuid::Uuid;

// chrono
use chrono::prelude::*;

// internal
use speech::{Format, Hypothesis, Phrase};

// Enum of event that comes from client
pub enum ClientEvent {
    Audio(Vec<u8>),
    Disconnect,
}

/// Enum of event that comes from server
pub enum ServerEvent {
    Connect(ws::Sender),
    Disconnect,
    TurnStart,
    SpeechStartDetected,
    SpeechHypothesis(Hypothesis),
    SpeechEndDetected,
    SpeechPhrase(Phrase),
    TurnEnd,
    Unknown,
}

/// Websocket instance that keeps the connection to the server alive
pub struct Instance {
    pub ws_out: ws::Sender,
    pub token: String,
    pub thread_out: Sender<ServerEvent>,
    pub format: Format,
}

/// Websocket handle for the client to use
#[no_mangle]
#[repr(C)]
pub struct Handle {
    pub send_tx: Sender<ClientEvent>,
    pub ws_out: Arc<Mutex<Option<ws::Sender>>>,
    pub send_thread: Option<JoinHandle<()>>,
    pub recv_thread: Option<JoinHandle<()>>,
}

impl Handle {
    pub fn close(&mut self) {
        self.send_tx.send(ClientEvent::Disconnect).unwrap();

        let ws_out_option = &(*self.ws_out.lock().unwrap());
        if let Some(ws_out) = ws_out_option {
            ws_out.close(ws::CloseCode::Normal).unwrap();
        }

        self.send_thread.take().unwrap().join().unwrap();
        self.recv_thread.take().unwrap().join().unwrap();
    }
}

/// Trait to be implemented by the client for handling Bing Speech recognition events
pub trait Handler {
    fn on_turn_start(&mut self) {}
    fn on_turn_end(&mut self) {}
    fn on_speech_start_detected(&mut self) {}
    fn on_speech_hypothesis(&mut self, Hypothesis) {}
    fn on_speech_phrase(&mut self, Phrase) {}
    fn on_speech_end_detected(&mut self) {}
}

impl Instance {
    fn parse_server_message(&self, msg: Message) -> Result<()> {
        match msg {
            Message::Text(text) => self.parse_server_message_text(&text)?,
            _ => warn!(target: "parse_server_message()", "Unimplemented"),
        };

        Ok(())
    }

    fn parse_server_message_text(&self, text: &str) -> Result<()> {
        info!(target: "parse_server_message_text()", "Received From Server: {}", text);

        let sections: Vec<&str> = text.split("\r\n\r\n").collect();
        let header = sections[0];
        let body = sections[1];

        let header_lines: Vec<&str> = header.split("\r\n").collect();
        for line in header_lines {
            let kv: Vec<&str> = line.split(':').collect();
            let key = kv[0].trim();
            let value = kv[1].trim();
            if key == "Path" {
                let event = match value {
                    "turn.start" => ServerEvent::TurnStart,
                    "turn.end" => ServerEvent::TurnEnd,
                    "speech.startDetected" => ServerEvent::SpeechStartDetected,
                    "speech.hypothesis" => {
                        let json = serde_json::from_slice(body.as_bytes()).unwrap();
                        ServerEvent::SpeechHypothesis(json)
                    },
                    "speech.phrase" => {
                        let value: serde_json::Value = serde_json::from_str(body).unwrap();
                        ServerEvent::SpeechPhrase(Phrase::from_json_value(&value).unwrap())
                    },
                    "speech.endDetected" => ServerEvent::SpeechEndDetected,
                    _ => ServerEvent::Unknown,
                };
                self.thread_out.send(event).unwrap();
            }
        }

        Ok(())
    }
}

/// Utility struct for communicating with Bing Speech API via Websocket using their protocol
/// Right now, it primarily serves to generate and keep track of audio UUID.
#[derive(Default)]
pub struct Protocol {
    audio_uuid: Option<String>,
}

impl Protocol {
    /// Creates a new Protocol instance
    pub fn new() -> Protocol {
        Protocol { audio_uuid: None }
    }

    /// Send speech configuration data to Bing Speech API via Websocket
    pub fn config(sender: &ws::Sender, cfg: &ConfigPayload) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let config_text = serde_json::to_string(&cfg).unwrap();
        let text = format!(
            "Path: {}\r\nX-RequestId: {}\r\nX-Timestamp: {}\r\nContent-Type: {}\r\n\r\n{}",
            "speech.config",
            generate_uuid(),
            now,
            "application/json; charset=utf-8",
            config_text
        );
        let msg = Message::Text(text);
        sender.send(msg)
    }

    /// Send audio data to Bing Speech API via Websocket
    pub fn audio(&mut self, sender: &ws::Sender, audio: &[u8]) -> Result<()> {
        let uuid = if let Some(ref uuid) = self.audio_uuid {
            uuid.clone()
        } else {
            let new_uuid = generate_uuid();
            self.audio_uuid = Some(new_uuid.clone());
            new_uuid
        };

        let mut data = Vec::new();
        let now = Local::now().to_rfc3339();
        let text = format!(
            "Path: {}\r\nX-RequestId: {}\r\nX-Timestamp: {}\r\nContent-Type: {}\r\n\r\n",
            "audio", uuid, now, "audio/x-wav",
        );

        let header_len = text.len() as u16;
        let s1 = ((header_len >> 8) & 0xFF) as u8;
        let s2 = (header_len & 0xFF) as u8;
        data.push(s1);
        data.push(s2);
        data.extend_from_slice(text.as_bytes());
        data.extend_from_slice(&audio);

        let msg = Message::Binary(data);
        sender.send(msg)
    }
}

impl ws::Handler for Instance {
    fn build_request(&mut self, url: &Url) -> Result<Request> {
        let mut request = Request::from_url(url)?;
        {
            let headers = request.headers_mut();
            let token = format!("Bearer {}", &self.token).as_bytes().to_vec();
            let connection_id = Uuid::new_v4()
                .to_string()
                .replace("-", "")
                .as_bytes()
                .to_vec();
            headers.push(("Authorization".to_string(), token));
            headers.push(("X-ConnectionId".to_string(), connection_id));
        }
        Ok(request)
    }

    fn on_open(&mut self, _shake: Handshake) -> Result<()> {
        self.thread_out
            .send(ServerEvent::Connect(self.ws_out.clone()))
            .map_err(|err| {
                Error::new(
                    ErrorKind::Internal,
                    format!("Unable to communicate between threads: {:?}.", err),
                )
            })
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        self.parse_server_message(msg)?;
        Ok(())
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        if let Err(err) = self.thread_out.send(ServerEvent::Disconnect) {
            error!(target: "on_close()", "{}", err);
        }
    }

    fn on_error(&mut self, err: Error) {
        error!(target: "on_error()", "{}", err);
    }
}

impl Handler for Protocol {
    fn on_turn_end(&mut self) {
        self.audio_uuid = None;
    }
}

/// Configuration struct for "speech.config" payload
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigPayload {
    pub context: ConfigPayloadContext,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigPayloadContext {
    pub system: ConfigPayloadContextSystem,
    pub os: ConfigPayloadContextOs,
    pub device: ConfigPayloadContextDevice,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigPayloadContextSystem {
    pub version: String,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigPayloadContextOs {
    pub platform: String,
    pub name: String,
    pub version: String,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigPayloadContextDevice {
    pub manufacturer: String,
    pub model: String,
    pub version: String,
}

/// Utility function for generating UUID without hyphens
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "")
}
