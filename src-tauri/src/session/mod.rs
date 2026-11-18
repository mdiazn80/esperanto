//! A listening session: audio → VAD → speech segments
//! → model → live transcript and translation, streamed to the frontend
//! through a Tauri `Channel`.
//!
//! - [`handler`] runs on the capture's DSP thread;
//! - [`workers`] transcribe partial snapshots and finished phrases;
//! - [`engine`] is the model behind them (local server or remote gateway);
//! - [`events`] are the messages the frontend receives.

mod engine;
mod events;
mod handler;
mod speakers;
mod workers;

use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};

use esperanto_audio::{AudioError, Capture, REPLAY_PREFIX, SYSTEM_AUDIO};
use esperanto_core::Language;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::{mpsc, watch};

use engine::Engine;
pub use events::{SessionEvent, SessionStarted};
use handler::Handler;
use workers::Shared;

use crate::error::{CommandError, CommandResult, ErrorCode};
use crate::history::HistoryState;
use crate::models::ModelsState;
use crate::{providers, server, settings};

/// Spoken and target languages, readable by the pipeline while it runs so
/// they can change mid-session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Languages {
    pub source: Language,
    pub target: Option<Language>,
}

struct Running {
    capture: Capture,
    languages: Arc<RwLock<Languages>>,
}

#[derive(Default)]
pub struct SessionState(Mutex<Option<Running>>);

impl SessionState {
    /// Applies new languages to a running session, from the next segment.
    pub fn set_languages(&self, languages: Languages) {
        if let Ok(guard) = self.0.lock()
            && let Some(running) = guard.as_ref()
            && let Ok(mut current) = running.languages.write()
        {
            *current = languages;
        }
    }

    pub fn is_running(&self) -> bool {
        self.0.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Stops the capture, if running. Speech in progress becomes a final
    /// segment, and segments already captured are still transcribed.
    /// Blocking (joins the capture threads).
    pub fn stop(&self) {
        let running = self.0.lock().ok().and_then(|mut slot| slot.take());
        if let Some(running) = running {
            let overruns = running.capture.overruns();
            if overruns > 0 {
                log::warn!("audio overruns during session: {overruns} samples");
            }
            running.capture.stop();
            log::info!("session stopped");
        }
    }
}

/// Stops any running session, e.g. when the interface reloads and can no
/// longer receive its events or stop it.
pub fn stop_running<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<SessionState>() {
        state.stop();
    }
}

fn already_running() -> CommandError {
    CommandError::new(
        ErrorCode::SessionAlreadyRunning,
        "a session is already running",
    )
}

/// The model the session will use, starting the local server if needed
/// (can take a few seconds the first time).
async fn engine(app: &AppHandle) -> CommandResult<(Engine, String, Option<String>)> {
    match providers::active(app)? {
        providers::Active::Local => {
            let models = app.state::<ModelsState>();
            let model = models.active_model().cloned().ok_or_else(|| {
                CommandError::new(ErrorCode::ModelNotInstalled, "no model installed")
            })?;
            let provider = server::provider(app, models.store(), &model).await?;
            let name = model.name.clone();
            Ok((Engine::local(app.clone(), model, provider), name, None))
        }
        providers::Active::Remote { profile, provider } => Ok((
            Engine::remote(app.clone(), provider),
            profile.model.clone(),
            profile.host(),
        )),
    }
}

/// Starts listening: checks the microphone permission (not needed for
/// system audio or replays), prepares the model, then starts the capture.
#[tauri::command]
#[specta::specta]
pub async fn start_session(
    app: AppHandle,
    device_id: Option<String>,
    events: Channel<SessionEvent>,
) -> CommandResult<SessionStarted> {
    let state = app.state::<SessionState>();
    if state.is_running() {
        return Err(already_running());
    }
    // Replaying a file instead of a microphone is a debugging aid only.
    let replay = device_id
        .as_deref()
        .is_some_and(|id| id.starts_with(REPLAY_PREFIX));
    if replay && !cfg!(debug_assertions) {
        return Err(CommandError::new(
            ErrorCode::InputDeviceNotFound,
            "file replay is only available in debug builds",
        ));
    }
    // System audio has its own permission, asked by macOS when the tap starts.
    let system = device_id.as_deref() == Some(SYSTEM_AUDIO);
    if !replay && !system {
        // Shows the system dialog the first time; blocks until the user answers.
        let granted =
            tauri::async_runtime::spawn_blocking(esperanto_audio::request_microphone_permission)
                .await
                .unwrap_or(false);
        if !granted {
            return Err(AudioError::PermissionDenied.into());
        }
    }

    let (engine, model_name, remote_host) = engine(&app).await?;
    let settings = settings::load(&app)?;
    let languages = Arc::new(RwLock::new(settings.languages()));
    let speakers = load_speakers(&app, settings.identify_speakers).await;
    let recorder = app.state::<HistoryState>().record(
        settings.keep_history,
        model_name.clone(),
        remote_host.clone(),
    );
    let shared = Shared {
        engine: Arc::new(engine),
        languages: Arc::clone(&languages),
        finished_up_to: Arc::new(AtomicU32::new(0)),
        queued: Arc::new(AtomicUsize::new(0)),
        channel: events,
        recorder: Arc::clone(&recorder),
        speakers: speakers.clone(),
    };
    let (partials_tx, partials_rx) = watch::channel(None);
    let (finals_tx, finals_rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(workers::partial_worker(shared.clone(), partials_rx));
    tauri::async_runtime::spawn(workers::final_worker(shared.clone(), finals_rx));

    // When the capture stops, the handler (and with it the job senders) is
    // dropped: the workers finish pending segments and exit.
    let handler = Handler::new(shared, partials_tx, finals_tx, system);
    // Opening a device (or creating the system-audio tap) can take a while.
    let (capture, used_default_device) = tauri::async_runtime::spawn_blocking(move || {
        start_capture_with_fallback(device_id.as_deref(), handler)
    })
    .await
    .map_err(CommandError::internal)??;

    let history_id = {
        let mut recorder = recorder.lock().map_err(CommandError::internal)?;
        recorder.set_device(capture.device_name());
        recorder.id().to_owned()
    };
    let started = SessionStarted {
        device_name: capture.device_name().to_owned(),
        model_name,
        remote_host,
        used_default_device,
        history_id,
        speaker_labels: speakers.is_some(),
    };
    log::info!(
        "session started: {:?}, model {} ({})",
        started.device_name,
        started.model_name,
        started.remote_host.as_deref().unwrap_or("local")
    );

    let mut slot = state.0.lock().map_err(CommandError::internal)?;
    if slot.is_some() {
        capture.stop();
        return Err(already_running());
    }
    *slot = Some(Running { capture, languages });
    Ok(started)
}

/// The speaker model, when speaker labels are on and it is installed. A
/// model that fails to load only turns the labels off.
async fn load_speakers(app: &AppHandle, wanted: bool) -> Option<Arc<speakers::Speakers>> {
    if !wanted {
        return None;
    }
    let Some(path) = app.state::<ModelsState>().speaker_model() else {
        log::info!("speaker labels are on, but the speaker model isn't installed");
        return None;
    };
    match tauri::async_runtime::spawn_blocking(move || speakers::Speakers::load(&path)).await {
        Ok(Ok(speakers)) => Some(Arc::new(speakers)),
        Ok(Err(error)) => {
            log::error!("{error}");
            None
        }
        Err(error) => {
            log::error!("could not load the speaker model: {error}");
            None
        }
    }
}

/// Starts the chosen microphone, or the system default if it isn't
/// connected (e.g. a headset that was unplugged).
fn start_capture_with_fallback(
    device_id: Option<&str>,
    handler: Handler,
) -> CommandResult<(Capture, bool)> {
    let wanted = device_id.filter(|id| !id.starts_with(REPLAY_PREFIX) && *id != SYSTEM_AUDIO);
    if let Some(id) = wanted
        && !esperanto_audio::list_input_devices()
            .map(|devices| devices.iter().any(|d| d.id == id))
            .unwrap_or(true)
    {
        log::warn!("microphone {id} not connected; using the system default");
        return Ok((esperanto_audio::start_capture(None, handler)?, true));
    }
    Ok((esperanto_audio::start_capture(device_id, handler)?, false))
}

/// Stops listening. Async: joining the capture threads must not block the
/// main thread.
#[tauri::command]
#[specta::specta]
pub async fn stop_session(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || stop_running(&app)).await;
}
