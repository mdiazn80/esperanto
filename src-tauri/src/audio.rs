//! Audio devices and permissions, for the frontend.

use esperanto_audio::MicrophonePermission;
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;

use crate::error::CommandResult;
use crate::util::open_external;

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioInput {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum MicPermission {
    Granted,
    Denied,
    NotDetermined,
}

impl From<MicrophonePermission> for MicPermission {
    fn from(permission: MicrophonePermission) -> Self {
        match permission {
            MicrophonePermission::Granted => MicPermission::Granted,
            MicrophonePermission::Denied => MicPermission::Denied,
            MicrophonePermission::NotDetermined => MicPermission::NotDetermined,
        }
    }
}

/// Input devices. Async: CoreAudio enumeration can take a moment and must
/// not block the main thread.
#[tauri::command]
#[specta::specta]
pub async fn list_audio_inputs() -> CommandResult<Vec<AudioInput>> {
    let devices = esperanto_audio::list_input_devices()?;
    Ok(devices
        .into_iter()
        .map(|d| AudioInput {
            id: d.id,
            name: d.name,
            is_default: d.is_default,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn microphone_permission() -> MicPermission {
    esperanto_audio::microphone_permission().into()
}

/// Opens System Settings › Privacy & Security › Microphone.
#[tauri::command]
#[specta::specta]
pub fn open_microphone_settings(app: AppHandle) {
    open_external(
        &app,
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        "microphone settings",
    );
}

/// Opens System Settings › Privacy & Security › Screen & System Audio
/// Recording, where system-audio access is granted.
#[tauri::command]
#[specta::specta]
pub fn open_system_audio_settings(app: AppHandle) {
    open_external(
        &app,
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
        "system audio settings",
    );
}
