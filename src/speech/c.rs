use std::ffi::CString;
use std::marker::Send;
use std::mem;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::ptr;
use std::sync::{Arc, Mutex};

use speech::websocket::*;
use speech::*;

#[no_mangle]
pub struct BingSpeech {
    handle: Speech,
}

#[no_mangle]
pub struct BingSpeechWebsocket {
    handle: Websocket,
}

#[no_mangle]
#[repr(C)]
pub struct BingSpeechWebsocketHandler {
    on_turn_start: fn(),
    on_turn_end: fn(),
    on_speech_start: fn(),
    on_speech_end: fn(),
    on_speech_hypothesis: fn(BingSpeechHypothesis),
    on_speech_phrase: fn(BingSpeechPhrase),
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
        }).collect()
}

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

impl Handler for BingSpeechHandler {
    fn on_turn_start(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_turn_start) };
        f();
    }

    fn on_turn_end(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_turn_end) };
        f();
    }

    fn on_speech_start(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_start) };
        f();
    }

    fn on_speech_end(&mut self) {
        let handler = self.c_handler.lock().unwrap();
        let f: extern "C" fn() = unsafe { mem::transmute(handler.on_speech_end) };
        f();
    }

    fn on_speech_hypothesis(&mut self, hypothesis: Hypothesis) {
        let handler = self.c_handler.lock().unwrap();
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
pub unsafe extern "C" fn bing_speech_recognize(
    bing_speech: *mut BingSpeech,
    c_audio: *const c_void,
    c_audio_len: c_int,
    c_mode: c_int,
    c_language: c_int,
    c_format: c_int,
    c_phrase: *mut BingSpeechPhrase,
) -> c_int {
    let audio: Vec<u8> = Vec::from_raw_parts(
        c_audio as *mut u8,
        c_audio_len as usize,
        c_audio_len as usize,
    );
    let (mode, ok) = mode_from_c(c_mode, c_language);
    if ok != 0 {
        return ok;
    }

    let format = if c_format > 0 {
        Format::Detailed
    } else {
        Format::Simple
    };

    let audio_1 = audio.clone();
    mem::forget(audio);

    if let Ok((_, _, Some(phrase))) = (*bing_speech).handle.recognize(audio_1, &mode, &format) {
        match phrase {
            Phrase::Simple(simple) => {
                (*c_phrase).recognition_status = to_c_string(&simple.recognition_status);
                (*c_phrase).display_text = to_c_string(&simple.display_text);
                (*c_phrase).offset = simple.offset;
                (*c_phrase).duration = simple.duration;
                (*c_phrase).nbest = ptr::null_mut();
                (*c_phrase).nbest_count = 0;
            }
            Phrase::Detailed(detailed) => {
                let mut nbest = nbest_to_c(&detailed.nbest);
                let nbest_count = detailed.nbest.len() as i32;
                (*c_phrase).recognition_status = to_c_string(&detailed.recognition_status);
                (*c_phrase).display_text = ptr::null_mut();
                (*c_phrase).offset = detailed.offset;
                (*c_phrase).duration = detailed.duration;
                (*c_phrase).nbest = nbest.as_mut_ptr();
                (*c_phrase).nbest_count = nbest_count;
                mem::forget(nbest);
            }
            Phrase::Silence(silence) => {
                (*c_phrase).recognition_status = to_c_string(&silence.recognition_status);
                (*c_phrase).display_text = ptr::null_mut();
                (*c_phrase).offset = silence.offset;
                (*c_phrase).duration = silence.duration;
                (*c_phrase).nbest = ptr::null_mut();
                (*c_phrase).nbest_count = 0;
            }
            Phrase::Unknown => {
                (*c_phrase).recognition_status = to_c_string("Unknown");
                (*c_phrase).display_text = ptr::null_mut();
                (*c_phrase).offset = 0.0;
                (*c_phrase).duration = 0.0;
                (*c_phrase).nbest = ptr::null_mut();
                (*c_phrase).nbest_count = 0;
            }
        };
        0
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_synthesize(
    bing_speech: *mut BingSpeech,
    c_text: *mut c_char,
    c_font: c_int,
    c_output: *mut *mut c_void,
    c_output_len: *mut c_int,
) {
    let text = CString::from_raw(c_text).into_string().unwrap();
    if let Ok((_, _, Some(mut data))) = (*bing_speech).handle.synthesize(&text, font_from_c(c_font))
    {
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
pub unsafe extern "C" fn bing_speech_websocket_free(c_websocket: *mut BingSpeechWebsocket) {
    Box::from_raw(c_websocket);
}

#[no_mangle]
pub unsafe extern "C" fn bing_speech_websocket_audio(
    handle: *mut BingSpeechWebsocket,
    audio: *const u8,
    audio_size: usize,
) -> i32 {
    const BUFFER_SIZE: usize = 4096;

    let audio: Vec<u8> = Vec::from_raw_parts(audio as *mut u8, audio_size, audio_size);
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
    match c_font {
        0 => voice::ar_eg::HODA,
        1 => voice::ar_sa::NAAYF,
        2 => voice::bg_bg::IVAN,
        3 => voice::ca_es::HERENA_RUS,
        4 => voice::ca_cz::JAKUB,
        5 => voice::da_dk::HELLE_RUS,
        6 => voice::de_at::MICHAEL,
        7 => voice::de_ch::KARSTEN,
        8 => voice::de_de::HEDDA,
        9 => voice::de_de::HEDDA_RUS,
        10 => voice::de_de::STEFAN_APOLLO,
        11 => voice::el_gr::STEFANOS,
        12 => voice::en_au::CATHERINE,
        13 => voice::en_au::HAYLEY_RUS,
        14 => voice::en_ca::LINDA,
        15 => voice::en_ca::HEATHER_RUS,
        16 => voice::en_gb::SUSAN_APOLLO,
        17 => voice::en_gb::HAZEL_RUS,
        18 => voice::en_gb::GEORGE_APOLLO,
        19 => voice::en_ie::SEAN,
        20 => voice::en_in::HEERA_APOLLO,
        21 => voice::en_in::PRIYA_RUS,
        22 => voice::en_in::RAVI_APOLLO,
        23 => voice::en_us::ZIRA_RUS,
        24 => voice::en_us::JESSA_RUS,
        25 => voice::en_us::BENJAMIN_RUS,
        26 => voice::es_es::LAURA_APOLLO,
        27 => voice::es_es::HELENA_RUS,
        28 => voice::es_es::PABLO_APOLLO,
        29 => voice::es_mx::HILDA_RUS,
        30 => voice::es_mx::RAUL_APOLLO,
        31 => voice::fi_fi::HEIDI_RUS,
        32 => voice::fr_ca::CAROLINE,
        33 => voice::fr_ca::HARMONIE_RUS,
        34 => voice::fr_ch::GUILLAUME,
        35 => voice::fr_fr::JULIE_APOLLO,
        36 => voice::fr_fr::HORTENSE_RUS,
        37 => voice::fr_fr::PAUL_APOLLO,
        38 => voice::he_il::ASAF,
        39 => voice::hi_in::KALPANA_APOLLO,
        40 => voice::hi_in::KALPANA,
        41 => voice::hi_in::HEMANT,
        42 => voice::hr_hr::MATEJ,
        43 => voice::hu_hu::SZABOLCS,
        44 => voice::id_id::ANDIKA,
        45 => voice::it_it::COSIMA_APOLLO,
        46 => voice::ja_jp::AYUMI_APOLLO,
        47 => voice::ja_jp::ICHIRO_APOLLO,
        48 => voice::ja_jp::HARUKA_RUS,
        49 => voice::ja_jp::LUCIA_RUS,
        50 => voice::ja_jp::EKATERINA_RUS,
        51 => voice::ko_kr::HEAMI_RUS,
        52 => voice::ms_my::RIZWAN,
        53 => voice::nb_no::HULDA_RUS,
        54 => voice::nl_nl::HANNA_RUS,
        55 => voice::pt_br::HELOISA_RUS,
        56 => voice::pt_br::DANIEL_APOLLO,
        57 => voice::ro_ro::ANDREI,
        58 => voice::ru_ru::IRINA_APOLLO,
        59 => voice::ru_ru::PAVEL_APOLLO,
        60 => voice::sk_sk::FILIP,
        61 => voice::sv_se::HEDVIG_RUS,
        62 => voice::ta_in::VALLUVAR,
        63 => voice::th_th::PATTARA,
        64 => voice::tr_tr::SEDA_RUS,
        65 => voice::vi_vn::AN,
        66 => voice::zh_cn::HUIHUI_RUS,
        67 => voice::zh_cn::YAOYAO_APOLLO,
        68 => voice::zh_cn::KANGKANG_APOLLO,
        69 => voice::zh_hk::TRACY_APOLLO,
        70 => voice::zh_hk::TRACY_RUS,
        71 => voice::zh_hk::DANNY_APOLLO,
        72 => voice::zh_tw::YATING_APOLLO,
        73 => voice::zh_tw::HANHAN_RUS,
        74 => voice::zh_tw::ZHIWEI_APOLLO,
        _ => voice::en_us::JESSA_RUS,
    }
}

#[no_mangle]
pub static MODE_INTERACTIVE: i32 = 0;
#[no_mangle]
pub static MODE_DICTATION: i32 = 1;
#[no_mangle]
pub static MODE_CONVERSATION: i32 = 2;

#[no_mangle]
pub static LANGUAGE_ARABIC_EGYPT: i32 = 0;
#[no_mangle]
pub static LANGUAGE_CATALAN_SPAIN: i32 = 1;
#[no_mangle]
pub static LANGUAGE_CHINESE_CHINA: i32 = 2;
#[no_mangle]
pub static LANGUAGE_CHINESE_HONG_KONG: i32 = 3;
#[no_mangle]
pub static LANGUAGE_CHINESE_TAIWAN: i32 = 4;
#[no_mangle]
pub static LANGUAGE_DANISH_DENMARK: i32 = 5;
#[no_mangle]
pub static LANGUAGE_DUTCH_NETHERLANDS: i32 = 6;
#[no_mangle]
pub static LANGUAGE_ENGLISH_AUSTRALIA: i32 = 7;
#[no_mangle]
pub static LANGUAGE_ENGLISH_CANADA: i32 = 8;
#[no_mangle]
pub static LANGUAGE_ENGLISH_INDIA: i32 = 9;
#[no_mangle]
pub static LANGUAGE_ENGLISH_NEW_ZEALAND: i32 = 10;
#[no_mangle]
pub static LANGUAGE_ENGLISH_UNITED_KINGDOM: i32 = 11;
#[no_mangle]
pub static LANGUAGE_ENGLISH_UNITED_STATES: i32 = 12;
#[no_mangle]
pub static LANGUAGE_FINNISH_FINLAND: i32 = 13;
#[no_mangle]
pub static LANGUAGE_FRENCH_CANADA: i32 = 14;
#[no_mangle]
pub static LANGUAGE_FRENCH_FRANCE: i32 = 15;
#[no_mangle]
pub static LANGUAGE_GERMAN_GERMANY: i32 = 16;
#[no_mangle]
pub static LANGUAGE_HINDI_INDIA: i32 = 17;
#[no_mangle]
pub static LANGUAGE_ITALIAN_ITALY: i32 = 18;
#[no_mangle]
pub static LANGUAGE_JAPANESE_JAPAN: i32 = 19;
#[no_mangle]
pub static LANGUAGE_KOREAN_KOREA: i32 = 20;
#[no_mangle]
pub static LANGUAGE_NORWEGIAN_NORWAY: i32 = 21;
#[no_mangle]
pub static LANGUAGE_POLISH_POLAND: i32 = 22;
#[no_mangle]
pub static LANGUAGE_PORTUGUESE_BRAZIL: i32 = 23;
#[no_mangle]
pub static LANGUAGE_PORTUGUESE_PORTUGAL: i32 = 24;
#[no_mangle]
pub static LANGUAGE_RUSSIAN_RUSSIA: i32 = 25;
#[no_mangle]
pub static LANGUAGE_SPANISH_MEXICO: i32 = 26;
#[no_mangle]
pub static LANGUAGE_SPANISH_SPAIN: i32 = 27;
#[no_mangle]
pub static LANGUAGE_SWEDISH_SWEDEN: i32 = 28;

#[no_mangle]
pub static FORMAT_SIMPLE: i32 = 0;
#[no_mangle]
pub static FORMAT_DETAILED: i32 = 1;

#[no_mangle]
pub static VOICE_FONT_AR_EG_HODA: i32 = 0;
#[no_mangle]
pub static VOICE_FONT_AR_SA_NAAYF: i32 = 1;
#[no_mangle]
pub static VOICE_FONT_BG_BG_IVAN: i32 = 2;
#[no_mangle]
pub static VOICE_FONT_CA_ES_HERENA_RUS: i32 = 3;
#[no_mangle]
pub static VOICE_FONT_CA_CZ_JAKUB: i32 = 4;
#[no_mangle]
pub static VOICE_FONT_DA_DK_HELLE_RUS: i32 = 5;
#[no_mangle]
pub static VOICE_FONT_DE_AT_MICHAEL: i32 = 6;
#[no_mangle]
pub static VOICE_FONT_DE_CH_KARSTEN: i32 = 7;
#[no_mangle]
pub static VOICE_FONT_DE_DE_HEDDA: i32 = 8;
#[no_mangle]
pub static VOICE_FONT_DE_DE_HEDDA_RUS: i32 = 9;
#[no_mangle]
pub static VOICE_FONT_DE_DE_STEFAN_APOLLO: i32 = 10;
#[no_mangle]
pub static VOICE_FONT_EL_GR_STEFANOS: i32 = 11;
#[no_mangle]
pub static VOICE_FONT_EN_AU_CATHERINE: i32 = 12;
#[no_mangle]
pub static VOICE_FONT_EN_AU_HAYLEY_RUS: i32 = 13;
#[no_mangle]
pub static VOICE_FONT_EN_CA_LINDA: i32 = 14;
#[no_mangle]
pub static VOICE_FONT_EN_CA_HEATHER_RUS: i32 = 15;
#[no_mangle]
pub static VOICE_FONT_EN_GB_SUSAN_APOLLO: i32 = 16;
#[no_mangle]
pub static VOICE_FONT_EN_GB_HAZEL_RUS: i32 = 17;
#[no_mangle]
pub static VOICE_FONT_EN_GB_GEORGE_APOLLO: i32 = 18;
#[no_mangle]
pub static VOICE_FONT_EN_IE_SEAN: i32 = 19;
#[no_mangle]
pub static VOICE_FONT_EN_IN_HEERA_APOLLO: i32 = 20;
#[no_mangle]
pub static VOICE_FONT_EN_IN_PRIYA_RUS: i32 = 21;
#[no_mangle]
pub static VOICE_FONT_EN_IN_RAVI_APOLLO: i32 = 22;
#[no_mangle]
pub static VOICE_FONT_EN_US_ZIRA_RUS: i32 = 23;
#[no_mangle]
pub static VOICE_FONT_EN_US_JESSA_RUS: i32 = 24;
#[no_mangle]
pub static VOICE_FONT_EN_US_BENJAMIN_RUS: i32 = 25;
#[no_mangle]
pub static VOICE_FONT_ES_ES_LAURA_APOLLO: i32 = 26;
#[no_mangle]
pub static VOICE_FONT_ES_ES_HELENA_RUS: i32 = 27;
#[no_mangle]
pub static VOICE_FONT_ES_ES_PABLO_APOLLO: i32 = 28;
#[no_mangle]
pub static VOICE_FONT_ES_MX_HILDA_RUS: i32 = 29;
#[no_mangle]
pub static VOICE_FONT_ES_MX_RAUL_APOLLO: i32 = 30;
#[no_mangle]
pub static VOICE_FONT_FI_FI_HEIDI_RUS: i32 = 31;
#[no_mangle]
pub static VOICE_FONT_FR_CA_CAROLINE: i32 = 32;
#[no_mangle]
pub static VOICE_FONT_FR_CA_HARMONIE_RUS: i32 = 33;
#[no_mangle]
pub static VOICE_FONT_FR_CH_GUILLAUME: i32 = 34;
#[no_mangle]
pub static VOICE_FONT_FR_FR_JULIE_APOLLO: i32 = 35;
#[no_mangle]
pub static VOICE_FONT_FR_FR_HORTENSE_RUS: i32 = 36;
#[no_mangle]
pub static VOICE_FONT_FR_FR_PAUL_APOLLO: i32 = 37;
#[no_mangle]
pub static VOICE_FONT_HE_IL_ASAF: i32 = 38;
#[no_mangle]
pub static VOICE_FONT_HI_IN_KALPANA_APOLLO: i32 = 39;
#[no_mangle]
pub static VOICE_FONT_HI_IN_KALPANA: i32 = 40;
#[no_mangle]
pub static VOICE_FONT_HI_IN_HEMANT: i32 = 41;
#[no_mangle]
pub static VOICE_FONT_HR_HR_MATEJ: i32 = 42;
#[no_mangle]
pub static VOICE_FONT_HU_HU_SZABOLCS: i32 = 43;
#[no_mangle]
pub static VOICE_FONT_ID_ID_ANDIKA: i32 = 44;
#[no_mangle]
pub static VOICE_FONT_IT_IT_COSIMA_APOLLO: i32 = 45;
#[no_mangle]
pub static VOICE_FONT_JA_JP_AYUMI_APOLLO: i32 = 46;
#[no_mangle]
pub static VOICE_FONT_JA_JP_ICHIRO_APOLLO: i32 = 47;
#[no_mangle]
pub static VOICE_FONT_JA_JP_HARUKA_RUS: i32 = 48;
#[no_mangle]
pub static VOICE_FONT_JA_JP_LUCIA_RUS: i32 = 49;
#[no_mangle]
pub static VOICE_FONT_JA_JP_EKATERINA_RUS: i32 = 50;
#[no_mangle]
pub static VOICE_FONT_KO_KR_HEAMI_RUS: i32 = 51;
#[no_mangle]
pub static VOICE_FONT_MS_MY_RIZWAN: i32 = 52;
#[no_mangle]
pub static VOICE_FONT_NB_NO_HULDA_RUS: i32 = 53;
#[no_mangle]
pub static VOICE_FONT_NL_NL_HANNA_RUS: i32 = 54;
#[no_mangle]
pub static VOICE_FONT_PT_BR_HELOISA_RUS: i32 = 55;
#[no_mangle]
pub static VOICE_FONT_PT_BR_DANIEL_APOLLO: i32 = 56;
#[no_mangle]
pub static VOICE_FONT_RO_RO_ANDREI: i32 = 57;
#[no_mangle]
pub static VOICE_FONT_RU_RU_IRINA_APOLLO: i32 = 58;
#[no_mangle]
pub static VOICE_FONT_RU_RU_PAVEL_APOLLO: i32 = 59;
#[no_mangle]
pub static VOICE_FONT_SK_SK_FILIP: i32 = 60;
#[no_mangle]
pub static VOICE_FONT_SV_SE_HEDVIG_RUS: i32 = 61;
#[no_mangle]
pub static VOICE_FONT_TA_IN_VALLUVAR: i32 = 62;
#[no_mangle]
pub static VOICE_FONT_TH_TH_PATTARA: i32 = 63;
#[no_mangle]
pub static VOICE_FONT_TR_TR_SEDA_RUS: i32 = 64;
#[no_mangle]
pub static VOICE_FONT_VI_VN_AN: i32 = 65;
#[no_mangle]
pub static VOICE_FONT_ZH_CN_HUIHUI_RUS: i32 = 66;
#[no_mangle]
pub static VOICE_FONT_ZH_CN_YAOYAO_APOLLO: i32 = 67;
#[no_mangle]
pub static VOICE_FONT_ZH_CN_KANGKANG_APOLLO: i32 = 68;
#[no_mangle]
pub static VOICE_FONT_ZH_HK_TRACY_APOLLO: i32 = 69;
#[no_mangle]
pub static VOICE_FONT_ZH_HK_TRACY_RUS: i32 = 70;
#[no_mangle]
pub static VOICE_FONT_ZH_HK_DANNY_APOLLO: i32 = 71;
#[no_mangle]
pub static VOICE_FONT_ZH_TW_YATING_APOLLO: i32 = 72;
#[no_mangle]
pub static VOICE_FONT_ZH_TW_HANHAN_RUS: i32 = 73;
#[no_mangle]
pub static VOICE_FONT_ZH_TW_ZHIWEI_APOLLO: i32 = 74;
