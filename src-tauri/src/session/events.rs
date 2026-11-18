//! What a session streams to the frontend through its `Channel`.

use esperanto_core::Language;
use serde::Serialize;
use specta::Type;

use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStatus {
    /// Inference keeps up.
    Live,
    /// Finished segments are queuing up.
    Behind,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionEvent {
    /// Input level (dBFS) and voice probability, ~20 times per second.
    Level {
        rms_db: f32,
        voice_probability: f32,
    },
    /// Speech started: a new segment (also after a 10 s cut).
    SpeechStarted {
        segment_id: u32,
    },
    /// Speech too short to keep; the segment is dropped.
    SpeechDiscarded {
        segment_id: u32,
    },
    /// Text so far for a segment still being spoken.
    Partial {
        segment_id: u32,
        stable: String,
        unstable: String,
    },
    /// Final transcript of a segment. `latencyMs`: from the end of the
    /// segment until this event. `startMs`/`endMs`: where the segment is in
    /// the session, in ms from its start.
    Final {
        segment_id: u32,
        text: String,
        language: Language,
        latency_ms: f64,
        start_ms: f64,
        end_ms: f64,
    },
    /// Who spoke a segment (speaker labels): speakers are numbered from 0
    /// in order of appearance. Sent before its `final`.
    Speaker {
        segment_id: u32,
        speaker: u32,
    },
    /// The segment had no speech the model could transcribe.
    Empty {
        segment_id: u32,
    },
    Translation {
        segment_id: u32,
        language: Language,
        text: String,
    },
    Status {
        status: PipelineStatus,
    },
    /// A segment couldn't be transcribed or translated; the session goes on.
    InferenceError {
        segment_id: u32,
        code: ErrorCode,
    },
    /// System audio has been completely silent for a while: either nothing
    /// is playing, or the system-audio permission was denied (macOS gives
    /// silence then, and has no public API to check). Sent once.
    SystemAudioSilent,
    /// Capture failed; the session must be stopped.
    Error {
        code: ErrorCode,
    },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionStarted {
    pub device_name: String,
    pub model_name: String,
    /// Host of the remote gateway; `null` when the model runs locally.
    pub remote_host: Option<String>,
    /// The chosen microphone wasn't connected; the system default is used.
    pub used_default_device: bool,
    /// The session's id in the history (also when history isn't kept).
    pub history_id: String,
    /// Phrases are labeled with their speaker (the setting is on and the
    /// speaker model is installed).
    pub speaker_labels: bool,
}
