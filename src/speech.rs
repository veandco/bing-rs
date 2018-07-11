// Tokio/Futures Imports
use futures::future::ok;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

// Hyper Imports
use hyper::client::{Client, Request};
use hyper::header::{Authorization, Bearer};
use hyper::StatusCode;
use hyper::{self, Headers, Method, Uri};
#[cfg(feature = "rustls")]
use hyper_rustls::HttpsConnector;
#[cfg(feature = "rust-native-tls")]
use hyper_tls;
#[cfg(feature = "rust-native-tls")]
type HttpsConnector = hyper_tls::HttpsConnector<hyper::client::HttpConnector>;

// Serde Imports
use serde_json;

// Internal Library Imports
use error::*;

use std::cell::RefCell;
use std::fmt::{self, Display};
use std::rc::Rc;

#[no_mangle]
#[repr(C)]
pub struct Speech {
    core: Rc<RefCell<Core>>,
    client: Rc<Client<HttpsConnector>>,
    subscription_key: String,
    token: String,
    token_uri: String,
    recognize_uri: String,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug)]
pub struct DetailedRecognitionResult {
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

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug)]
pub struct DetailedRecognitionResponse {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
    #[serde(rename = "NBest")]
    pub nbest: Vec<DetailedRecognitionResult>,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug)]
pub struct SimpleRecognitionResponse {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "DisplayText")]
    pub display_text: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
}

#[no_mangle]
#[repr(C)]
#[derive(Deserialize, Debug)]
pub enum Response {
    Simple(SimpleRecognitionResponse),
    Detailed(DetailedRecognitionResponse),
}

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

pub enum Mode {
    Interactive(InteractiveDictationLanguage),
    Conversation(ConversationLanguage),
    Dictation(InteractiveDictationLanguage),
}

pub enum Format {
    Simple,
    Detailed,
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
    pub fn new<T>(subscription_key: T) -> Result<Self>
    where
        T: ToString,
    {
        let core = Core::new()?;
        let handle = core.handle();
        let client = Client::configure()
            .connector(HttpsConnector::new(4, &handle))
            .build(&handle);
        Ok(Speech {
            core: Rc::new(RefCell::new(core)),
            client: Rc::new(client),
            subscription_key: subscription_key.to_string(),
            token: String::new(),
            token_uri: String::from("https://api.cognitive.microsoft.com/sts/v1.0/issueToken"),
            recognize_uri: String::from("https://speech.platform.bing.com/speech/recognition"),
        })
    }

    /// Sets Bing Speech subscription key
    pub fn set_subscription_key(&mut self, key: &str) {
        self.subscription_key = String::from(key);
    }

    /// Sets Bing Speech token URI
    pub fn set_token_uri(&mut self, uri: &str) {
        self.token_uri = String::from(uri);
    }

    /// Sets Bing Speech recognition URI
    pub fn set_recognize_uri(&mut self, uri: &str) {
        self.recognize_uri = String::from(uri);
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
    pub fn fetch_token(&mut self) -> Result<(Headers, StatusCode, Option<String>)> {
        let uri: Uri = self.token_uri.parse()?;
        let mut request = Request::new(Method::Post, uri);
        let mut core_ref = self.core.try_borrow_mut()?;
        let client = &self.client;

        request.set_body("");
        {
            let headers_ref = request.headers_mut();
            headers_ref.set_raw("Ocp-Apim-Subscription-Key", self.subscription_key.clone());
        }

        let work = client.request(request).and_then(|res| {
            let header = res.headers().clone();
            let status = res.status();
            res.body()
                .fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    ok::<_, hyper::Error>(v)
                })
                .map(move |chunks| {
                    if chunks.is_empty() {
                        Ok((header, status, None))
                    } else {
                        let token = String::from_utf8(chunks)?;
                        Ok((header, status, Some(token)))
                    }
                })
        });

        let result = core_ref.run(work)?;
        if let Ok(ref tuple) = result {
            if let Some(ref token) = tuple.2 {
                self.token = token.clone();
            }
        }

        result
    }

    /// Recognize text from provided audio data
    ///
    /// # Examples
    ///
    /// ```
    /// use bing_rs::speech::*;
    ///
    /// let speech = Speech::new("your_subscription_key").unwrap();
    ///
    /// speech.fetch_token().unwrap();
    ///
    /// let mut file = File::open("assets/audio.raw").unwrap();
    /// let mut audio = Vec::new();
    ///
    /// match client.recognize(
    ///     audio,
    ///     Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates),
    ///     Format::Detailed,
    /// ) {
    ///     Ok((_, _, Some(ref response))) => match response {
    ///         Response::Detailed(response) => {
    ///             println!("RecognitionStatus: {}", response.recognition_status);
    ///             println!("Offset: {}", response.offset);
    ///             println!("Duration: {}", response.duration);
    ///             println!("NBest");
    ///             println!("========");
    ///
    ///             for (i, ref result) in response.nbest.iter().enumerate() {
    ///                 println!("#{}", i);
    ///                 println!("--------");
    ///                 println!("    Confidence: {}", result.confidence);
    ///                 println!("    Lexical: {}", result.lexical);
    ///                 println!("    ITN: {}", result.itn);
    ///                 println!("    MaskedITN: {}", result.masked_itn);
    ///                 println!("    Display: {}", result.display);
    ///             }
    ///         }
    ///         _ => println!("Not handling simple response"),
    ///     },
    ///     Ok((_, _, None)) => println!("Ok but no result"),
    ///     Err(err) => println!("Error: {}", err),
    /// }
    /// ```
    pub fn recognize(
        self,
        audio: Vec<u8>,
        mode: Mode,
        format: Format,
    ) -> Result<(Headers, StatusCode, Option<Response>)> {
        let language = match &mode {
            Mode::Interactive(language) | Mode::Dictation(language) => format!("{}", language),
            Mode::Conversation(language) => format!("{}", language),
        };
        let uri = format!(
            "https://speech.platform.bing.com/speech/recognition/{}/cognitiveservices/v1?language={}&format={}",
            mode,
            language,
            format
        );
        let mut core_ref = self.core.try_borrow_mut()?;
        let client = self.client;

        // Build Request
        let uri: Uri = uri.parse()?;
        let mut request = Request::new(Method::Post, uri);
        request.set_body(audio);
        {
            let headers_ref = request.headers_mut();
            headers_ref.set(Authorization(Bearer { token: self.token }));
            headers_ref.set_raw(
                "Content-Type",
                "audio/wav; codec=audio/pcm; samplerate=16000",
            );
        }

        // Send Request
        let work = client.request(request).and_then(|res| {
            let header = res.headers().clone();
            let status = res.status();
            res.body()
                .fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    ok::<_, hyper::Error>(v)
                })
                .map(move |chunks| {
                    if chunks.is_empty() {
                        Ok((header, status, None))
                    } else {
                        let response = match format {
                            Format::Detailed => {
                                Response::Detailed(serde_json::from_slice(&chunks)?)
                            }
                            Format::Simple => Response::Simple(serde_json::from_slice(&chunks)?),
                        };
                        Ok((header, status, Some(response)))
                    }
                })
        });
        core_ref.run(work)?
    }
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
