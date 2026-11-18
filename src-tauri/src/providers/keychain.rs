//! API tokens of remote profiles, stored in the macOS Keychain (D21).

use crate::error::{CommandError, CommandResult, ErrorCode};

const SERVICE: &str = "com.mdiazn80.esperanto";

fn entry(id: &str) -> CommandResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("provider:{id}"))
        .map_err(|e| CommandError::new(ErrorCode::KeychainFailed, e))
}

pub fn get(id: &str) -> CommandResult<Option<String>> {
    match entry(id)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(CommandError::new(ErrorCode::KeychainFailed, e)),
    }
}

pub fn set(id: &str, token: &str) -> CommandResult<()> {
    entry(id)?
        .set_password(token)
        .map_err(|e| CommandError::new(ErrorCode::KeychainFailed, e))
}

pub fn delete(id: &str) -> CommandResult<()> {
    match entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(CommandError::new(ErrorCode::KeychainFailed, e)),
    }
}
