extern crate bing_rs;

use bing_rs::speech::Speech;
use std::fs::File;
use std::io::Read;

const SUBSCRIPTION_KEY: &'static str = "SUBSCRIPTION_KEY";

fn main() {
    let mut client = Speech::new(SUBSCRIPTION_KEY).unwrap();
    assert!(client.fetch_token().is_ok());
    let mut file = File::open("assets/audio.raw").unwrap();
    let mut audio = Vec::new();
    assert!(file.read_to_end(&mut audio).is_ok());
    if let Ok((_, _, Some(ref response))) = client.recognize(audio) {
        println!("RecognitionStatus: {}", response.recognition_status);
        println!("Offset: {}", response.offset);
        println!("Duration: {}", response.duration);
        println!("NBest");
        println!("========");

        for (i, ref result) in response.nbest.iter().enumerate() {
            println!("#{}", i);
            println!("--------");
            println!("    Confidence: {}", result.confidence);
            println!("    Lexical: {}", result.lexical);
            println!("    ITN: {}", result.itn);
            println!("    MaskedITN: {}", result.masked_itn);
            println!("    Display: {}", result.display);
        }
    }
}