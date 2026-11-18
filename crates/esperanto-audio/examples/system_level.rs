//! Captures system audio for a few seconds and prints its level, to check
//! the Core Audio tap and its permission.
//!
//! cargo run -p esperanto-audio --example system_level -- [seconds]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use esperanto_audio::{AudioError, CaptureHandler, SYSTEM_AUDIO, start_capture};

#[derive(Clone, Default)]
struct Stats(Arc<Mutex<(usize, f64, f32)>>); // samples, sum of squares, peak

impl CaptureHandler for Stats {
    fn samples(&mut self, samples: &[f32]) {
        let mut s = self.0.lock().unwrap();
        s.0 += samples.len();
        s.1 += samples.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
        s.2 = samples.iter().fold(s.2, |m, v| m.max(v.abs()));
    }
    fn error(&mut self, error: AudioError) {
        eprintln!("error: {error}");
    }
}

fn main() {
    let secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let stats = Stats::default();
    let capture = start_capture(Some(SYSTEM_AUDIO), stats.clone()).expect("cannot start capture");
    println!(
        "capturing system audio from {:?} for {secs} s",
        capture.device_name()
    );
    for _ in 0..secs {
        std::thread::sleep(Duration::from_secs(1));
        let mut s = stats.0.lock().unwrap();
        let rms = if s.0 > 0 {
            (s.1 / s.0 as f64).sqrt()
        } else {
            0.0
        };
        println!("  {} samples, rms {:.4}, peak {:.4}", s.0, rms, s.2);
        *s = (0, 0.0, 0.0);
    }
    capture.stop();
}
