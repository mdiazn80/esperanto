//! Audio capture for Esperanto.
//!
//! Captures an input device with `cpal`, converts it to 16 kHz mono on a
//! separate thread and hands it to a callback. The real-time audio callback
//! only copies samples into a lock-free ring buffer.

mod capture;
mod devices;
mod permission;
mod resample;
mod wav;

pub use capture::{Capture, CaptureHandler, REPLAY_PREFIX, SYSTEM_AUDIO, start_capture};
pub use devices::{InputDevice, list_input_devices};
pub use permission::{MicrophonePermission, microphone_permission, request_microphone_permission};
pub use wav::load_wav;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no input device available")]
    NoInputDevice,
    #[error("no output device to capture system audio from")]
    NoOutputDevice,
    #[error("input device not found: {0}")]
    DeviceNotFound(String),
    #[error("microphone access denied")]
    PermissionDenied,
    #[error("audio stream error: {0}")]
    Stream(String),
    #[error("invalid audio file: {0}")]
    InvalidFile(String),
}
