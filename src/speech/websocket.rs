use std::sync::{Arc, Mutex};
use std::thread;
use url::Url;
use uuid::Uuid;
use ws;

use chrono::prelude::*;
use serde_json;

use speech::*;

/// Server event handler
pub trait Handler {
    fn on_turn_start(&mut self) {}
    fn on_turn_end(&mut self) {}
    fn on_speech_start(&mut self) {}
    fn on_speech_end(&mut self) {}
    fn on_speech_hypothesis(&mut self, _hypothesis: Hypothesis) {}
    fn on_speech_phrase(&mut self, _phrase: Phrase) {}
}

/// Enum of event that comes from server
#[derive(Clone)]
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

pub struct Websocket {
    sender: Arc<Mutex<Option<ws::Sender>>>,
    audio_uuid: Arc<Mutex<Option<String>>>,
}

pub struct MyHandler {
    token: Arc<Mutex<String>>,
    handler: Arc<Mutex<Handler + Send + Sync>>,
    audio_uuid: Arc<Mutex<Option<String>>>,
}

struct Factory {
    sender: Arc<Mutex<Option<ws::Sender>>>,
    token: Arc<Mutex<String>>,
    handler: Arc<Mutex<Handler + Send + Sync>>,
    audio_uuid: Arc<Mutex<Option<String>>>,
}

impl ws::Factory for Factory {
    type Handler = MyHandler;

    fn connection_made(&mut self, sender: ws::Sender) -> MyHandler {
        *self.sender.lock().unwrap() = Some(sender);

        MyHandler {
            token: self.token.clone(),
            handler: self.handler.clone(),
            audio_uuid: self.audio_uuid.clone(),
        }
    }

    fn client_connected(&mut self, sender: ws::Sender) -> MyHandler {
        *self.sender.lock().unwrap() = Some(sender);

        MyHandler {
            token: self.token.clone(),
            handler: self.handler.clone(),
            audio_uuid: self.audio_uuid.clone(),
        }
    }
}

impl Websocket {
    pub fn new() -> Websocket {
        let sender = Arc::new(Mutex::new(None));
        let audio_uuid = Arc::new(Mutex::new(None));

        Websocket { sender, audio_uuid }
    }

    /// Open the Websocket connection
    pub fn connect(
        &self,
        token: Arc<Mutex<String>>,
        mode: &Mode,
        format: &Format,
        is_custom_speech: bool,
        endpoint_id: &str,
        handler: Arc<Mutex<Handler + Send + Sync>>,
    ) -> Result<()> {
        // Create new WebSocket instance
        let mut ws = ws::WebSocket::new(Factory {
            sender: self.sender.clone(),
            token: token.clone(),
            handler: handler.clone(),
            audio_uuid: self.audio_uuid.clone(),
        }).unwrap();

        // Connect to Bing Speech Websocket endpoint
        let url = Self::build_url(mode, format, is_custom_speech, endpoint_id);
        ws.connect(url.parse()?)?;
        thread::spawn(move || {
            ws.run().unwrap();
        });

        Ok(())
    }

    /// Send speech configuration data to Bing Speech API via Websocket
    pub fn config(&mut self, cfg: &ConfigPayload) -> ws::Result<()> {
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

        let msg = ws::Message::Text(text);
        if let Some(ref mut s) = *self.sender.lock().unwrap() {
            s.send(msg)
        } else {
            Ok(())
        }
    }

    /// Send audio data to Bing Speech API via Websocket
    pub fn audio(&mut self, audio: &[u8]) -> ws::Result<()> {
        if let Ok(sender_guard) = self.sender.lock() {
            if let Some(ref sender) = *sender_guard {
                let mut v = self.audio_uuid.lock().unwrap();
                let uuid = if let Some(uuid) = v.clone() {
                    uuid.clone()
                } else {
                    let uuid = generate_uuid();
                    *v = Some(uuid.clone());
                    uuid
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

                let msg = ws::Message::Binary(data);
                return sender.send(msg);
            }
        }

        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        let sender = self.sender.lock().unwrap();

        if let Some(ref sender) = *sender {
            Ok(sender.shutdown()?)
        } else {
            Ok(())
        }
    }

    fn build_url(
        mode: &Mode,
        format: &Format,
        is_custom_speech: bool,
        endpoint_id: &str,
    ) -> String {
        let language = match mode {
            Mode::Interactive(language) | Mode::Dictation(language) => language.to_string(),
            Mode::Conversation(language) => language.to_string(),
        };
        if is_custom_speech {
            format!(
                "wss://westus.stt.speech.microsoft.com/speech/recognition/{}/cognitiveservices/v1?cid={}&language={}&format={}",
                mode,
                endpoint_id,
                language,
                format
            )
        } else {
            format!(
                "wss://speech.platform.bing.com/speech/recognition/{}/cognitiveservices/v1?language={}&format={}",
                mode,
                language,
                format
            )
        }
    }
}

impl MyHandler {
    fn parse_server_message(&self, msg: ws::Message) -> ws::Result<()> {
        match msg {
            ws::Message::Text(text) => self.parse_server_message_text(&text)?,
            _ => warn!("Unimplemented"),
        };

        Ok(())
    }

    fn parse_server_message_text(&self, text: &str) -> ws::Result<()> {
        info!("Received From Server: {}", text);

        let sections: Vec<&str> = text.split("\r\n\r\n").collect();
        let header = sections[0];
        let body = sections[1];

        let header_lines: Vec<&str> = header.split("\r\n").collect();
        for line in header_lines {
            let kv: Vec<&str> = line.split(':').collect();
            let key = kv[0].trim();
            let value = kv[1].trim();
            if key == "Path" {
                let h = self.handler.clone();
                let mut h = h.lock().unwrap();
                match value {
                    "turn.start" => {
                        h.on_turn_start();
                    }
                    "turn.end" => {
                        *self.audio_uuid.lock().unwrap() = None;
                        h.on_turn_end();
                    }
                    "speech.startDetected" => {
                        h.on_speech_start();
                    }
                    "speech.endDetected" => {
                        h.on_speech_end();
                    }
                    "speech.hypothesis" => {
                        let json = serde_json::from_slice(body.as_bytes()).unwrap();
                        h.on_speech_hypothesis(json);
                    }
                    "speech.phrase" => {
                        let value: serde_json::Value = serde_json::from_str(body).unwrap();
                        let phrase = Phrase::from_json_value(&value).unwrap();
                        h.on_speech_phrase(phrase);
                    }
                    _ => {}
                };
            }
        }

        Ok(())
    }
}

impl ws::Handler for MyHandler {
    fn build_request(&mut self, url: &Url) -> ws::Result<ws::Request> {
        info!("Building request");
        let mut request = ws::Request::from_url(url)?;
        {
            let headers = request.headers_mut();
            let token = format!("Bearer {}", self.token.lock().unwrap())
                .as_bytes()
                .to_vec();
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

    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        info!("Connected");
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        info!("Got message");
        self.parse_server_message(msg)?;
        Ok(())
    }

    fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
        info!("Disconnected");
    }

    fn on_error(&mut self, err: ws::Error) {
        error!("{}", err);
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
