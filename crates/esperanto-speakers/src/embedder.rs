use std::path::Path;

use sherpa_onnx::{SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig};

#[derive(Debug, thiserror::Error)]
pub enum SpeakersError {
    #[error("could not load the speaker model {0}")]
    Load(String),
}

/// Computes voice prints with a speaker-embedding model (ONNX).
pub struct SpeakerEmbedder {
    extractor: SpeakerEmbeddingExtractor,
}

/// The models expect 16 kHz audio, like the rest of the pipeline.
const SAMPLE_RATE: i32 = 16_000;

impl SpeakerEmbedder {
    pub fn new(model: &Path) -> Result<Self, SpeakersError> {
        let config = SpeakerEmbeddingExtractorConfig {
            model: Some(model.to_string_lossy().into_owned()),
            num_threads: 1,
            debug: false,
            provider: Some("cpu".into()),
        };
        let extractor = SpeakerEmbeddingExtractor::create(&config)
            .ok_or_else(|| SpeakersError::Load(model.display().to_string()))?;
        Ok(Self { extractor })
    }

    /// Length of the voice prints.
    pub fn dim(&self) -> usize {
        self.extractor.dim() as usize
    }

    /// The voice print of a phrase, or `None` if it is too short.
    pub fn embed(&self, samples: &[f32]) -> Option<Vec<f32>> {
        let stream = self.extractor.create_stream()?;
        stream.accept_waveform(SAMPLE_RATE, samples);
        stream.input_finished();
        if !self.extractor.is_ready(&stream) {
            return None;
        }
        self.extractor.compute(&stream)
    }
}
