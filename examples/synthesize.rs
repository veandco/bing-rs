extern crate bing_rs;
extern crate cpal;

use bing_rs::speech::*;
use std::env;
use std::sync::mpsc::channel;
use std::thread;

fn vec_u8_to_i16(input: &[u8]) -> Vec<i16> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < input.len() / 2 {
        let j = i * 2;
        let h = ((input[j + 1] as i16) << 8) as i16;
        let l = input[j] as i16;
        output.push(h | l);
        i += 1;
    }

    output
}

fn main() {
    let args: Vec<String> = env::args().map(|v| v).collect();
    let text = if args.len() > 1 {
        args[1].clone()
    } else {
        "Hello World!".to_string()
    };
    let mut client = Speech::new(&env::var("SUBSCRIPTION_KEY").unwrap()).unwrap();
    assert!(client.fetch_token().is_ok());

    match client.synthesize(&text, voice::en_us::JESSA_RUS) {
        Ok((_, _, Some(audio))) => {
            let device =
                cpal::default_output_device().expect("Failed to get default output device");
            let format = cpal::Format {
                channels: 1,
                sample_rate: cpal::SampleRate(16000),
                data_type: cpal::SampleFormat::I16,
            };
            let event_loop = cpal::EventLoop::new();
            let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
            let mut audio = vec_u8_to_i16(&audio);
            let (tx, rx) = channel();
            event_loop.play_stream(stream_id.clone());

            // Play the audio until it's finished
            thread::spawn(move || {
                event_loop.run(move |_, data| match data {
                    cpal::StreamData::Output {
                        buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
                    } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            if audio.len() == 0 {
                                tx.send(true).unwrap();
                                return;
                            }

                            let value = audio.remove(0);
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    }
                    _ => (),
                });
            });

            rx.recv().unwrap();
        }
        Ok((_, _, None)) => println!("Empty response"),
        Err(err) => println!("Error: {}", err),
    }
}
