//! Where inference runs: the local model, or a remote model
//! behind an API gateway with a Bearer token (D12, D21).
//!
//! Profiles (name, URL, model) are stored with the settings; tokens live in
//! the macOS Keychain and are never sent back to the frontend or logged.

mod config;
mod keychain;

use esperanto_inference::{InferenceError, OpenAiCompatibleProvider, OpenAiConfig};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Runtime};

pub use config::RemoteProfile;
use config::{ProvidersConfig, validate_base_url};

use crate::error::{CommandError, CommandResult, ErrorCode};
use crate::server;
use crate::util::random_hex;

/// What the frontend sees of a profile (no token).
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub has_token: bool,
    /// Plain HTTP to a local or private address (allowed, with a warning).
    pub plain_http: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProvidersView {
    /// Active remote profile id; `null` = the local model.
    pub active: Option<String>,
    pub profiles: Vec<ProfileView>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    /// `null` creates a new profile.
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedProfile {
    /// Id of the saved profile (new or existing).
    pub id: String,
    pub view: ProvidersView,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    /// Models the server lists, if it supports listing.
    pub models: Option<Vec<String>>,
    /// Whether the configured model is among them (`null` if unknown).
    pub model_listed: Option<bool>,
    pub latency_ms: f64,
}

/// The provider sessions should use.
pub enum Active {
    Local,
    Remote {
        profile: RemoteProfile,
        provider: OpenAiCompatibleProvider,
    },
}

/// Maps a failed remote request to a code the UI can explain.
pub fn inference_error_code(error: &InferenceError) -> ErrorCode {
    match error {
        InferenceError::Http(_) => ErrorCode::ProviderUnreachable,
        InferenceError::Status {
            status: 401 | 403, ..
        } => ErrorCode::ProviderUnauthorized,
        InferenceError::Status { .. } | InferenceError::InvalidAnswer(_) => {
            ErrorCode::ProviderRejected
        }
    }
}

fn view(config: &ProvidersConfig) -> ProvidersView {
    ProvidersView {
        active: config.active.clone(),
        profiles: config
            .remote
            .iter()
            .map(|p| ProfileView {
                id: p.id.clone(),
                name: p.name.clone(),
                base_url: p.base_url.clone(),
                model: p.model.clone(),
                has_token: keychain::get(&p.id)
                    .inspect_err(|e| log::warn!("can't read the token of {}: {e}", p.id))
                    .ok()
                    .flatten()
                    .is_some(),
                plain_http: validate_base_url(&p.base_url).is_ok_and(|(_, plain)| plain),
            })
            .collect(),
    }
}

fn find<'a>(config: &'a ProvidersConfig, id: &str) -> CommandResult<&'a RemoteProfile> {
    config
        .find(id)
        .ok_or_else(|| CommandError::new(ErrorCode::ProviderNotFound, id))
}

fn client(profile: &RemoteProfile) -> CommandResult<OpenAiCompatibleProvider> {
    let token = keychain::get(&profile.id)?
        .ok_or_else(|| CommandError::new(ErrorCode::ProviderTokenMissing, &profile.id))?;
    let mut config = OpenAiConfig::new(&profile.base_url, &profile.model);
    config.api_token = Some(token);
    OpenAiCompatibleProvider::new(config).map_err(CommandError::internal)
}

/// The provider a new session should use.
pub fn active<R: Runtime>(app: &AppHandle<R>) -> CommandResult<Active> {
    let config = config::load(app)?;
    let Some(id) = config.active.as_deref() else {
        return Ok(Active::Local);
    };
    let profile = find(&config, id)?.clone();
    let provider = client(&profile)?;
    Ok(Active::Remote { profile, provider })
}

// Commands are async: they read the Keychain, which can block (or show a
// system prompt) and must not stall the main thread.

#[tauri::command]
#[specta::specta]
pub async fn list_providers(app: AppHandle) -> CommandResult<ProvidersView> {
    Ok(view(&config::load(&app)?))
}

/// Creates or updates a profile (not its token).
#[tauri::command]
#[specta::specta]
pub async fn save_provider(app: AppHandle, profile: ProfileInput) -> CommandResult<SavedProfile> {
    let name = profile.name.trim();
    let model = profile.model.trim();
    if name.is_empty() || model.is_empty() {
        return Err(CommandError::new(
            ErrorCode::ProviderInvalid,
            "name and model are required",
        ));
    }
    let (base_url, _) = validate_base_url(&profile.base_url)
        .map_err(|code| CommandError::new(code, &profile.base_url))?;

    let mut config = config::load(&app)?;
    let saved = RemoteProfile {
        id: profile.id.clone().unwrap_or_else(|| random_hex(8)),
        name: name.to_owned(),
        base_url,
        model: model.to_owned(),
    };
    let id = saved.id.clone();
    match config.remote.iter_mut().find(|p| p.id == saved.id) {
        Some(existing) => *existing = saved,
        None if profile.id.is_none() => config.remote.push(saved),
        None => return Err(CommandError::new(ErrorCode::ProviderNotFound, &id)),
    }
    config::save(&app, &config)?;
    Ok(SavedProfile {
        id,
        view: view(&config),
    })
}

/// Deletes a profile and its token. If it was active, the local model is used.
#[tauri::command]
#[specta::specta]
pub async fn delete_provider(app: AppHandle, id: String) -> CommandResult<ProvidersView> {
    let mut config = config::load(&app)?;
    find(&config, &id)?;
    keychain::delete(&id)?;
    config.remote.retain(|p| p.id != id);
    if config.active.as_deref() == Some(&id) {
        config.active = None;
    }
    config::save(&app, &config)?;
    Ok(view(&config))
}

/// Chooses where inference runs: a profile id, or `null` for the local model.
/// Applies to the next session. Choosing a remote engine frees the memory
/// of the local model server.
#[tauri::command]
#[specta::specta]
pub async fn set_active_provider(
    app: AppHandle,
    id: Option<String>,
) -> CommandResult<ProvidersView> {
    let mut config = config::load(&app)?;
    if let Some(id) = &id {
        find(&config, id)?;
    }
    config.active = id;
    config::save(&app, &config)?;
    log::info!(
        "inference provider: {}",
        config.active.as_deref().unwrap_or("local")
    );
    if config.active.is_some() {
        server::stop(&app, None).await;
    }
    Ok(view(&config))
}

/// Stores a profile's token in the Keychain. It is never returned.
#[tauri::command]
#[specta::specta]
pub async fn set_provider_token(app: AppHandle, id: String, token: String) -> CommandResult<()> {
    find(&config::load(&app)?, &id)?;
    let token = token.trim();
    if token.is_empty() {
        return Err(CommandError::new(ErrorCode::ProviderTokenMissing, &id));
    }
    keychain::set(&id, token)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_provider_token(app: AppHandle, id: String) -> CommandResult<()> {
    find(&config::load(&app)?, &id)?;
    keychain::delete(&id)
}

/// Checks the gateway: reachable, token accepted, model takes audio.
#[tauri::command]
#[specta::specta]
pub async fn test_provider(app: AppHandle, id: String) -> CommandResult<ConnectionTest> {
    let profile = find(&config::load(&app)?, &id)?.clone();
    let provider = client(&profile)?;
    let report = provider.check().await.map_err(|e| {
        log::warn!("connection test of {} failed: {e}", profile.base_url);
        CommandError::new(inference_error_code(&e), e)
    })?;
    Ok(ConnectionTest {
        model_listed: report
            .models
            .as_ref()
            .map(|models| models.iter().any(|m| m == &profile.model)),
        models: report.models,
        latency_ms: report.audio_latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_request_failures_to_codes() {
        let status = |status| InferenceError::Status {
            status,
            body: String::new(),
        };
        assert_eq!(
            inference_error_code(&status(401)),
            ErrorCode::ProviderUnauthorized
        );
        assert_eq!(
            inference_error_code(&status(403)),
            ErrorCode::ProviderUnauthorized
        );
        assert_eq!(
            inference_error_code(&status(500)),
            ErrorCode::ProviderRejected
        );
    }
}
