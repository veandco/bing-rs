// tokio / futures
use futures::{Future, Stream};
use tokio_core::reactor::Core;

// hyper
use hyper::client::{Client, HttpConnector};
use hyper::StatusCode;
use hyper::{Body, HeaderMap, Method, Request, Uri};
#[cfg(feature = "rustls")]
use hyper_rustls::HttpsConnector;
#[cfg(feature = "rust-native-tls")]
use hyper_tls;
#[cfg(feature = "rust-native-tls")]
type HttpsConnector = hyper_tls::HttpsConnector<hyper::client::HttpConnector>;

// serde_json
use serde_json;

// internal
pub mod c;
pub mod voice;
pub mod websocket;
use self::websocket::*;
use errors::*;

// std
use std::cell::RefCell;
use std::fmt::{self, Display};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[no_mangle]
pub struct Speech {
    pub token: Arc<Mutex<String>>,
    core: Rc<RefCell<Core>>,
    client: Rc<Client<HttpsConnector<HttpConnector>>>,
    subscription_key: String,
    is_custom_speech: bool,
    endpoint_id: String,
}

impl Speech {
    /// Creates a new Bing Speech handle
    ///
    /// # Examples
    ///
    /// ```
    /// use bing_rs::speech::*;
    ///
    /// let speech = Speech::new("your_subscription_key").unwrap();
    /// ```
    pub fn new<T>(subscription_key: &T) -> Result<Self>
    where
        T: ToString,
    {
        let core = Core::new()?;
        let client = Client::builder().build(HttpsConnector::new(4));
        Ok(Speech {
            token: Arc::new(Mutex::new(String::new())),
            core: Rc::new(RefCell::new(core)),
            client: Rc::new(client),
            subscription_key: subscription_key.to_string(),
            is_custom_speech: false,
            endpoint_id: String::new(),
        })
    }

    /// Enable / Disable Bing Custom Speech
    pub fn set_custom_speech(&mut self, is_custom_speech: bool) {
        self.is_custom_speech = is_custom_speech;
    }

    /// Sets Bing Speech subscription key
    pub fn set_subscription_key(&mut self, key: &str) {
        self.subscription_key = String::from(key);
    }

    /// Sets Bing Custom Speech Endpoint ID
    pub fn set_endpoint_id(&mut self, endpoint_id: &str) {
        self.endpoint_id = String::from(endpoint_id);
    }

    /// Fetch new Bing Speech token
    ///
    /// # Examples
    ///
    /// ```
    /// use bing_rs::speech::*;
    ///
    /// let speech = Speech::new("your_subscription_key").unwrap();
    ///
    /// speech.fetch_token().unwrap();
    /// ```
    pub fn fetch_token(&mut self) -> Result<(HeaderMap, StatusCode, Option<String>)> {
        let uri: Uri = if self.is_custom_speech {
            "https://westus.api.cognitive.microsoft.com/sts/v1.0/issueToken"
        } else {
            "https://api.cognitive.microsoft.com/sts/v1.0/issueToken"
        }.parse()
        .unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("Ocp-Apim-Subscription-Key", self.subscription_key.as_str())
            .header("Content-Length", "0")
            .body(Body::empty())
            .unwrap();
        let mut core_ref = self.core.try_borrow_mut()?;
        let client = &self.client;

        let work = client.request(request).and_then(|res| {
            let header = res.headers().clone();
            let status = res.status();
            res.into_body().concat2().map(move |chunks| {
                if chunks.is_empty() {
                    Ok((header, status, None))
                } else {
                    let token = String::from_utf8(chunks.to_vec())?;
                    Ok((header, status, Some(token)))
                }
            })
        });

        let result = core_ref.run(work)?;
        if let Ok(ref tuple) = result {
            if let Some(ref token) = tuple.2 {
                *self.token.lock().unwrap() = token.clone();
            }
        }

        result
    }

    pub fn auto_fetch_token(&mut self) {
        let token_1 = self.token.clone();
        let subscription_key = self.subscription_key.clone();
        let is_custom_speech = self.is_custom_speech;

        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(9 * 60));

            let token_2 = token_1.clone();
            let uri: Uri = if is_custom_speech {
                "https://westus.api.cognitive.microsoft.com/sts/v1.0/issueToken"
            } else {
                "https://api.cognitive.microsoft.com/sts/v1.0/issueToken"
            }.parse()
            .unwrap();

            let request = Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Ocp-Apim-Subscription-Key", subscription_key.as_str())
                .header("Content-Length", "0")
                .body(Body::empty())
                .unwrap();

            let mut core = Core::new().unwrap();
            let client = Client::builder().build(HttpsConnector::new(1));
            let work = client.request(request).and_then(|res| {
                res.into_body().concat2().map(move |chunks| {
                    if !chunks.is_empty() {
                        let token = String::from_utf8(chunks.to_vec()).unwrap();
                        if let Ok(mut t) = token_2.lock() {
                            *t = token;
                        }
                    }
                })
            });
            core.run(work).unwrap();
        });
    }

    /// Recognize text from provided audio data
    ///
    /// See `examples/simple.rs` or `examples/simple_custom.rs` for examples.
    pub fn recognize(
        &self,
        audio: Vec<u8>,
        mode: &Mode,
        format: &Format,
    ) -> Result<(HeaderMap, StatusCode, Option<Phrase>)> {
        let language = match &mode {
            Mode::Interactive(language) | Mode::Dictation(language) => format!("{}", language),
            Mode::Conversation(language) => format!("{}", language),
        };
        let uri: Uri = if self.is_custom_speech {
            format!(
                "https://westus.stt.speech.microsoft.com/speech/recognition/{}/cognitiveservices/v1?cid={}&language={}&format={}",
                mode,
                &self.endpoint_id,
                language,
                format
            )
        } else {
            format!(
                "https://speech.platform.bing.com/speech/recognition/{}/cognitiveservices/v1?language={}&format={}",
                mode,
                language,
                format
            )
        }.parse()
            .unwrap();

        let mut core_ref = self.core.try_borrow_mut()?;
        let client = &self.client;

        // Build Request
        let audio = if self.is_custom_speech {
            let mut final_audio = Vec::new();
            let wav_header: Vec<u8> = vec![
                0x52, 0x49, 0x46, 0x46, 0xc4, 0x09, 0x01, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
                0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x80, 0x3e, 0x00, 0x00,
                0x00, 0x7d, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61, 0xa0, 0x09,
                0x01, 0x00,
            ];
            final_audio.extend_from_slice(&wav_header);
            final_audio.extend_from_slice(&audio);
            final_audio
        } else {
            audio
        };
        let content_type = if self.is_custom_speech {
            "application/octet-stream"
        } else {
            "audio/wav; codec=audio/pcm; samplerate=16000"
        };
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.lock().unwrap().clone()).as_str(),
            ).header("Content-Type", content_type)
            .body(Body::from(audio))
            .unwrap();

        // Send Request
        let work = client.request(request).and_then(|res| {
            let header = res.headers().clone();
            let status = res.status();
            res.into_body().concat2().map(move |chunks| {
                if chunks.is_empty() {
                    Ok((header, status, None))
                } else {
                    let value: serde_json::Value = serde_json::from_slice(&chunks.to_vec())?;
                    let phrase = Phrase::from_json_value(&value)?;
                    Ok((header, status, Some(phrase)))
                }
            })
        });
        core_ref.run(work)?
    }

    /// Synthesize voice from a text
    ///
    /// See `examples/synthesize.rs` for an example.
    pub fn synthesize(
        &self,
        text: &str,
        font: &voice::Font,
    ) -> Result<(HeaderMap, StatusCode, Option<Vec<u8>>)> {
        let uri: Uri = "https://speech.platform.bing.com/synthesize"
            .parse()
            .unwrap();
        let mut core_ref = self.core.try_borrow_mut()?;
        let client = &self.client;

        // Build Request
        let data = format!("<speak version='1.0' xml:lang='en-US'><voice xml:lang='{}' xml:gender='{}' name='{}'>{}</voice></speak>", font.lang, font.gender, font.name, text);
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.lock().unwrap().clone()).as_str(),
            ).header("Content-Type", "application/ssml+xml")
            .header("X-Microsoft-OutputFormat", "raw-16khz-16bit-mono-pcm")
            .header("User-Agent", "bing-rs")
            .body(Body::from(data))
            .unwrap();

        // Send Request
        let work = client.request(request).and_then(|res| {
            let header = res.headers().clone();
            let status = res.status();
            res.into_body().concat2().map(move |chunks| {
                if chunks.is_empty() {
                    Ok((header, status, None))
                } else {
                    Ok((header, status, Some(chunks.to_vec())))
                }
            })
        });
        core_ref.run(work)?
    }
}

/// Struct for storing DetailedPhrase's recognized text information
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub struct DetailedPhraseItem {
    #[serde(rename = "Confidence")]
    pub confidence: f64,
    #[serde(rename = "Lexical")]
    pub lexical: String,
    #[serde(rename = "ITN")]
    pub itn: String,
    #[serde(rename = "MaskedITN")]
    pub masked_itn: String,
    #[serde(rename = "Display")]
    pub display: String,
}

/// Recognition result when "detailed" format is used for speech recognition
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub struct DetailedPhrase {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
    #[serde(rename = "NBest")]
    pub nbest: Vec<DetailedPhraseItem>,
}

/// Recognition result when "simple" format is used for speech recognition
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub struct SimplePhrase {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "DisplayText")]
    pub display_text: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
}

/// Silence recognition result when there's nothing detected
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub struct SilencePhrase {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
}

/// Partial speech recognition result when still in the middle of speech
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub struct Hypothesis {
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
}

/// Enum for matching simple, detailed, and silence recognition result
#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug, Clone)]
pub enum Phrase {
    Simple(SimplePhrase),
    Detailed(DetailedPhrase),
    Silence(SilencePhrase),
    Unknown,
}

impl Phrase {
    pub fn from_json_value(value: &serde_json::Value) -> Result<Self> {
        if let Some(object) = value.as_object() {
            let recognition_status = object["RecognitionStatus"].as_str().unwrap();
            if recognition_status == "Success" {
                if object.contains_key("DisplayText") {
                    return Ok(Phrase::Simple(serde_json::from_value(value.clone())?));
                } else {
                    return Ok(Phrase::Detailed(serde_json::from_value(value.clone())?));
                }
            } else if recognition_status == "InitialSilenceTimeout" {
                return Ok(Phrase::Silence(serde_json::from_value(value.clone())?));
            }
        }

        Ok(Phrase::Unknown)
    }
}

/// Supported interactive and dictation languages by Bing
pub enum InteractiveDictationLanguage {
    ArabicEgypt,
    CatalanSpain,
    DanishDenmark,
    GermanGermany,
    EnglishAustralia,
    EnglishCanada,
    EnglishUnitedKingdom,
    EnglishIndia,
    EnglishNewZealand,
    EnglishUnitedStates,
    SpanishSpain,
    SpanishMexico,
    FinnishFinland,
    FrenchCanada,
    FrenchFrance,
    HindiIndia,
    ItalianItaly,
    JapaneseJapan,
    KoreanKorea,
    NorwegianNorway,
    DutchNetherlands,
    PolishPoland,
    PortugueseBrazil,
    PortuguesePortugal,
    RussianRussia,
    SwedishSweden,
    ChineseChina,
    ChineseHongKong,
    ChineseTaiwan,
}

/// Supported conversation languages by Bing
pub enum ConversationLanguage {
    ArabicEgypt,
    GermanGermany,
    EnglishUnitedStates,
    SpanishSpain,
    FrenchFrance,
    ItalianItaly,
    JapaneseJapan,
    PortugueseBrazil,
    RussianRussia,
    ChineseChina,
}

/// Enum for matching mode and language
pub enum Mode {
    Interactive(InteractiveDictationLanguage),
    Conversation(ConversationLanguage),
    Dictation(InteractiveDictationLanguage),
}

/// Enum for the different format of speech recognition result
#[derive(Clone, PartialEq)]
pub enum Format {
    Simple,
    Detailed,
}

impl Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Interactive(_) => write!(f, "interactive"),
            Mode::Conversation(_) => write!(f, "conversation"),
            Mode::Dictation(_) => write!(f, "dictation"),
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Format::Detailed => write!(f, "detailed"),
            Format::Simple => write!(f, "simple"),
        }
    }
}

impl Display for InteractiveDictationLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            InteractiveDictationLanguage::ArabicEgypt => "ar-EG",
            InteractiveDictationLanguage::CatalanSpain => "ca-ES",
            InteractiveDictationLanguage::DanishDenmark => "da-DK",
            InteractiveDictationLanguage::GermanGermany => "de-DE",
            InteractiveDictationLanguage::EnglishAustralia => "en-AU",
            InteractiveDictationLanguage::EnglishCanada => "en-CA",
            InteractiveDictationLanguage::EnglishUnitedKingdom => "en-GB",
            InteractiveDictationLanguage::EnglishIndia => "en-IN",
            InteractiveDictationLanguage::EnglishNewZealand => "en-NZ",
            InteractiveDictationLanguage::EnglishUnitedStates => "en-US",
            InteractiveDictationLanguage::SpanishSpain => "es-ES",
            InteractiveDictationLanguage::SpanishMexico => "es-MX",
            InteractiveDictationLanguage::FinnishFinland => "fi-FI",
            InteractiveDictationLanguage::FrenchCanada => "fr-CA",
            InteractiveDictationLanguage::FrenchFrance => "fr-FR",
            InteractiveDictationLanguage::HindiIndia => "hi-IN",
            InteractiveDictationLanguage::ItalianItaly => "it-IT",
            InteractiveDictationLanguage::JapaneseJapan => "ja-JP",
            InteractiveDictationLanguage::KoreanKorea => "ko-KR",
            InteractiveDictationLanguage::NorwegianNorway => "nb-NO",
            InteractiveDictationLanguage::DutchNetherlands => "nl-NL",
            InteractiveDictationLanguage::PolishPoland => "pl-PL",
            InteractiveDictationLanguage::PortugueseBrazil => "pt-BR",
            InteractiveDictationLanguage::PortuguesePortugal => "pt-PT",
            InteractiveDictationLanguage::RussianRussia => "ru-RU",
            InteractiveDictationLanguage::SwedishSweden => "sv-SE",
            InteractiveDictationLanguage::ChineseChina => "zh-CN",
            InteractiveDictationLanguage::ChineseHongKong => "zh-HK",
            InteractiveDictationLanguage::ChineseTaiwan => "zh-TW",
        };
        write!(f, "{}", s)
    }
}

impl Display for ConversationLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ConversationLanguage::ArabicEgypt => "ar-EG",
            ConversationLanguage::GermanGermany => "de-DE",
            ConversationLanguage::EnglishUnitedStates => "en-US",
            ConversationLanguage::SpanishSpain => "es-ES",
            ConversationLanguage::FrenchFrance => "fr-FR",
            ConversationLanguage::ItalianItaly => "it-IT",
            ConversationLanguage::JapaneseJapan => "ja-JP",
            ConversationLanguage::PortugueseBrazil => "pt-BR",
            ConversationLanguage::RussianRussia => "ru-RU",
            ConversationLanguage::ChineseChina => "zh-CN",
        };
        write!(f, "{}", s)
    }
}

impl Display for Hypothesis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Text: {}", self.text)?;
        writeln!(f, "Offset: {}", self.offset)?;
        writeln!(f, "Duration: {}", self.duration)?;
        Ok(())
    }
}

impl Display for Phrase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Phrase::Detailed(detailed) => {
                writeln!(f, "RecognitionStatus: {}", detailed.recognition_status)?;
                writeln!(f, "Offset: {}", detailed.offset)?;
                writeln!(f, "Duration: {}", detailed.duration)?;
                writeln!(f, "NBest")?;
                writeln!(f, "========")?;

                for (i, item) in detailed.nbest.iter().enumerate() {
                    writeln!(f, "#{}", i)?;
                    writeln!(f, "--------")?;
                    writeln!(f, "    Confidence: {}", item.confidence)?;
                    writeln!(f, "    Lexical: {}", item.lexical)?;
                    writeln!(f, "    ITN: {}", item.itn)?;
                    writeln!(f, "    MaskedITN: {}", item.masked_itn)?;
                    writeln!(f, "    Display: {}", item.display)?;
                }
            }
            Phrase::Simple(simple) => {
                writeln!(f, "RecognitionStatus: {}", simple.recognition_status)?;
                writeln!(f, "DisplayText: {}", simple.display_text)?;
                writeln!(f, "Offset: {}", simple.offset)?;
                writeln!(f, "Duration: {}", simple.duration)?;
            }
            Phrase::Silence(silence) => {
                writeln!(f, "RecognitionStatus: {}", silence.recognition_status)?;
                writeln!(f, "Offset: {}", silence.offset)?;
                writeln!(f, "Duration: {}", silence.duration)?;
            }
            Phrase::Unknown => {
                writeln!(f, "RecognitionStatus: Unknown")?;
            }
        };

        Ok(())
    }
}

/// Auto-detected speech configuration payload
pub fn default_speech_config() -> ConfigPayload {
    #[cfg(target_os = "windows")]
    let platform = "Windows";
    #[cfg(target_os = "osx")]
    let platform = "macOS";
    #[cfg(target_os = "linux")]
    let platform = "Linux";
    #[cfg(target_os = "freebsd")]
    let platform = "FreeBSD";
    #[cfg(target_os = "dragonfly")]
    let platform = "DragonflyBSD";
    #[cfg(target_os = "bitrig")]
    let platform = "Bitrig";
    #[cfg(target_os = "openbsd")]
    let platform = "OpenBSD";
    #[cfg(target_os = "netbsd")]
    let platform = "NetBSD";
    #[cfg(target_os = "ios")]
    let platform = "iOS";
    #[cfg(target_os = "android")]
    let platform = "Android";

    ConfigPayload {
        context: ConfigPayloadContext {
            system: ConfigPayloadContextSystem {
                version: "0.0.2".to_string(),
            },
            os: ConfigPayloadContextOs {
                platform: platform.to_string(),
                name: "Unknown".to_string(),
                version: "Unknown".to_string(),
            },
            device: ConfigPayloadContextDevice {
                manufacturer: "Unknown".to_string(),
                model: "Unknown".to_string(),
                version: "Unknown".to_string(),
            },
        },
    }
}
