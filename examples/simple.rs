extern crate bing_rs;

use bing_rs::speech::*;
use std::fs::File;
use std::io::Read;

const SUBSCRIPTION_KEY: &str = "SUBSCRIPTION_KEY";

fn main() {
    let mut client = Speech::new(&SUBSCRIPTION_KEY).unwrap();
    assert!(client.fetch_token().is_ok());
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    assert!(file.read_to_end(&mut audio).is_ok());

    let mode = Mode::Interactive(InteractiveDictationLanguage::EnglishUnitedStates);
    match client.recognize(audio, &mode, &Format::Simple) {
        Ok((_, _, Some(phrase))) => println!("{}", phrase),
        Ok((_, _, None)) => println!("Empty response"),
        Err(err) => println!("Error: {}", err),
    }
}
