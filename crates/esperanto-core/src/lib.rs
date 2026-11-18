//! Core domain types shared by every Esperanto crate.

pub mod language;
pub mod listener;
pub mod segmenter;
pub mod speech;
pub mod stabilizer;

pub use language::{Language, SourceLanguage, TargetLanguage, translation_target};
pub use listener::{ListenEvent, Listener};
pub use segmenter::{SegmentEnd, Segmenter, SegmenterConfig, SpeechEvent};
pub use speech::{AudioSegment, SpeechResult, SpeechTask};
pub use stabilizer::{Partial, Stabilizer};
