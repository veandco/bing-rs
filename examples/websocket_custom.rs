extern crate bing_rs;
#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate chrono;
extern crate env_logger;
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

fn main() {
    env_logger::init();

    // Setup running state variable
    let running = Arc::new(AtomicBool::new(true));

    // Setup OS signal handler
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let (_sdone, rdone) = chan::sync::<()>(0);

    // Fetch token
    let mut client = Speech::new(&env::var("SUBSCRIPTION_KEY").unwrap()).unwrap();
    client.set_custom_speech(true);
    client.set_endpoint_id(&env::var("ENDPOINT_ID").unwrap());
    client.fetch_token().unwrap();
    client.auto_fetch_token();

    // Open Websocket Connection
    let mut ws = Websocket::new(client.token.clone());
    let ws_rx = ws.server_event_receiver();
    let mode = Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates);
    ws.open(
        &mode,
        &Format::Simple,
        true,
        &env::var("ENDPOINT_ID").unwrap(),
    );
    ws.config(&default_speech_config()).unwrap();

    // Load audio data
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    file.read_to_end(&mut audio).unwrap();

    // Add some silence to the end of audio data
    for _ in 0..1024 * 100 {
        audio.push(0);
    }

    // Run continuous audio data transfer in another thread
    let ws = Arc::new(Mutex::new(ws));
    let ws_1 = ws.clone();
    let running_1 = running.clone();
    thread::spawn(move || {
        let mut i = 0;

        while running_1.load(Ordering::Relaxed) {
            const BUFFER_SIZE: usize = 4096;

            // Send audio data to Bing
            if let Ok(mut ws) = ws_1.lock() {
                ws.audio(&audio[i..i + BUFFER_SIZE].to_vec()).unwrap();
            }

            // Go to the next audio data chunk
            i += BUFFER_SIZE;
            if audio.len() - i < BUFFER_SIZE {
                i = 0;
            }

            // Wait for some time to simulate real microphone audio data period
            thread::sleep(Duration::from_millis(256));
        }
    });

    let running_2 = running.clone();
    thread::spawn(move || {
        while running_2.load(Ordering::Relaxed) {
            match ws_rx.recv() {
                Ok(event) => match event {
                    ServerEvent::TurnStart => println!("Bing Speech: Turn Start\n"),
                    ServerEvent::TurnEnd => println!("Bing Speech: Turn End\n"),
                    ServerEvent::SpeechStartDetected => {
                        println!("Bing Speech: Speech Start Detected\n")
                    }
                    ServerEvent::SpeechHypothesis(hypothesis) => {
                        println!("Bing Speech: Speech Hypothesis");
                        println!("==============================");
                        println!("{}\n", hypothesis);
                    }
                    ServerEvent::SpeechEndDetected => {
                        println!("Bing Speech: Speech End Detected\n")
                    }
                    ServerEvent::SpeechPhrase(phrase) => {
                        println!("Bing Speech: Speech Phrase");
                        println!("==========================");
                        println!("{}\n", phrase);
                    }
                    _ => {}
                },
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
    });

    // Blocks until we close the program manually via SIGINT or SIGTERM
    chan_select! {
        signal.recv() -> signal => {
            println!("Received signal: {:?}", signal);
            running.store(false, Ordering::Relaxed);
        },
        rdone.recv() => {
            println!("Program completed normally.");
        },
    }

    // Close the Websocket connection
    let mut ws = ws.lock().unwrap();
    (*ws).close();
}
