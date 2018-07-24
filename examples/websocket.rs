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
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bing_rs::speech::websocket::*;
use bing_rs::speech::*;
use chan_signal::Signal;

struct MyHandler;

impl Handler for MyHandler {
    fn on_turn_start(&mut self) {
        println!("Bing Speech: Turn Start\n");
    }

    fn on_turn_end(&mut self) {
        println!("Bing Speech: Turn End\n");
    }

    fn on_speech_start_detected(&mut self) {
        println!("Bing Speech: Speech Start Detected\n");
    }

    fn on_speech_hypothesis(&mut self, hypothesis: Hypothesis) {
        println!("Bing Speech: Speech Hypothesis");
        println!("==============================");
        println!("{}\n", hypothesis);
    }

    fn on_speech_end_detected(&mut self) {
        println!("Bing Speech: Speech End Detected\n");
    }

    fn on_speech_phrase(&mut self, phrase: Phrase) {
        println!("Bing Speech: Speech Phrase");
        println!("==========================");
        println!("{}\n", phrase);
    }
}

fn main() {
    env_logger::init();

    // Setup running state variable
    let running = Arc::new(Mutex::new(true));

    // Setup OS signal handler
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let (_sdone, rdone) = chan::sync::<()>(0);

    // Fetch token
    let mut client = Speech::new(&env::var("SUBSCRIPTION_KEY").unwrap()).unwrap();
    client.fetch_token().unwrap();
    client.auto_fetch_token();

    // Load audio data
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    file.read_to_end(&mut audio).unwrap();

    // Add some silence to the end of audio data
    for _ in 0..1024 * 50 {
        audio.push(0);
    }

    // Connect to Bing Speech via Websocket
    let mode = Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates);
    let handle = Arc::new(Mutex::new(client
        .websocket(&mode, Format::Detailed, Arc::new(Mutex::new(MyHandler {})))
        .unwrap()));

    // Run continuous audio data transfer in another thread
    let handle_clone = handle.clone();
    let running_clone = running.clone();
    thread::spawn(move || {
        let mut i = 0;

        while *running_clone.lock().unwrap() {
            const BUFFER_SIZE: usize = 4096;

            {
                // Send audio data to Bing Speech
                let handle = handle_clone.lock().unwrap();
                (*handle)
                    .send_tx
                    .send(ClientEvent::Audio(audio[i..i + BUFFER_SIZE].to_vec()))
                    .unwrap();
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

    // Blocks until we close the program manually via SIGINT or SIGTERM
    chan_select! {
        signal.recv() -> signal => {
            println!("Received signal: {:?}", signal);
            *running.lock().unwrap() = false;
        },
        rdone.recv() => {
            println!("Program completed normally.");
        },
    }

    // Close the Websocket connection
    let mut handle = handle.lock().unwrap();
    (*handle).close();
}
