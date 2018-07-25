use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use url::Url;
use uuid::Uuid;
use ws;

use chrono::prelude::*;
use serde_json;

use speech::*;

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
    ws: Option<ws::WebSocket<MyFactory>>,
    ws_thread: Option<JoinHandle<()>>,
    sender: Arc<Mutex<Option<ws::Sender>>>,
    server_event_tx: Arc<Mutex<Option<Sender<ServerEvent>>>>,
    server_event_from_handler_tx: Option<Sender<ServerEvent>>,
    server_event_from_handler_rx: Option<Receiver<ServerEvent>>,
    server_event_from_handler_thread: Option<JoinHandle<()>>,
    server_event_from_handler_running: Arc<AtomicBool>,
    token: Arc<Mutex<String>>,
    audio_uuid: Arc<Mutex<Option<String>>>,
}

impl Websocket {
    pub fn new(token: Arc<Mutex<String>>) -> Websocket {
        Websocket {
            ws: None,
            ws_thread: None,
            sender: Arc::new(Mutex::new(None)),
            server_event_tx: Arc::new(Mutex::new(None)),
            server_event_from_handler_tx: None,
            server_event_from_handler_rx: None,
            server_event_from_handler_thread: None,
            server_event_from_handler_running: Arc::new(AtomicBool::new(true)),
            token,
            audio_uuid: Arc::new(Mutex::new(None)),
        }
    }

    /// Open the Websocket connection
    pub fn open(
        &mut self,
        mode: &Mode,
        format: &Format,
        is_custom_speech: bool,
        endpoint_id: &str,
    ) {
        let (tx, rx) = channel();
        self.server_event_from_handler_tx = Some(tx);
        self.server_event_from_handler_rx = Some(rx);

        if let Ok(sender_option) = self.sender.lock() {
            if let Some(_) = *sender_option {
                return;
            }
        }

        let ws = ws::WebSocket::new(MyFactory {
            sender: self.sender.clone(),
            token: self.token.clone(),
            server_event_tx: self.server_event_tx.clone(),
            server_event_from_handler_tx: self.server_event_from_handler_tx.clone(),
        }).unwrap();

        self.ws = Some(ws);

        // Url for the client
        let url = Self::build_url(mode, format, is_custom_speech, endpoint_id);

        // Queue a WebSocket connection to the url
        if let Some(ref mut ws) = self.ws {
            ws.connect(url).unwrap();
        }

        self.run();
    }

    /// Close the Websocket connection
    pub fn close(&mut self) {
        let mut ok = false;

        // Shutdown Websocket if exists
        if let Ok(sender_guard) = self.sender.lock() {
            if let Some(ref sender) = *sender_guard {
                match sender.shutdown() {
                    Ok(_) => {
                        ok = true;
                        info!("Shutting down");
                    }
                    Err(err) => error!("{}", err),
                }
            }
        }

        // Wait for Websocket thread to end
        if let Some(t) = self.ws_thread.take() {
            t.join().unwrap();
        }

        // Set Websocket sender to None
        if ok {
            if let Ok(mut sender_guard) = self.sender.lock() {
                *sender_guard = None;
            }

            self.ws.take();
        }
    }

    /// Construct a channel for receiving server events
    pub fn server_event_receiver(&mut self) -> Receiver<ServerEvent> {
        let (tx, rx) = channel();

        *self.server_event_tx.lock().unwrap() = Some(tx);

        rx
    }

    fn run(&mut self) {
        if self.ws.is_some() {
            let running_1 = self.server_event_from_handler_running.clone();
            let audio_uuid_1 = self.audio_uuid.clone();
            if let Some(rx) = self.server_event_from_handler_rx.take() {
                self.server_event_from_handler_thread = Some(thread::spawn(move || {
                    while running_1.load(Ordering::Relaxed) {
                        match rx.recv() {
                            Ok(ServerEvent::TurnEnd) => {
                                *audio_uuid_1.lock().unwrap() = None;
                            }
                            _ => {}
                        }
                    }
                }));
            }

            let ws = self.ws.take().unwrap();
            self.ws_thread = Some(thread::spawn(move || match ws.run() {
                Err(err) => error!("{}", err),
                _ => {}
            }));
        }
    }

    fn build_url(mode: &Mode, format: &Format, is_custom_speech: bool, endpoint_id: &str) -> Url {
        let language = match mode {
            Mode::Interactive(language) | Mode::Dictation(language) => language.to_string(),
            Mode::Conversation(language) => language.to_string(),
        };
        let uri = if is_custom_speech {
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
        };

        uri.parse().unwrap()
    }

    /// Send speech configuration data to Bing Speech API via Websocket
    pub fn config(&self, cfg: &ConfigPayload) -> ws::Result<()> {
        if let Ok(sender_guard) = self.sender.lock() {
            if let Some(ref sender) = *sender_guard {
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
                return sender.send(msg);
            }
        }

        Ok(())
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
}

struct MyHandler {
    token: Arc<Mutex<String>>,
    server_event_tx: Arc<Mutex<Option<Sender<ServerEvent>>>>,
    server_event_from_handler_tx: Option<Sender<ServerEvent>>,
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
                let event = match value {
                    "turn.start" => ServerEvent::TurnStart,
                    "turn.end" => ServerEvent::TurnEnd,
                    "speech.startDetected" => ServerEvent::SpeechStartDetected,
                    "speech.hypothesis" => {
                        let json = serde_json::from_slice(body.as_bytes()).unwrap();
                        ServerEvent::SpeechHypothesis(json)
                    }
                    "speech.phrase" => {
                        let value: serde_json::Value = serde_json::from_str(body).unwrap();
                        ServerEvent::SpeechPhrase(Phrase::from_json_value(&value).unwrap())
                    }
                    "speech.endDetected" => ServerEvent::SpeechEndDetected,
                    _ => ServerEvent::Unknown,
                };

                if let Some(ref tx) = self.server_event_from_handler_tx {
                    tx.send(event.clone()).unwrap();
                }

                if let Ok(guard) = self.server_event_tx.lock() {
                    if let Some(ref server_event_tx) = *guard {
                        server_event_tx.send(event).unwrap();
                    }
                }
            }
        }

        Ok(())
    }
}

impl ws::Handler for MyHandler {
    fn build_request(&mut self, url: &Url) -> ws::Result<ws::Request> {
        let mut request = ws::Request::from_url(url)?;
        {
            let headers = request.headers_mut();
            let token = format!("Bearer {}", &self.token.lock().unwrap())
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
        self.parse_server_message(msg)?;
        Ok(())
    }

    fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
        if let Ok(guard) = self.server_event_tx.lock() {
            if let Some(ref server_event_tx) = *guard {
                if let Err(err) = server_event_tx.send(ServerEvent::Disconnect) {
                    error!(target: "on_close()", "{}", err);
                }
            }
        }

        info!("Disconnected");
    }

    fn on_error(&mut self, err: ws::Error) {
        error!("{}", err);
    }
}

struct MyFactory {
    sender: Arc<Mutex<Option<ws::Sender>>>,
    token: Arc<Mutex<String>>,
    server_event_tx: Arc<Mutex<Option<Sender<ServerEvent>>>>,
    server_event_from_handler_tx: Option<Sender<ServerEvent>>,
}

impl ws::Factory for MyFactory {
    type Handler = MyHandler;

    fn connection_made(&mut self, sender: ws::Sender) -> MyHandler {
        if let Ok(mut sender_guard) = self.sender.lock() {
            *sender_guard = Some(sender);
        }

        MyHandler {
            token: self.token.clone(),
            server_event_tx: self.server_event_tx.clone(),
            server_event_from_handler_tx: self.server_event_from_handler_tx.clone(),
        }
    }

    fn client_connected(&mut self, sender: ws::Sender) -> MyHandler {
        if let Ok(mut sender_guard) = self.sender.lock() {
            *sender_guard = Some(sender);
        }

        MyHandler {
            token: self.token.clone(),
            server_event_tx: self.server_event_tx.clone(),
            server_event_from_handler_tx: self.server_event_from_handler_tx.clone(),
        }
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
