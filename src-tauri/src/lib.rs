mod audio;
mod error;
mod history;
mod i18n;
mod models;
mod providers;
mod server;
mod session;
mod settings;
mod store;
mod tray;
mod util;
mod window;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands};

/// Absolute path of the generated TypeScript bindings. Resolved at compile
/// time so it doesn't depend on the working directory the app is launched from.
const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        settings::get_settings,
        settings::set_settings,
        audio::list_audio_inputs,
        audio::microphone_permission,
        audio::open_microphone_settings,
        audio::open_system_audio_settings,
        session::start_session,
        session::stop_session,
        history::list_history,
        history::get_history_session,
        history::delete_history_session,
        history::clear_history,
        history::export_session,
        models::list_models,
        models::download_model,
        models::cancel_download,
        models::delete_model,
        models::open_model_license,
        providers::list_providers,
        providers::save_provider,
        providers::delete_provider,
        providers::set_active_provider,
        providers::set_provider_token,
        providers::clear_provider_token,
        providers::test_provider,
    ])
}

fn export_bindings(
    builder: &Builder<tauri::Wry>,
) -> Result<(), impl std::fmt::Display + std::fmt::Debug> {
    builder.export(
        specta_typescript::Typescript::default().header("// @ts-nocheck\n"),
        BINDINGS_PATH,
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Log every panic with its thread, so failures in background threads
    // (e.g. audio capture) can be diagnosed from the log file.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        log::error!(
            "panic in thread {:?}: {info}",
            thread.name().unwrap_or("unnamed")
        );
        default_hook(info);
    }));

    let builder = specta_builder();

    // Keep the bindings fresh during development. Never fatal: a debug build
    // may run where the source tree isn't writable.
    #[cfg(debug_assertions)]
    if let Err(error) = export_bindings(&builder) {
        log::warn!("could not export TypeScript bindings: {error}");
    }

    tauri::Builder::default()
        // Must be registered first: a second launch exits here and focuses the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::focus_main(app);
        }))
        // ~/Library/Logs/com.mdiazn80.esperanto/esperanto.log (+ the terminal in development).
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("esperanto".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                // Dependencies are noisy at debug level.
                .level_for("reqwest", log::LevelFilter::Info)
                .level_for("hyper_util", log::LevelFilter::Info)
                .max_file_size(5_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(window::state_flags())
                // Restored in `setup` by window::restore_and_show, fitted to the current monitors.
                .skip_initial_state(window::MAIN)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(session::SessionState::default())
        .manage(server::ServerState::default())
        .invoke_handler(builder.invoke_handler())
        .on_window_event(window::on_window_event)
        .on_web_content_process_terminate(window::on_web_content_terminated)
        .setup(move |app| {
            builder.mount_events(app);

            let handle = app.handle();
            app.manage(models::ModelsState::new(handle)?);
            app.manage(history::HistoryState::new(handle)?);
            let ui_language = settings::load(handle)
                .map(|s| s.ui_language)
                .unwrap_or_default();
            tray::create(handle, i18n::resolve(ui_language))?;

            if let Err(error) = window::restore_and_show(handle) {
                log::warn!("could not restore window state: {error}");
            }
            debug_assert!(app.get_webview_window(window::MAIN).is_some());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                server::shutdown(app);
            }
            // Clicking the Dock icon while the window is hidden brings it back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = event
            {
                window::focus_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(test)]
mod tests {
    /// Regenerates `src/bindings.ts`.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::specta_builder())
            .expect("failed to export TypeScript bindings");
    }
}
