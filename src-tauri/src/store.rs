//! Values persisted in `settings.json` with `tauri-plugin-store`.
//!
//! Only Rust touches the store; the frontend goes through typed commands
//! and has no store permissions.

use serde::Serialize;
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::error::{CommandError, CommandResult, ErrorCode};

const STORE_FILE: &str = "settings.json";

/// The stored value under `key`, as JSON (`None` if absent).
pub fn get<R: Runtime>(app: &AppHandle<R>, key: &str) -> CommandResult<Option<serde_json::Value>> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| CommandError::new(ErrorCode::SettingsLoadFailed, e))?;
    Ok(store.get(key))
}

/// Parses the stored value under `key`. Absent → `None`; present but
/// invalid → an error (callers decide whether defaults are acceptable).
pub fn get_parsed<R: Runtime, T: DeserializeOwned>(
    app: &AppHandle<R>,
    key: &str,
) -> CommandResult<Option<T>> {
    get(app, key)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| CommandError::new(ErrorCode::SettingsLoadFailed, format!("{key}: {e}")))
}

pub fn put<R: Runtime, T: Serialize>(
    app: &AppHandle<R>,
    key: &str,
    value: &T,
) -> CommandResult<()> {
    let save_error =
        |e: &dyn std::fmt::Display| CommandError::new(ErrorCode::SettingsSaveFailed, e);
    let store = app.store(STORE_FILE).map_err(|e| save_error(&e))?;
    let value = serde_json::to_value(value).map_err(|e| save_error(&e))?;
    store.set(key, value);
    store.save().map_err(|e| save_error(&e))
}
