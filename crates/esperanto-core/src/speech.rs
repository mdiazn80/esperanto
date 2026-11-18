//! The unit of work sent to an inference provider.

use crate::{Language, SourceLanguage};

/// Mono PCM audio, as f32 samples in `[-1.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSegment {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl AudioSegment {
    /// Sample rate every provider receives.
    pub const SAMPLE_RATE: u32 = 16_000;

    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / f64::from(self.sample_rate)
    }
}

/// What to do with a segment: transcribe it, and optionally translate it.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechTask {
    pub audio: AudioSegment,
    pub source_language: SourceLanguage,
    /// `None` = transcription only.
    pub target_language: Option<Language>,
}

/// Result of a [`SpeechTask`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpeechResult {
    pub transcript: String,
    pub detected_language: Option<Language>,
    pub translation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_from_samples() {
        let audio = AudioSegment {
            samples: vec![0.0; 24_000],
            sample_rate: 16_000,
        };
        assert!((audio.duration_secs() - 1.5).abs() < f64::EPSILON);
    }
}
