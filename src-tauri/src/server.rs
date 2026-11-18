//! Runs the local inference server (`llama-server` from llama.cpp) for
//! the installed model.
//!
//! The server listens on `127.0.0.1` on a random port and requires a random
//! per-launch API key (passed through the environment, so it doesn't show in
//! `ps`), so other local processes can't use it. It is started on demand,
//! kept running between sessions (loading the model takes seconds), and
//! stopped when the model is deleted, a remote engine is chosen, or the app
//! exits.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use esperanto_inference::{OpenAiCompatibleProvider, OpenAiConfig};
use esperanto_models::{FileRole, ModelEntry, ModelStore};
use tauri::{AppHandle, Manager};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::error::{CommandError, CommandResult, ErrorCode};
use crate::util::random_hex;

const BINARY: &str = "llama-server";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(180);
/// Attempts to start the server; a second one covers losing the random port
/// to another process between choosing it and the server binding it.
const START_ATTEMPTS: usize = 2;
/// Where Homebrew installs binaries. Apps opened from Finder don't get the
/// shell's `PATH`, so these are searched explicitly.
const EXTRA_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];

struct Running {
    child: Child,
    model_id: String,
    base_url: String,
    api_key: String,
}

#[derive(Default)]
pub struct ServerState {
    running: Mutex<Option<Running>>,
    /// PID of the current server process, also while it is still loading.
    /// Kept outside the async lock so the app can always kill it on exit.
    pid: AtomicU32,
}

/// Finds `llama-server`: `ESPERANTO_LLAMA_SERVER`, next to the app's
/// executable (bundled sidecar), then `PATH` and Homebrew's folders.
fn find_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ESPERANTO_LLAMA_SERVER").map(PathBuf::from) {
        return path.is_file().then_some(path);
    }
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(BINARY)));
    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    bundled
        .into_iter()
        .chain(path_dirs.into_iter().map(|dir| dir.join(BINARY)))
        .chain(EXTRA_DIRS.iter().map(|dir| Path::new(dir).join(BINARY)))
        .find(|path| path.is_file())
}

fn free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn failed(detail: impl ToString) -> CommandError {
    CommandError::new(ErrorCode::LlamaServerFailed, detail)
}

/// Whether a start failed because the process exited early (e.g. its port
/// was taken), which is worth one retry.
enum StartError {
    ExitedEarly(CommandError),
    Fatal(CommandError),
}

async fn start_once(
    app: &AppHandle,
    store: &ModelStore,
    model: &ModelEntry,
    binary: &Path,
) -> Result<Running, StartError> {
    let fatal = |e: CommandError| StartError::Fatal(e);
    let weights = model
        .file(FileRole::Model)
        .map(|f| store.file_path(model, f))
        .ok_or_else(|| fatal(CommandError::new(ErrorCode::ModelNotInstalled, &model.id)))?;
    let port = free_port().map_err(|e| fatal(failed(e)))?;
    let api_key = random_hex(32);
    let log_dir = app.path().app_log_dir().map_err(|e| fatal(failed(e)))?;
    std::fs::create_dir_all(&log_dir).map_err(|e| fatal(failed(e)))?;
    let log_file = log_dir.join("llama-server.log");

    let mut command = Command::new(binary);
    command
        .arg("--model")
        .arg(&weights)
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        // Read by llama-server instead of `--api-key`, which `ps` would show.
        .env("LLAMA_API_KEY", &api_key)
        // All layers on the GPU (Metal); 2 slots so a partial pass doesn't
        // block a final one; 4096 tokens each (10 s of audio is ~125).
        .args([
            "--n-gpu-layers",
            "99",
            "--parallel",
            "2",
            "--ctx-size",
            "8192",
        ])
        // No prompt cache in RAM (8 GB by default): every request carries
        // different audio, so it only grows memory (9.2 GB peak with it).
        .args(["--cache-ram", "0"])
        .arg("--no-webui")
        .arg("--log-file")
        .arg(&log_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(projector) = model.file(FileRole::AudioProjector) {
        command
            .arg("--mmproj")
            .arg(store.file_path(model, projector));
    }
    if let Some(template) = &model.chat_template {
        command.args(["--no-jinja", "--chat-template", template]);
    }

    log::info!(
        "starting {} for {} on port {port}",
        binary.display(),
        model.id
    );
    let mut child = command.spawn().map_err(|e| fatal(failed(e)))?;
    let state = app.state::<ServerState>();
    state.pid.store(child.id().unwrap_or(0), Ordering::Relaxed);
    let base_url = format!("http://127.0.0.1:{port}");

    // Wait until the model is loaded.
    let http = reqwest::Client::new();
    let started = Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            state.pid.store(0, Ordering::Relaxed);
            return Err(StartError::ExitedEarly(failed(format!(
                "llama-server exited ({status}); see {}",
                log_file.display()
            ))));
        }
        let healthy = http
            .get(format!("{base_url}/health"))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        if healthy {
            log::info!(
                "llama-server ready in {:.1}s",
                started.elapsed().as_secs_f64()
            );
            return Ok(Running {
                child,
                model_id: model.id.clone(),
                base_url,
                api_key,
            });
        }
        if started.elapsed() > STARTUP_TIMEOUT {
            let _ = child.kill().await;
            state.pid.store(0, Ordering::Relaxed);
            return Err(fatal(failed("llama-server didn't become ready in time")));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn start(app: &AppHandle, store: &ModelStore, model: &ModelEntry) -> CommandResult<Running> {
    let binary = find_binary().ok_or_else(|| {
        CommandError::new(ErrorCode::LlamaServerNotFound, "llama-server not found")
    })?;
    let mut last = None;
    for _ in 0..START_ATTEMPTS {
        match start_once(app, store, model, &binary).await {
            Ok(running) => return Ok(running),
            Err(StartError::Fatal(error)) => return Err(error),
            Err(StartError::ExitedEarly(error)) => {
                log::warn!("{error}; retrying");
                last = Some(error);
            }
        }
    }
    Err(last.unwrap_or_else(|| failed("llama-server could not start")))
}

/// Returns a provider for `model`, starting (or restarting) the server if
/// needed.
pub async fn provider(
    app: &AppHandle,
    store: &ModelStore,
    model: &ModelEntry,
) -> CommandResult<OpenAiCompatibleProvider> {
    let state = app.state::<ServerState>();
    let mut running = state.running.lock().await;

    let reusable = running.as_mut().is_some_and(|server| {
        server.model_id == model.id && matches!(server.child.try_wait(), Ok(None))
    });
    if !reusable {
        if let Some(mut old) = running.take() {
            let _ = old.child.kill().await;
        }
        *running = Some(start(app, store, model).await?);
    }

    let server = running.as_ref().expect("server was just started");
    let mut config = OpenAiConfig::new(format!("{}/v1", server.base_url), &model.id);
    config.api_token = Some(server.api_key.clone());
    OpenAiCompatibleProvider::new(config).map_err(failed)
}

/// Stops the server if it runs `model_id` (or any model, with `None`), to
/// free its memory.
pub async fn stop(app: &AppHandle, model_id: Option<&str>) {
    let state = app.state::<ServerState>();
    let mut running = state.running.lock().await;
    let matches = running
        .as_ref()
        .is_some_and(|server| model_id.is_none_or(|id| server.model_id == id));
    if matches && let Some(mut server) = running.take() {
        let _ = server.child.kill().await;
        state.pid.store(0, Ordering::Relaxed);
        log::info!("llama-server stopped");
    }
}

/// Kills the server on app exit, even if it is still loading (its handle is
/// then held by the starting task, so it is killed by PID).
pub fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<ServerState>() else {
        return;
    };
    let pid = state.pid.swap(0, Ordering::Relaxed);
    if pid != 0 {
        let _ = std::process::Command::new("/bin/kill")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_port_is_bindable() {
        let port = free_port().unwrap();
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}
