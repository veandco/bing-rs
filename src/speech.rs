// Tokio/Futures Imports
use futures::future::ok;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

// Hyper Imports
use hyper::{ self, Headers, Uri, Method };
use hyper::client::{Client, Request};
use hyper::header::{Authorization, Bearer};
use hyper::StatusCode;
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

use std::rc::Rc;
use std::cell::RefCell;

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
pub struct RecognitionResult {
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
pub struct RecognitionResponse {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Duration")]
    pub duration: f64,
    #[serde(rename = "NBest")]
    pub nbest: Vec<RecognitionResult>,
}

impl Speech {
    pub fn new<T>(subscription_key: T) -> Result<Self> where T: ToString {
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
            recognize_uri: String::from("https://speech.platform.bing.com/speech/recognition/interactive/cognitiveservices/v1?language=en-us&format=detailed")
        })
    }

    pub fn set_subscription_key(&mut self, key: &str) {
        self.subscription_key = String::from(key);
    }

    pub fn set_token_uri(&mut self, uri: &str) {
        self.token_uri = String::from(uri);
    }

    pub fn set_recognize_uri(&mut self, uri: &str) {
        self.recognize_uri = String::from(uri);
    }

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

        let work = client
            .request(request)
            .and_then(|res| {
                let header = res.headers().clone();
                let status = res.status();
                res.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    ok::<_, hyper::Error>(v)
                }).map(move |chunks| {
                    if chunks.is_empty() {
                        Ok((header, status, None))
                    } else {
                        let token = String::from_utf8(chunks)?;
                        Ok((
                            header,
                            status,
                            Some(token)
                        ))
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

    pub fn recognize(self, audio: Vec<u8>) -> Result<(Headers, StatusCode, Option<RecognitionResponse>)> {
        let uri: Uri = self.recognize_uri.parse()?;
        let mut request = Request::new(Method::Post, uri);
        let mut core_ref = self.core.try_borrow_mut()?;
        let client = self.client;

        request.set_body(audio);
        {
            let headers_ref = request.headers_mut();
            headers_ref.set(Authorization(Bearer{ token: self.token }));
            headers_ref.set_raw("Content-Type", "audio/wav; codec=audio/pcm; samplerate=16000");
        }

        let work = client
            .request(request)
            .and_then(|res| {
                let header = res.headers().clone();
                let status = res.status();
                res.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    ok::<_, hyper::Error>(v)
                }).map(move |chunks| {
                    if chunks.is_empty() {
                        Ok((header, status, None))
                    } else {
                        Ok((
                            header,
                            status,
                            Some(serde_json::from_slice(&chunks)?)
                        ))
                    }
                })
            });
        core_ref.run(work)?
    }
}