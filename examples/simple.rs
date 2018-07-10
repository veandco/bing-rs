extern crate bing_rs;

use bing_rs::speech::*;
use std::fs::File;
use std::io::Read;

const SUBSCRIPTION_KEY: &'static str = "SUBSCRIPTION_KEY";

fn main() {
    let mut client = Speech::new(SUBSCRIPTION_KEY).unwrap();
    assert!(client.fetch_token().is_ok());
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    assert!(file.read_to_end(&mut audio).is_ok());

    match client.recognize(audio, Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates), Format::Simple) {
        Ok((_, _, Some(ref response))) => {
            match response {
                Response::Simple(response) => {
                    println!("RecognitionStatus: {}", response.recognition_status);
                    println!("DisplayText: {}", response.display_text);
                    println!("Offset: {}", response.offset);
                    println!("Duration: {}", response.duration);
                },
                _ => println!("Not handling detailed response"),
            }
        },
        Ok((_, _, None)) => println!("Ok but no result"),
        Err(err) => println!("Error: {}", err),
    }
}