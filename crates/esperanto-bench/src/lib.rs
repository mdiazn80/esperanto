//! Benchmark harness for Esperanto's inference providers.
//!
//! Replays a test set of audio clips through an OpenAI-compatible server and
//! measures transcription quality (WER), translation quality (chrF) and
//! latency.

pub mod manifest;
pub mod metrics;
pub mod report;
pub mod run;
