//! Checks the speaker model and the clustering on real recordings:
//! prints the cosine similarity of every pair of clips and the speaker the
//! tracker assigns to each, in order.
//!
//! cargo run -p esperanto-speakers --example speakers -- MODEL.onnx a.wav b.wav …
//!
//! With `--halves` first, each clip is split in two instead: the halves are
//! the same speaker, different clips (usually) different speakers. It prints
//! both similarity distributions, to choose the clustering thresholds.
//! (16 kHz mono WAV files, 16-bit or float)

use esperanto_speakers::{MIN_RELIABLE_SECS, SpeakerEmbedder, SpeakerTracker};

fn read(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("readable WAV");
    let spec = reader.spec();
    assert!(
        spec.sample_rate == 16_000 && spec.channels == 1,
        "{path}: expected 16 kHz mono"
    );
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.expect("sample"))
            .collect(),
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| f32::from(s.expect("sample")) / f32::from(i16::MAX))
            .collect(),
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm(a) * norm(b))
}

/// Same-speaker (halves) vs different-speaker (other clips) similarities.
fn halves(embedder: &SpeakerEmbedder, clips: &[String]) {
    let mut pairs = Vec::new();
    for clip in clips {
        let samples = read(clip);
        let half = samples.len() / 2;
        if (half as f64) < 2.0 * 16_000.0 {
            continue; // halves under 2 s
        }
        let a = embedder.embed(&samples[..half]).expect("long enough");
        let b = embedder.embed(&samples[half..]).expect("long enough");
        pairs.push((a, b));
    }
    let mut same: Vec<f32> = pairs.iter().map(|(a, b)| cosine(a, b)).collect();
    let mut different = Vec::new();
    for (i, (a, _)) in pairs.iter().enumerate() {
        for (b, _) in &pairs[i + 1..] {
            different.push(cosine(a, b));
        }
    }
    let stats = |name: &str, v: &mut Vec<f32>| {
        v.sort_by(f32::total_cmp);
        let pct = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
        println!(
            "{name}: n={} min={:.2} p5={:.2} p50={:.2} p95={:.2} max={:.2}",
            v.len(),
            v[0],
            pct(0.05),
            pct(0.5),
            pct(0.95),
            v[v.len() - 1]
        );
    };
    stats("same speaker", &mut same);
    stats("different", &mut different);
    println!("histogram of different-clip similarities:");
    for bucket in -2..10 {
        let (lo, hi) = (bucket as f32 / 10.0, (bucket + 1) as f32 / 10.0);
        let n = different.iter().filter(|s| **s >= lo && **s < hi).count();
        println!(
            "  {lo:5.1}..{hi:4.1} {n:6} {}",
            "#".repeat(n * 60 / different.len().max(1))
        );
    }
    for threshold in [0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6] {
        let missed = same.iter().filter(|s| **s < threshold).count();
        let confused = different.iter().filter(|s| **s >= threshold).count();
        println!(
            "threshold {threshold:.2}: same split {:.1} %, different merged {:.2} %",
            100.0 * missed as f64 / same.len() as f64,
            100.0 * confused as f64 / different.len() as f64
        );
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let halves_mode = args.first().is_some_and(|a| a == "--halves");
    if halves_mode {
        args.remove(0);
    }
    let (model, clips) = args
        .split_first()
        .expect("usage: [--halves] MODEL.onnx WAV…");
    let embedder = SpeakerEmbedder::new(model.as_ref()).expect("model loads");
    println!("embedding size: {}", embedder.dim());
    if halves_mode {
        halves(&embedder, clips);
        return;
    }

    let mut prints = Vec::new();
    for clip in clips {
        let samples = read(clip);
        let started = std::time::Instant::now();
        let print = embedder.embed(&samples).expect("long enough");
        println!(
            "{clip}: {:.1} s, embedded in {} ms",
            samples.len() as f64 / 16_000.0,
            started.elapsed().as_millis()
        );
        prints.push((samples.len() as f64 / 16_000.0, print));
    }

    println!("\nsimilarity:");
    for (i, (_, a)) in prints.iter().enumerate() {
        let row: Vec<String> = prints
            .iter()
            .map(|(_, b)| format!("{:5.2}", cosine(a, b)))
            .collect();
        println!("{:>3} {}", i, row.join(" "));
    }

    let mut tracker = SpeakerTracker::default();
    println!("\nassigned:");
    for (clip, (secs, print)) in clips.iter().zip(&prints) {
        let speaker = tracker.assign(print, *secs >= MIN_RELIABLE_SECS);
        println!("{clip}: {speaker:?}");
    }
}
