extern crate bing_rs;
#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate chrono;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate ws;

use std::env;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bing_rs::speech::websocket::*;
use bing_rs::speech::*;
use chan_signal::Signal;

struct MyHandler;

impl Handler for MyHandler {
    fn on_turn_start(&mut self) {
        println!("Turn Start\n");
    }

    fn on_turn_end(&mut self) {
        println!("Turn End\n");
    }

    fn on_speech_start(&mut self) {
        println!("Speech Start Detected\n");
    }

    fn on_speech_hypothesis(&mut self, hypothesis: Hypothesis) {
        println!("Speech Hypothesis");
        println!("=================");
        println!("{}\n", hypothesis);
    }

    fn on_speech_end(&mut self) {
        println!("Speech End Detected\n");
    }

    fn on_speech_phrase(&mut self, phrase: Phrase) {
        println!("Speech Phrase");
        println!("=============");
        println!("{}\n", phrase);
    }
}

fn main() {
    env_logger::init();

    // Setup running state variable
    let running = Arc::new(AtomicBool::new(true));

    // Setup OS signal handler
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    // Setup variables
    let awake = Arc::new(AtomicBool::new(true));

    // Load audio data
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    let mut i = 0;
    file.read_to_end(&mut audio).unwrap();

    // Add some silence to the end of audio data
    for _ in 0..1024 * 50 {
        audio.push(0);
    }

    // Switch awake mode periodically
    let awake_1 = awake.clone();
    let running_1 = running.clone();
    thread::spawn(move || {
        while running_1.load(Ordering::Relaxed) {
            let awake_neg = !awake_1.load(Ordering::Relaxed);
            awake_1.store(awake_neg, Ordering::Relaxed);
            thread::sleep(Duration::from_secs(5));
        }
    });

    // Setup Bing Speech Client
    let mut client = Speech::new(&env::var("SUBSCRIPTION_KEY").unwrap()).unwrap();
    let token = client.token.clone();
    client.fetch_token().unwrap();
    client.auto_fetch_token();

    // Setup Bing Speech Websocket
    let mode = Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates);
    let format = Format::Detailed;
    let handler = Arc::new(Mutex::new(MyHandler{}));
    let mut ws = Websocket::new();

    // Send audio data
    let awake_1 = awake.clone();
    let running_1 = running.clone();
    thread::spawn(move || {
        let mut awake = awake_1.load(Ordering::Relaxed);

        while running_1.load(Ordering::Relaxed) {
            let previous_awake = awake;
            awake = awake_1.load(Ordering::Relaxed);

            // When awake state changes, connect / disconnect Websocket
            if awake != previous_awake {
                if awake {
                    info!("Awake");
                    ws.connect(token.clone(), &mode, &format, false, "", handler.clone()).unwrap();
                } else {
                    info!("Sleep");
                    ws.disconnect().unwrap();
                }
            }

            // If we're awake, send audio data
            if awake {
                const BUFFER_SIZE: usize = 4096;

                // Send audio data to Bing
                if let Err(_) = ws.audio(&audio[i..i + BUFFER_SIZE].to_vec()) {
                    warn!("Failed to send audio");
                }

                // Go to the next audio data chunk
                i += BUFFER_SIZE;
                if audio.len() - i < BUFFER_SIZE {
                    i = 0;
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // Blocks until we close the program manually via SIGINT or SIGTERM
    chan_select! {
        signal.recv() -> signal => {
            println!("Received signal: {:?}", signal);
            running.store(false, Ordering::Relaxed);
        },
    }
}