use cpal::traits::{DeviceTrait, HostTrait};

use crate::AudioError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDevice {
    /// Stable identifier (survives restarts and reconnections where possible).
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<InputDevice>, AudioError> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());
    let devices = host
        .input_devices()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok(devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            Some(InputDevice {
                is_default: default_id.as_deref() == Some(id.as_str()),
                name: device.to_string(),
                id,
            })
        })
        .collect())
}

pub(crate) fn find_input_device(id: Option<&str>) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    match id {
        None => host.default_input_device().ok_or(AudioError::NoInputDevice),
        Some(id) => {
            let parsed = id
                .parse()
                .map_err(|_| AudioError::DeviceNotFound(id.to_owned()))?;
            host.device_by_id(&parsed)
                .ok_or_else(|| AudioError::DeviceNotFound(id.to_owned()))
        }
    }
}
