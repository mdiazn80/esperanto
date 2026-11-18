//! Microphone permission. On macOS a denied permission doesn't make the
//! stream fail: it just delivers silence, so it must be checked explicitly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophonePermission {
    Granted,
    Denied,
    /// The user hasn't been asked yet.
    NotDetermined,
}

#[cfg(target_os = "macos")]
mod platform {
    #![allow(unsafe_code)]

    use std::sync::mpsc;
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    use super::MicrophonePermission;

    pub fn status() -> MicrophonePermission {
        // SAFETY: AVMediaTypeAudio is an immutable constant provided by AVFoundation.
        let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
            return MicrophonePermission::Denied;
        };
        // SAFETY: plain class method call with a valid media type (audio).
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
        match status {
            AVAuthorizationStatus::Authorized => MicrophonePermission::Granted,
            AVAuthorizationStatus::NotDetermined => MicrophonePermission::NotDetermined,
            _ => MicrophonePermission::Denied,
        }
    }

    pub fn request() -> bool {
        // SAFETY: as above.
        let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
            return false;
        };
        let (sender, receiver) = mpsc::channel();
        let handler = RcBlock::new(move |granted: Bool| {
            let _ = sender.send(granted.as_bool());
        });
        // SAFETY: valid media type; the block is retained by AVFoundation until it runs.
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
        }
        // The system dialog waits for the user; give up after two minutes.
        receiver
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or(false)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::MicrophonePermission;

    pub fn status() -> MicrophonePermission {
        MicrophonePermission::Granted
    }

    pub fn request() -> bool {
        true
    }
}

pub fn microphone_permission() -> MicrophonePermission {
    platform::status()
}

/// Shows the system permission dialog if needed and waits for the answer.
/// Blocking: call it off the main thread.
pub fn request_microphone_permission() -> bool {
    match platform::status() {
        MicrophonePermission::Granted => true,
        MicrophonePermission::Denied => false,
        MicrophonePermission::NotDetermined => platform::request(),
    }
}
