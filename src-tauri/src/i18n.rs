//! Localization for native UI (tray, menus) built in Rust.
//!
//! Reuses the frontend locale files so there is a single source of truth
//! for every user-facing string.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

use esperanto_core::Language;

use crate::settings::UiLanguage;

/// A language the interface is translated into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    Es,
}

impl From<Language> for Locale {
    fn from(language: Language) -> Self {
        match language {
            Language::En => Locale::En,
            Language::Es => Locale::Es,
        }
    }
}

impl Locale {
    const ALL: [Locale; 2] = [Locale::En, Locale::Es];
    const DEFAULT: Locale = Locale::En;

    /// The spoken language matching this interface language.
    pub fn language(self) -> Language {
        match self {
            Locale::En => Language::En,
            Locale::Es => Language::Es,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Es => "es",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Locale::En => include_str!("../../src/locales/en/common.json"),
            Locale::Es => include_str!("../../src/locales/es/common.json"),
        }
    }
}

static CATALOGS: LazyLock<HashMap<Locale, Value>> = LazyLock::new(|| {
    Locale::ALL
        .into_iter()
        .map(|locale| {
            let catalog = serde_json::from_str(locale.source())
                .unwrap_or_else(|e| panic!("invalid {} locale file: {e}", locale.code()));
            (locale, catalog)
        })
        .collect()
});

/// Picks the interface language: the user's choice, or the first OS
/// language we support, falling back to English. Mirrors
/// `resolveUiLanguage` in `src/lib/i18n.ts`.
pub fn resolve(setting: UiLanguage) -> Locale {
    resolve_with(setting, sys_locale::get_locales())
}

fn resolve_with(setting: UiLanguage, system: impl IntoIterator<Item = String>) -> Locale {
    match setting {
        UiLanguage::En => Locale::En,
        UiLanguage::Es => Locale::Es,
        UiLanguage::System => system
            .into_iter()
            .find_map(|tag| Language::from_tag(&tag))
            .map_or(Locale::DEFAULT, Locale::from),
    }
}

fn lookup(locale: Locale, key: &str) -> Option<&'static str> {
    key.split('.')
        .try_fold(&CATALOGS[&locale], |node, part| node.get(part))?
        .as_str()
}

/// Translates a dotted key (e.g. `"tray.quit"`), falling back to English
/// and then to the key itself.
pub fn t(locale: Locale, key: &str) -> String {
    lookup(locale, key)
        .or_else(|| lookup(Locale::DEFAULT, key))
        .unwrap_or(key)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key translated from Rust. Keep in sync when adding native UI.
    const RUST_KEYS: &[&str] = &["tray.tooltip", "tray.show", "tray.quit"];

    #[test]
    fn rust_keys_exist_in_every_locale() {
        for locale in Locale::ALL {
            for key in RUST_KEYS {
                assert!(
                    lookup(locale, key).is_some(),
                    "missing \"{key}\" in {} locale",
                    locale.code()
                );
            }
        }
    }

    #[test]
    fn translates_and_falls_back() {
        assert_eq!(t(Locale::Es, "tray.quit"), "Salir de Esperanto");
        assert_eq!(t(Locale::En, "no.such.key"), "no.such.key");
    }

    #[test]
    fn resolves_ui_language() {
        let system = |tags: &[&str]| tags.iter().map(|t| t.to_string()).collect::<Vec<_>>();
        assert_eq!(resolve_with(UiLanguage::Es, system(&["en-US"])), Locale::Es);
        assert_eq!(
            resolve_with(UiLanguage::System, system(&["fr-FR", "es_ES", "en"])),
            Locale::Es
        );
        assert_eq!(
            resolve_with(UiLanguage::System, system(&["de"])),
            Locale::En
        );
        assert_eq!(resolve_with(UiLanguage::System, system(&[])), Locale::En);
    }
}
