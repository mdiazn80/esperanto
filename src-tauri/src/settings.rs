//! User settings and their commands. Stored in `settings.json` (see
//! [`crate::store`]).

use esperanto_core::{Language, SourceLanguage, TargetLanguage};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, Runtime};

use crate::error::CommandResult;
use crate::session::{Languages, SessionState};
use crate::{history, i18n, store, tray};

const SETTINGS_KEY: &str = "settings";

/// Language of the app's interface (independent of transcription languages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    /// Follow the operating system's language.
    #[default]
    System,
    En,
    Es,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system's appearance.
    #[default]
    System,
    Light,
    Dark,
}

/// What a session listens to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AudioSource {
    #[default]
    Microphone,
    /// What the Mac is playing (Core Audio process tap, macOS 14.2+).
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub ui_language: UiLanguage,
    pub theme: Theme,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub audio_source: AudioSource,
    /// Input device id; `None` follows the system default.
    pub input_device: Option<String>,
    /// Save each session on this Mac for the History view (D22).
    pub keep_history: bool,
    /// Label each phrase with its speaker (needs the speaker model, D24).
    pub identify_speakers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_language: UiLanguage::default(),
            theme: Theme::default(),
            source_language: SourceLanguage::default(),
            target_language: TargetLanguage::default(),
            audio_source: AudioSource::default(),
            input_device: None,
            keep_history: true,
            identify_speakers: false,
        }
    }
}

impl Settings {
    /// The spoken language must be chosen: automatic detection isn't
    /// reliable enough yet. `Auto` becomes `fallback`.
    pub fn require_fixed_source(&mut self, fallback: Language) {
        if self.source_language == SourceLanguage::Auto {
            self.source_language = SourceLanguage::Fixed(fallback);
        }
    }

    /// Parses stored settings. Unknown or invalid values fall back to the
    /// defaults instead of failing, so a bad file never blocks the app.
    fn from_stored(value: Option<serde_json::Value>) -> Self {
        value
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }
}

impl Settings {
    /// The languages a session uses.
    pub fn languages(&self) -> Languages {
        Languages {
            source: match self.source_language {
                SourceLanguage::Fixed(lang) => lang,
                // Unreachable after `normalize`, which `load` and `set_settings` apply.
                SourceLanguage::Auto => i18n::resolve(self.ui_language).language(),
            },
            target: self.target_language.language(),
        }
    }

    fn normalize(&mut self) {
        self.require_fixed_source(i18n::resolve(self.ui_language).language());
    }
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> CommandResult<Settings> {
    let mut settings = Settings::from_stored(store::get(app, SETTINGS_KEY)?);
    settings.normalize();
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(app: AppHandle) -> CommandResult<Settings> {
    load(&app)
}

/// Saves the settings and applies them where the webview can't: a running
/// session (languages, from the next phrase; history), and the tray (its
/// labels).
#[tauri::command]
#[specta::specta]
pub fn set_settings(app: AppHandle, mut settings: Settings) -> CommandResult<Settings> {
    settings.normalize();
    store::put(&app, SETTINGS_KEY, &settings)?;
    app.state::<SessionState>()
        .set_languages(settings.languages());
    if !settings.keep_history {
        history::stop_keeping(&app);
    }
    if let Err(error) = tray::set_locale(&app, i18n::resolve(settings.ui_language)) {
        log::warn!("could not relabel tray: {error}");
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn automatic_source_language_becomes_a_fixed_one() {
        let mut settings = Settings::default();
        settings.require_fixed_source(Language::Es);
        assert_eq!(
            settings.source_language,
            SourceLanguage::Fixed(Language::Es)
        );

        settings.source_language = SourceLanguage::Fixed(Language::En);
        settings.require_fixed_source(Language::Es);
        assert_eq!(
            settings.source_language,
            SourceLanguage::Fixed(Language::En)
        );
    }

    #[test]
    fn missing_settings_use_defaults() {
        assert_eq!(Settings::from_stored(None), Settings::default());
    }

    #[test]
    fn invalid_settings_use_defaults() {
        let stored = json!({ "theme": "purple" });
        assert_eq!(Settings::from_stored(Some(stored)), Settings::default());
    }

    #[test]
    fn partial_settings_keep_known_fields() {
        let stored = json!({
            "theme": "dark",
            "targetLanguage": { "kind": "translate", "language": "es" }
        });
        let settings = Settings::from_stored(Some(stored));
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(
            settings.target_language,
            TargetLanguage::Translate(Language::Es)
        );
        assert_eq!(settings.ui_language, UiLanguage::System);
        // Settings saved before history existed keep it on (D22).
        assert!(settings.keep_history);
    }
}
