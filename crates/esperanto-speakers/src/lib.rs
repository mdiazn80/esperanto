//! Speaker labels.
//!
//! - [`SpeakerEmbedder`] turns a phrase (16 kHz mono) into a voice print
//!   with a speaker-embedding model run by `sherpa-onnx`;
//! - [`SpeakerTracker`] groups the prints of a session into speakers.

mod embedder;
mod tracker;

pub use embedder::{SpeakerEmbedder, SpeakersError};
pub use tracker::{SpeakerTracker, TrackerConfig};

/// Phrases shorter than this (in seconds) give unreliable voice prints.
pub const MIN_RELIABLE_SECS: f64 = 2.0;
