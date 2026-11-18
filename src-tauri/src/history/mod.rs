//! Session history: every session is saved
//! on this Mac, unless the user turns history off.
//!
//! - [`model`]: the saved session and its summary;
//! - [`store`]: one JSON file per session in `app_data/history/`;
//! - [`recorder`]: records the running session as phrases arrive;
//! - [`export`]: text and subtitle files.

mod export;
mod model;
mod recorder;
mod store;

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, Runtime, State};

pub use export::{ExportContent, ExportFormat};
pub use model::{HistorySession, HistorySummary};
pub use recorder::Recorder;
use store::HistoryStore;

use crate::error::{CommandError, CommandResult, ErrorCode};

pub struct HistoryState {
    store: Arc<HistoryStore>,
    /// The session being recorded (or the last one).
    current: Mutex<Option<Arc<Mutex<Recorder>>>>,
}

impl HistoryState {
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        Ok(Self {
            store: Arc::new(HistoryStore::new(
                app.path().app_data_dir()?.join("history"),
            )),
            current: Mutex::default(),
        })
    }

    /// Starts recording a session; saved to disk only if `keep` is set.
    pub fn record(
        &self,
        keep: bool,
        model_name: String,
        remote_host: Option<String>,
    ) -> Arc<Mutex<Recorder>> {
        let store = keep.then(|| Arc::clone(&self.store));
        let recorder = Arc::new(Mutex::new(Recorder::new(store, model_name, remote_host)));
        if let Ok(mut current) = self.current.lock() {
            *current = Some(Arc::clone(&recorder));
        }
        recorder
    }

    /// Stops saving the current session, if it is `id` (any, with `None`),
    /// and deletes what was saved of it.
    fn stop_keeping_current(&self, id: Option<&str>) {
        let current = self.current.lock().ok().and_then(|c| c.clone());
        if let Some(recorder) = current
            && let Ok(mut recorder) = recorder.lock()
            && id.is_none_or(|id| recorder.id() == id)
        {
            recorder.stop_keeping();
        }
    }
}

impl HistoryState {
    /// A session by id: the current one from memory (it may not be on
    /// disk, e.g. with history off), any other from disk.
    fn session(&self, id: &str) -> CommandResult<HistorySession> {
        let current = self.current.lock().ok().and_then(|c| c.clone());
        if let Some(recorder) = current
            && let Ok(recorder) = recorder.lock()
            && recorder.id() == id
        {
            return Ok(recorder.session().clone());
        }
        self.store.load(id).map_err(failed)
    }
}

/// Called when the user turns history off: nothing of the running session
/// stays on disk either.
pub fn stop_keeping<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<HistoryState>() {
        state.stop_keeping_current(None);
    }
}

fn failed(error: std::io::Error) -> CommandError {
    let code = if error.kind() == std::io::ErrorKind::NotFound {
        ErrorCode::HistoryNotFound
    } else {
        ErrorCode::HistoryFailed
    };
    CommandError::new(code, error)
}

/// Saved sessions, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_history(app: AppHandle) -> CommandResult<Vec<HistorySummary>> {
    app.state::<HistoryState>().store.list().map_err(failed)
}

#[tauri::command]
#[specta::specta]
pub async fn get_history_session(app: AppHandle, id: String) -> CommandResult<HistorySession> {
    app.state::<HistoryState>().session(&id)
}

#[tauri::command]
#[specta::specta]
pub fn delete_history_session(state: State<'_, HistoryState>, id: String) -> CommandResult<()> {
    state.stop_keeping_current(Some(&id));
    state.store.delete(&id).map_err(failed)
}

/// Deletes every saved session, including the one being recorded.
#[tauri::command]
#[specta::specta]
pub fn clear_history(state: State<'_, HistoryState>) -> CommandResult<()> {
    state.stop_keeping_current(None);
    state.store.clear().map_err(failed)
}

/// A file name without path separators or characters Finder dislikes.
fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').trim();
    if cleaned.is_empty() {
        "Esperanto".to_owned()
    } else {
        cleaned.chars().take(100).collect()
    }
}

/// Exports a session (saved, or the current one) to a file the user picks
/// in the save panel. `fileName` (without extension) is suggested there and
/// titles Markdown files. Returns the saved path, or `None` if cancelled.
#[tauri::command]
#[specta::specta]
pub async fn export_session(
    app: AppHandle,
    id: String,
    format: ExportFormat,
    content: ExportContent,
    file_name: String,
) -> CommandResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    let session = app.state::<HistoryState>().session(&id)?;
    let name = safe_file_name(&file_name);
    let text = export::export(&session, format, content, &name);
    let extension = format.extension();
    let dialog = app
        .dialog()
        .file()
        .set_file_name(format!("{name}.{extension}"))
        .add_filter(extension.to_uppercase(), &[extension]);
    // The panel is modal; wait for it off the async runtime.
    let Some(picked) = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file())
        .await
        .map_err(CommandError::internal)?
    else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| CommandError::new(ErrorCode::ExportFailed, e))?;
    std::fs::write(&path, text).map_err(|e| CommandError::new(ErrorCode::ExportFailed, e))?;
    log::info!("exported session {id} as {extension}");
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::safe_file_name;

    #[test]
    fn file_names_cannot_escape_the_chosen_folder() {
        assert_eq!(
            safe_file_name("Esperanto 18/11/2026, 19:20"),
            "Esperanto 18-11-2026, 19-20"
        );
        assert_eq!(safe_file_name("../../etc"), "-..-etc");
        assert_eq!(safe_file_name("   "), "Esperanto");
    }
}
