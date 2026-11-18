//! Reads WAV files into 16 kHz mono, with the same conversion as live
//! capture (so tests and benchmarks hear what the app hears).

use std::path::Path;

use esperanto_core::AudioSegment;
use hound::{SampleFormat, WavReader};

use crate::AudioError;
use crate::resample::ToMono16k;

pub fn load_wav(path: &Path) -> Result<AudioSegment, AudioError> {
    let invalid =
        |e: &dyn std::fmt::Display| AudioError::InvalidFile(format!("{}: {e}", path.display()));
    let reader = WavReader::open(path).map_err(|e| invalid(&e))?;
    let spec = reader.spec();
    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| invalid(&e))?,
        SampleFormat::Int => {
            if !(1..=32).contains(&spec.bits_per_sample) {
                return Err(invalid(&format!(
                    "unsupported {} bits per sample",
                    spec.bits_per_sample
                )));
            }
            let scale = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()
                .map_err(|e| invalid(&e))?
        }
    };
    let mut converter = ToMono16k::new(spec.sample_rate, usize::from(spec.channels))?;
    let mut samples = converter.process(&interleaved)?.to_vec();
    samples.extend_from_slice(converter.flush()?);
    Ok(AudioSegment {
        samples,
        sample_rate: AudioSegment::SAMPLE_RATE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmixes_and_resamples_to_16k() {
        let dir = std::env::temp_dir().join(format!("esperanto-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stereo.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 32_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..32_000 {
            writer.write_sample(i16::MAX / 2).unwrap();
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();

        let audio = load_wav(&path).unwrap();
        assert_eq!(audio.sample_rate, 16_000);
        assert!(
            (15_900..=16_000).contains(&audio.samples.len()),
            "{}",
            audio.samples.len()
        );
        assert!((audio.samples[8_000] - 0.25).abs() < 0.01);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reports_files_that_are_not_wav() {
        let path = std::env::temp_dir().join(format!("esperanto-not-wav-{}", std::process::id()));
        std::fs::write(&path, b"not audio").unwrap();
        assert!(matches!(load_wav(&path), Err(AudioError::InvalidFile(_))));
        std::fs::remove_file(path).unwrap();
    }
}
