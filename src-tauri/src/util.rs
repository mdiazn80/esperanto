//! Small helpers shared by several modules.

use tauri::{AppHandle, Runtime};

/// `bytes` random bytes from the OS generator, hex-encoded.
pub fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).expect("the OS random generator is unavailable");
    hex::encode(buffer)
}

/// Opens a URL with the system (browser, System Settings…). Failures are
/// logged: there is nothing else the user could do about them.
pub fn open_external<R: Runtime>(app: &AppHandle<R>, url: &str, what: &str) {
    use tauri_plugin_opener::OpenerExt;
    if let Err(error) = app.opener().open_url(url, None::<&str>) {
        log::warn!("could not open {what}: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_hex_has_the_requested_length_and_varies() {
        let (a, b) = (random_hex(32), random_hex(32));
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(random_hex(8).len(), 16);
    }
}
