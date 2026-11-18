//! Errors returned to the frontend.
//!
//! Rust never returns user-facing text: the frontend translates `code`.
//! `detail` is diagnostic information for logs only.

use esperanto_audio::AudioError;
use esperanto_models::DownloadError;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// A bug or an unexpected state (e.g. a poisoned lock).
    Internal,
    SettingsLoadFailed,
    SettingsSaveFailed,
    MicrophonePermissionDenied,
    NoInputDevice,
    NoOutputDevice,
    InputDeviceNotFound,
    AudioStreamFailed,
    SessionAlreadyRunning,
    ModelNotFound,
    DownloadInProgress,
    DownloadFailed,
    DownloadCancelled,
    ModelCorrupted,
    InsufficientDiskSpace,
    ModelDeleteFailed,
    ModelNotInstalled,
    LlamaServerNotFound,
    LlamaServerFailed,
    InferenceFailed,
    ProviderNotFound,
    ProviderInvalid,
    ProviderInsecureUrl,
    ProviderTokenMissing,
    ProviderUnreachable,
    ProviderUnauthorized,
    ProviderRejected,
    KeychainFailed,
    HistoryNotFound,
    HistoryFailed,
    ExportFailed,
}

#[derive(Debug, Serialize, Type, thiserror::Error)]
#[error("{code:?}: {detail}")]
pub struct CommandError {
    pub code: ErrorCode,
    pub detail: String,
}

impl CommandError {
    pub fn new(code: ErrorCode, detail: impl ToString) -> Self {
        Self {
            code,
            detail: detail.to_string(),
        }
    }

    pub fn internal(detail: impl ToString) -> Self {
        Self::new(ErrorCode::Internal, detail)
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

impl From<&AudioError> for ErrorCode {
    fn from(error: &AudioError) -> Self {
        match error {
            AudioError::NoInputDevice => ErrorCode::NoInputDevice,
            AudioError::NoOutputDevice => ErrorCode::NoOutputDevice,
            AudioError::DeviceNotFound(_) | AudioError::InvalidFile(_) => {
                ErrorCode::InputDeviceNotFound
            }
            AudioError::PermissionDenied => ErrorCode::MicrophonePermissionDenied,
            AudioError::Stream(_) => ErrorCode::AudioStreamFailed,
        }
    }
}

impl From<AudioError> for CommandError {
    fn from(error: AudioError) -> Self {
        Self::new(ErrorCode::from(&error), error)
    }
}

impl From<DownloadError> for CommandError {
    fn from(error: DownloadError) -> Self {
        let code = match &error {
            DownloadError::Cancelled => ErrorCode::DownloadCancelled,
            DownloadError::ChecksumMismatch { .. } => ErrorCode::ModelCorrupted,
            DownloadError::InsufficientSpace { .. } => ErrorCode::InsufficientDiskSpace,
            DownloadError::Http(_) | DownloadError::Status { .. } | DownloadError::Io(_) => {
                ErrorCode::DownloadFailed
            }
        };
        Self::new(code, error)
    }
}
