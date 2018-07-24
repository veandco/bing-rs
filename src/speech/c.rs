use std::ffi::CString;
use std::marker::Send;
use std::mem;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::ptr;
use std::sync::{Arc, Mutex};

use speech::{
    ClientEvent, DetailedPhraseItem, Format, Handle, Handler, Hypothesis,
    InteractiveDictationLanguage, Mode, Phrase, Speech,
};

#[no_mangle]
#[repr(C)]
pub struct BingSpeech {
    handle: Speech,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechWebsocket {
    handle: Handle,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechWebsocketHandler {
    on_turn_start: *mut c_void,
    on_turn_end: *mut c_void,
    on_speech_start_detected: *mut c_void,
    on_speech_end_detected: *mut c_void,
    on_speech_hypothesis: *mut c_void,
    on_speech_phrase: *mut c_void,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechResult {
    pub confidence: c_double,
    pub lexical: *mut c_char,
    pub itn: *mut c_char,
    pub masked_itn: *mut c_char,
    pub display: *mut c_char,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechPhrase {
    recognition_status: *mut c_char,
    display_text: *mut c_char,
    offset: c_double,
    duration: c_double,
    nbest: *mut BingSpeechResult,
    nbest_count: c_int,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechHypothesis {
    text: *mut c_char,
    offset: c_double,
    duration: c_double,
}

unsafe impl Send for BingSpeechWebsocketHandler {}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechHandler {
    c_handler: Arc<Mutex<BingSpeechWebsocketHandler>>,
}

fn nbest_to_c(nbest: &[DetailedPhraseItem]) -> Vec<BingSpeechResult> {
    nbest
        .iter()
        .map(|result| BingSpeechResult {
            confidence: result.confidence,
            lexical: to_c_string(&result.lexical),
            itn: to_c_string(&result.itn),
            masked_itn: to_c_string(&result.masked_itn),
            display: to_c_string(&result.display),
        })
        .collect()
}

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

impl Handler for BingSpeechHandler {
    fn on_turn_start(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_turn_start as usize == 0 {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_turn_start) };
        f();
    }

    fn on_turn_end(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_turn_end.is_null() {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_turn_end) };
        f();
    }

    fn on_speech_start_detected(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_start_detected.is_null() {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_start_detected) };
        f();
    }

    fn on_speech_end_detected(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_end_detected.is_null() {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_end_detected) };
        f();
    }

    fn on_speech_hypothesis(&mut self, hypothesis: Hypothesis) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_hypothesis.is_null() {
            return;
        }

        let f: extern "C" fn(BingSpeechHypothesis) =
            unsafe { mem::transmute(handler.on_speech_hypothesis) };
        f(BingSpeechHypothesis {
            text: to_c_string(&hypothesis.text),
            offset: hypothesis.offset,
            duration: hypothesis.duration,
        });
    }

    fn on_speech_phrase(&mut self, phrase: Phrase) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_phrase.is_null() {
            return;
        }

        let f: extern "C" fn(BingSpeechPhrase) =
            unsafe { mem::transmute(handler.on_speech_phrase) };
        let phrase = match phrase {
            Phrase::Simple(simple) => BingSpeechPhrase {
                recognition_status: to_c_string(&simple.recognition_status),
                display_text: to_c_string(&simple.display_text),
                offset: simple.offset,
                duration: simple.duration,
                nbest: ptr::null_mut(),
                nbest_count: 0,
            },
            Phrase::Detailed(detailed) => {
                let nbest_count = detailed.nbest.len() as i32;
                let mut nbest = nbest_to_c(&detailed.nbest);
                let phrase = BingSpeechPhrase {
                    recognition_status: to_c_string(&detailed.recognition_status),
                    display_text: ptr::null_mut(),
                    offset: detailed.offset,
                    duration: detailed.duration,
                    nbest: nbest.as_mut_ptr(),
                    nbest_count,
                };
                mem::forget(nbest);
                phrase
            },
            Phrase::Silence(silence) => BingSpeechPhrase {
                    recognition_status: to_c_string(&silence.recognition_status),
                    display_text: ptr::null_mut(),
                    offset: silence.offset,
                    duration: silence.duration,
                    nbest: ptr::null_mut(),
                    nbest_count: 0,
            },
            Phrase::Unknown => BingSpeechPhrase {
                recognition_status: to_c_string("Unknown"),
                display_text: ptr::null_mut(),
                offset: 0.0,
                duration: 0.0,
                nbest: ptr::null_mut(),
                nbest_count: 0,
            }
        };
        f(phrase);
    }
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_new(subscription_key: *mut c_char) -> *mut BingSpeech {
    let subscription_key = CString::from_raw(subscription_key).into_string().unwrap();
    let bing_speech = Box::new(BingSpeech {
        handle: Speech::new(&subscription_key).unwrap(),
    });

    Box::into_raw(bing_speech)
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_free(bing_speech: *mut BingSpeech) {
    Box::from_raw(bing_speech);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_set_custom_speech(
    bing_speech: *mut BingSpeech,
    is_custom_speech: c_int,
) {
    (*bing_speech)
        .handle
        .set_custom_speech(is_custom_speech > 0);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_set_endpoint_id(
    bing_speech: *mut BingSpeech,
    endpoint_id: *mut c_char,
) {
    let endpoint_id = CString::from_raw(endpoint_id).into_string().unwrap();
    (*bing_speech).handle.set_endpoint_id(&endpoint_id);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_fetch_token(bing_speech: *mut BingSpeech) -> *mut c_char {
    let result = (*bing_speech).handle.fetch_token();
    if let Ok((_, _, Some(token))) = result {
        to_c_string(&token)
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket(
    bing_speech: *mut BingSpeech,
    handler: BingSpeechWebsocketHandler,
) -> *mut BingSpeechWebsocket {
    let mode = Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates);
    let handle = (*bing_speech)
        .handle
        .websocket(
            &mode,
            Format::Detailed,
            Arc::new(Mutex::new(BingSpeechHandler {
                c_handler: Arc::new(Mutex::new(handler)),
            })),
        )
        .unwrap();

    let handle = Box::new(BingSpeechWebsocket { handle });

    Box::into_raw(handle)
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_audio(
    handle: *mut BingSpeechWebsocket,
    audio: *mut u8,
    audio_size: usize,
) -> i32 {
    const BUFFER_SIZE: usize = 4096;

    let audio: Vec<u8> = Vec::from_raw_parts(audio, audio_size, audio_size);
    let mut i = 0;

    while i < audio_size {
        let j = if audio_size - i < BUFFER_SIZE {
            audio_size
        } else {
            i + BUFFER_SIZE
        };

        // Send audio data to Bing Speech
        let result = (*handle)
            .handle
            .send_tx
            .send(ClientEvent::Audio(audio[i..j].to_vec()));
        if let Err(err) = result {
            error!(target: "bing_speech_websocket_audio()", "{}", err);
            return 2;
        }

        i = j;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_close(handle: *mut BingSpeechWebsocket) {
    (*handle).handle.close();
}
