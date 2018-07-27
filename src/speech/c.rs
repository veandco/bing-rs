use std::ffi::CString;
use std::marker::Send;
use std::mem;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::ptr;
use std::sync::{Arc, Mutex};

use speech::websocket::*;
use speech::*;

#[no_mangle]
#[repr(C)]
pub struct BingSpeech {
    handle: Speech,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechWebsocket {
    handle: Websocket,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechWebsocketHandler {
    on_turn_start: *mut c_void,
    on_turn_end: *mut c_void,
    on_speech_start: *mut c_void,
    on_speech_end: *mut c_void,
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

    fn on_speech_start(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_start.is_null() {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_start) };
        f();
    }

    fn on_speech_end(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        if handler.on_speech_end.is_null() {
            return;
        }

        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_end) };
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
            }
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
            },
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

    mem::forget(subscription_key);
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
    mem::forget(endpoint_id);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_fetch_token(bing_speech: *mut BingSpeech) -> *mut c_char {
    let result = (*bing_speech).handle.fetch_token();
    match result {
        Ok((_, _, Some(token))) => to_c_string(&token),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_auto_fetch_token(bing_speech: *mut BingSpeech) {
    (*bing_speech).handle.auto_fetch_token();
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_synthesize(bing_speech: *mut BingSpeech, c_text: *mut c_char, c_font: c_int, c_output: *mut *mut c_void, c_output_len: *mut c_int) {
    let text = CString::from_raw(c_text).into_string().unwrap();
    if let Ok((_, _, Some(mut data))) = (*bing_speech).handle.synthesize(&text, font_from_c(c_font)) {
        *c_output_len = data.len() as i32;
        *c_output = data.as_mut_ptr() as *mut c_void;
        mem::forget(data);
    } else {
        *c_output_len = 0;
        *c_output = ptr::null_mut();
    }
    mem::forget(text);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_new() -> *mut BingSpeechWebsocket {
    let handle = Websocket::new();
    let websocket = Box::new(BingSpeechWebsocket { handle });
    Box::into_raw(websocket)
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_connect(
    c_speech: *mut BingSpeech,
    c_websocket: *mut BingSpeechWebsocket,
    c_mode: c_int,
    c_language: c_int,
    c_format: c_int,
    c_is_custom_speech: c_int,
    c_endpoint_id: *mut c_char,
    handler: BingSpeechWebsocketHandler,
) -> c_int {
    let (mode, ok) = mode_from_c(c_mode, c_language);
    if ok != 0 {
        return ok;
    }
    let format = format_from_c(c_format);
    let is_custom_speech = c_is_custom_speech > 0;
    let endpoint_id = if is_custom_speech {
        CString::from_raw(c_endpoint_id).into_string().unwrap()
    } else {
        "".to_string()
    };

    // Connect to Websocket
    let result = (*c_websocket).handle.connect(
        (*c_speech).handle.token.clone(),
        &mode,
        &format,
        is_custom_speech,
        &endpoint_id,
        Arc::new(Mutex::new(BingSpeechHandler {
            c_handler: Arc::new(Mutex::new(handler)),
        })),
    );
    mem::forget(endpoint_id);

    match result {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_disconnect(
    c_websocket: *mut BingSpeechWebsocket,
) -> c_int {
    let result = (*c_websocket).handle.disconnect();

    match result {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    }
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
        let result = (*handle).handle.audio(&audio[i..j].to_vec());
        if let Err(err) = result {
            error!("{}", err);
            return 2;
        }

        i = j;
    }

    mem::forget(audio);

    0
}

fn mode_from_c(c_mode: c_int, c_language: c_int) -> (Mode, c_int) {
    match c_mode {
        0 => match c_language {
            0 => (
                Mode::Interactive(InteractiveDictationLanguage::ArabicEgypt),
                0,
            ),
            1 => (
                Mode::Interactive(InteractiveDictationLanguage::CatalanSpain),
                0,
            ),
            2 => (
                Mode::Interactive(InteractiveDictationLanguage::ChineseChina),
                0,
            ),
            3 => (
                Mode::Interactive(InteractiveDictationLanguage::ChineseHongKong),
                0,
            ),
            4 => (
                Mode::Interactive(InteractiveDictationLanguage::ChineseTaiwan),
                0,
            ),
            5 => (
                Mode::Interactive(InteractiveDictationLanguage::DanishDenmark),
                0,
            ),
            6 => (
                Mode::Interactive(InteractiveDictationLanguage::DutchNetherlands),
                0,
            ),
            7 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishAustralia),
                0,
            ),
            8 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishCanada),
                0,
            ),
            9 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishIndia),
                0,
            ),
            10 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishNewZealand),
                0,
            ),
            11 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedKingdom),
                0,
            ),
            12 => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates),
                0,
            ),
            13 => (
                Mode::Interactive(InteractiveDictationLanguage::FinnishFinland),
                0,
            ),
            14 => (
                Mode::Interactive(InteractiveDictationLanguage::FrenchCanada),
                0,
            ),
            15 => (
                Mode::Interactive(InteractiveDictationLanguage::FrenchFrance),
                0,
            ),
            16 => (
                Mode::Interactive(InteractiveDictationLanguage::GermanGermany),
                0,
            ),
            17 => (
                Mode::Interactive(InteractiveDictationLanguage::HindiIndia),
                0,
            ),
            18 => (
                Mode::Interactive(InteractiveDictationLanguage::ItalianItaly),
                0,
            ),
            19 => (
                Mode::Interactive(InteractiveDictationLanguage::JapaneseJapan),
                0,
            ),
            20 => (
                Mode::Interactive(InteractiveDictationLanguage::KoreanKorea),
                0,
            ),
            21 => (
                Mode::Interactive(InteractiveDictationLanguage::NorwegianNorway),
                0,
            ),
            22 => (
                Mode::Interactive(InteractiveDictationLanguage::PolishPoland),
                0,
            ),
            23 => (
                Mode::Interactive(InteractiveDictationLanguage::PortugueseBrazil),
                0,
            ),
            24 => (
                Mode::Interactive(InteractiveDictationLanguage::PortuguesePortugal),
                0,
            ),
            25 => (
                Mode::Interactive(InteractiveDictationLanguage::RussianRussia),
                0,
            ),
            26 => (
                Mode::Interactive(InteractiveDictationLanguage::SpanishMexico),
                0,
            ),
            27 => (
                Mode::Interactive(InteractiveDictationLanguage::SpanishSpain),
                0,
            ),
            28 => (
                Mode::Interactive(InteractiveDictationLanguage::SwedishSweden),
                0,
            ),
            _ => (
                Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates),
                1,
            ),
        },
        1 => match c_language {
            0 => (
                Mode::Dictation(InteractiveDictationLanguage::ArabicEgypt),
                0,
            ),
            1 => (
                Mode::Dictation(InteractiveDictationLanguage::CatalanSpain),
                0,
            ),
            2 => (
                Mode::Dictation(InteractiveDictationLanguage::ChineseChina),
                0,
            ),
            3 => (
                Mode::Dictation(InteractiveDictationLanguage::ChineseHongKong),
                0,
            ),
            4 => (
                Mode::Dictation(InteractiveDictationLanguage::ChineseTaiwan),
                0,
            ),
            5 => (
                Mode::Dictation(InteractiveDictationLanguage::DanishDenmark),
                0,
            ),
            6 => (
                Mode::Dictation(InteractiveDictationLanguage::DutchNetherlands),
                0,
            ),
            7 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishAustralia),
                0,
            ),
            8 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishCanada),
                0,
            ),
            9 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishIndia),
                0,
            ),
            10 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishNewZealand),
                0,
            ),
            11 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishUnitedKingdom),
                0,
            ),
            12 => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishUnitedStates),
                0,
            ),
            13 => (
                Mode::Dictation(InteractiveDictationLanguage::FinnishFinland),
                0,
            ),
            14 => (
                Mode::Dictation(InteractiveDictationLanguage::FrenchCanada),
                0,
            ),
            15 => (
                Mode::Dictation(InteractiveDictationLanguage::FrenchFrance),
                0,
            ),
            16 => (
                Mode::Dictation(InteractiveDictationLanguage::GermanGermany),
                0,
            ),
            17 => (Mode::Dictation(InteractiveDictationLanguage::HindiIndia), 0),
            18 => (
                Mode::Dictation(InteractiveDictationLanguage::ItalianItaly),
                0,
            ),
            19 => (
                Mode::Dictation(InteractiveDictationLanguage::JapaneseJapan),
                0,
            ),
            20 => (
                Mode::Dictation(InteractiveDictationLanguage::KoreanKorea),
                0,
            ),
            21 => (
                Mode::Dictation(InteractiveDictationLanguage::NorwegianNorway),
                0,
            ),
            22 => (
                Mode::Dictation(InteractiveDictationLanguage::PolishPoland),
                0,
            ),
            23 => (
                Mode::Dictation(InteractiveDictationLanguage::PortugueseBrazil),
                0,
            ),
            24 => (
                Mode::Dictation(InteractiveDictationLanguage::PortuguesePortugal),
                0,
            ),
            25 => (
                Mode::Dictation(InteractiveDictationLanguage::RussianRussia),
                0,
            ),
            26 => (
                Mode::Dictation(InteractiveDictationLanguage::SpanishMexico),
                0,
            ),
            27 => (
                Mode::Dictation(InteractiveDictationLanguage::SpanishSpain),
                0,
            ),
            28 => (
                Mode::Dictation(InteractiveDictationLanguage::SwedishSweden),
                0,
            ),
            _ => (
                Mode::Dictation(InteractiveDictationLanguage::EnglishUnitedStates),
                1,
            ),
        },
        2 => match c_language {
            0 => (Mode::Conversation(ConversationLanguage::ArabicEgypt), 0),
            1 => (Mode::Conversation(ConversationLanguage::ChineseChina), 0),
            2 => (
                Mode::Conversation(ConversationLanguage::EnglishUnitedStates),
                0,
            ),
            3 => (Mode::Conversation(ConversationLanguage::FrenchFrance), 0),
            4 => (Mode::Conversation(ConversationLanguage::GermanGermany), 0),
            5 => (Mode::Conversation(ConversationLanguage::ItalianItaly), 0),
            6 => (Mode::Conversation(ConversationLanguage::JapaneseJapan), 0),
            7 => (
                Mode::Conversation(ConversationLanguage::PortugueseBrazil),
                0,
            ),
            8 => (Mode::Conversation(ConversationLanguage::RussianRussia), 0),
            9 => (Mode::Conversation(ConversationLanguage::SpanishSpain), 0),
            _ => (
                Mode::Conversation(ConversationLanguage::EnglishUnitedStates),
                1,
            ),
        },
        _ => (
            Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates),
            0,
        ),
    }
}

fn format_from_c(c_format: c_int) -> Format {
    match c_format {
        0 => Format::Simple,
        _ => Format::Detailed,
    }
}

fn font_from_c(c_font: c_int) -> &'static voice::Font {
    voice::en_us::JESSA_RUS
}