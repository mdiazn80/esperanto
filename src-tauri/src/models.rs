//! Model management commands: the catalog with each
//! model's install status, downloads with progress, cancel and delete.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use esperanto_core::Language;
use esperanto_models::{Catalog, FileRole, ModelEntry, ModelStatus, ModelStore, Runtime};
use serde::Serialize;
use specta::Type;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult, ErrorCode};
use crate::server;
use crate::util::open_external;

pub struct ModelsState {
    catalog: Catalog,
    store: ModelStore,
    http: reqwest::Client,
    /// Cancel flags of downloads in progress, by model id.
    downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ModelsState {
    pub fn new(app: &AppHandle) -> tauri::Result<Self> {
        let root = app.path().app_data_dir()?.join("models");
        Ok(Self {
            catalog: Catalog::bundled(),
            store: ModelStore::new(root),
            http: reqwest::Client::builder()
                .user_agent(concat!("Esperanto/", env!("CARGO_PKG_VERSION")))
                // A stalled connection fails (and frees the download slot)
                // instead of hanging forever; resuming picks it up again.
                .connect_timeout(Duration::from_secs(15))
                .read_timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| tauri::Error::Anyhow(e.into()))?,
            downloads: Mutex::default(),
        })
    }

    fn entry(&self, id: &str) -> CommandResult<&ModelEntry> {
        self.catalog
            .get(id)
            .ok_or_else(|| CommandError::new(ErrorCode::ModelNotFound, id))
    }

    pub fn store(&self) -> &ModelStore {
        &self.store
    }

    /// The installed speech model to use, preferring the recommended one.
    pub fn active_model(&self) -> Option<&ModelEntry> {
        let mut installed: Vec<_> = self
            .catalog
            .models
            .iter()
            .filter(|m| m.runtime == Runtime::LlamaServer)
            .filter(|m| !self.is_downloading(&m.id))
            .filter(|m| self.store.status(m) == ModelStatus::Installed)
            .collect();
        installed.sort_by_key(|m| !m.recommended);
        installed.into_iter().next()
    }

    /// Path of the installed speaker-embedding model (speaker labels).
    pub fn speaker_model(&self) -> Option<std::path::PathBuf> {
        let model = self
            .catalog
            .models
            .iter()
            .filter(|m| m.runtime == Runtime::SherpaOnnx && !self.is_downloading(&m.id))
            .find(|m| self.store.status(m) == ModelStatus::Installed)?;
        let file = model.file(FileRole::SpeakerEmbedding)?;
        Some(self.store.file_path(model, file))
    }

    fn is_downloading(&self, id: &str) -> bool {
        self.downloads
            .lock()
            .map(|d| d.contains_key(id))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InstallState {
    NotInstalled,
    /// A paused or interrupted download that can be resumed.
    Partial {
        downloaded_bytes: f64,
    },
    Downloading,
    Installed,
}

/// What a model is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModelKind {
    /// Transcription and translation.
    Speech,
    /// Voice prints for speaker labels.
    Speakers,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub kind: ModelKind,
    pub name: String,
    pub description: String,
    pub languages: Vec<Language>,
    /// Bytes (as a JS number; models are far below 2^53).
    pub size_bytes: f64,
    pub min_ram_gb: u32,
    pub recommended: bool,
    pub license_name: String,
    pub status: InstallState,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelsOverview {
    pub models: Vec<ModelInfo>,
    pub disk_usage_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded_bytes: f64,
    pub total_bytes: f64,
}

fn info(state: &ModelsState, model: &ModelEntry) -> ModelInfo {
    let status = if state.is_downloading(&model.id) {
        InstallState::Downloading
    } else {
        match state.store.status(model) {
            ModelStatus::Installed => InstallState::Installed,
            ModelStatus::Partial { downloaded } => InstallState::Partial {
                downloaded_bytes: downloaded as f64,
            },
            ModelStatus::NotInstalled => InstallState::NotInstalled,
        }
    };
    ModelInfo {
        id: model.id.clone(),
        kind: match model.runtime {
            Runtime::LlamaServer => ModelKind::Speech,
            Runtime::SherpaOnnx => ModelKind::Speakers,
        },
        name: model.name.clone(),
        description: model.description.clone(),
        languages: model.languages.clone(),
        size_bytes: model.total_size() as f64,
        min_ram_gb: model.min_ram_gb,
        recommended: model.recommended,
        license_name: model.license.name.clone(),
        status,
    }
}

/// The catalog with each model's status. Async: it walks the models folder.
#[tauri::command]
#[specta::specta]
pub async fn list_models(app: AppHandle) -> ModelsOverview {
    let state = app.state::<ModelsState>();
    ModelsOverview {
        models: state
            .catalog
            .models
            .iter()
            .map(|m| info(&state, m))
            .collect(),
        disk_usage_bytes: state.store.disk_usage() as f64,
    }
}

/// Downloads (or resumes) a model. Resolves when it is installed.
#[tauri::command]
#[specta::specta]
pub async fn download_model(
    app: AppHandle,
    id: String,
    on_progress: Channel<DownloadProgress>,
) -> CommandResult<()> {
    let state = app.state::<ModelsState>();
    let model = state.entry(&id)?.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut downloads = state.downloads.lock().map_err(CommandError::internal)?;
        if downloads.contains_key(&id) {
            return Err(CommandError::new(ErrorCode::DownloadInProgress, &id));
        }
        downloads.insert(id.clone(), Arc::clone(&cancel));
    }

    let result = esperanto_models::download(&state.http, &state.store, &model, &cancel, |p| {
        let _ = on_progress.send(DownloadProgress {
            downloaded_bytes: p.downloaded as f64,
            total_bytes: p.total as f64,
        });
    })
    .await;

    if let Ok(mut downloads) = state.downloads.lock() {
        downloads.remove(&id);
    }
    result.map_err(|error| {
        let error = CommandError::from(error);
        if error.code != ErrorCode::DownloadCancelled {
            log::error!("download of {id} failed: {error}");
        }
        error
    })
}

/// Stops a download; what was downloaded is kept so it can resume. Takes
/// effect when the next chunk arrives (at most the read timeout).
#[tauri::command]
#[specta::specta]
pub fn cancel_download(state: State<'_, ModelsState>, id: String) {
    if let Some(cancel) = state
        .downloads
        .lock()
        .ok()
        .and_then(|d| d.get(&id).cloned())
    {
        cancel.store(true, Ordering::Relaxed);
    }
}

/// Deletes a model's files (complete or partial), stopping the local
/// server first if it is running that model.
#[tauri::command]
#[specta::specta]
pub async fn delete_model(app: AppHandle, id: String) -> CommandResult<()> {
    let state = app.state::<ModelsState>();
    let model = state.entry(&id)?.clone();
    if state.is_downloading(&id) {
        return Err(CommandError::new(ErrorCode::DownloadInProgress, &id));
    }
    // A session may be using the model through the local server.
    if app.state::<crate::session::SessionState>().is_running() {
        return Err(CommandError::new(ErrorCode::SessionAlreadyRunning, &id));
    }
    server::stop(&app, Some(&id)).await;
    state
        .store
        .delete(&model)
        .map_err(|e| CommandError::new(ErrorCode::ModelDeleteFailed, e))
}

/// Opens the model's license page. Only catalog URLs can be opened.
#[tauri::command]
#[specta::specta]
pub fn open_model_license(
    app: AppHandle,
    state: State<'_, ModelsState>,
    id: String,
) -> CommandResult<()> {
    open_external(&app, &state.entry(&id)?.license.url, "license page");
    Ok(())
}
