//! Prints the speech segments the listener finds in a 16 kHz mono WAV.
//! Useful to tune the VAD and segmenter.
//!
//! cargo run -p esperanto-core --example segment_wav -- path/to/audio.wav

use esperanto_core::{ListenEvent, Listener, SegmenterConfig, SpeechEvent};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: segment_wav <file.wav>");
    let mut reader = hound::WavReader::open(&path).expect("cannot open WAV");
    let spec = reader.spec();
    assert_eq!(
        (spec.sample_rate, spec.channels),
        (16_000, 1),
        "expected 16 kHz mono"
    );
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| f32::from(s.expect("bad sample")) / 32_768.0)
        .collect();

    let mut listener = Listener::new(SegmenterConfig::default());
    let mut events = Vec::new();
    let mut position = 0;
    // Feed in 10 ms chunks, like the capture thread.
    for chunk in samples.chunks(160) {
        listener.push(chunk, &mut events);
        position += chunk.len();
        for event in events.drain(..) {
            let at = position as f64 / 16_000.0;
            match event {
                ListenEvent::Speech(SpeechEvent::Started) => println!("{at:6.2}s  start"),
                ListenEvent::Speech(SpeechEvent::Ended { segment, reason }) => println!(
                    "{at:6.2}s  end    {:.2}s segment ({reason:?})",
                    segment.duration_secs()
                ),
                ListenEvent::Speech(SpeechEvent::Discarded) => println!("{at:6.2}s  discarded"),
                ListenEvent::Level { .. } => {}
            }
        }
    }
    listener.flush(&mut events);
    for event in events {
        if let ListenEvent::Speech(SpeechEvent::Ended { segment, reason }) = event {
            println!(
                "   end    {:.2}s segment ({reason:?})",
                segment.duration_secs()
            );
        }
    }
}
