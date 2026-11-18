//! Languages supported for transcription and translation.
//!
//! v1 supports English and Spanish only.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A spoken or written language supported by Esperanto.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Es,
}

impl Language {
    /// Every supported language, in display order.
    pub const ALL: [Language; 2] = [Language::En, Language::Es];

    /// BCP 47 language tag.
    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Es => "es",
        }
    }

    /// English name, for prompts sent to models.
    pub fn english_name(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Es => "Spanish",
        }
    }

    /// Parses a language tag such as `"es"`, `"es-419"` or `"en_US"`.
    pub fn from_tag(tag: &str) -> Option<Language> {
        let base = tag.split(['-', '_']).next()?.to_ascii_lowercase();
        Language::ALL.into_iter().find(|l| l.code() == base)
    }
}

/// Language spoken in the audio: detected automatically or fixed by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase", tag = "kind", content = "language")]
pub enum SourceLanguage {
    #[default]
    Auto,
    Fixed(Language),
}

/// Language to translate into, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase", tag = "kind", content = "language")]
pub enum TargetLanguage {
    /// Transcription only.
    #[default]
    None,
    Translate(Language),
}

/// The language to translate a segment spoken in `spoken` into, if any.
/// No translation when there is no target or it is the spoken language.
pub fn translation_target(target: Option<Language>, spoken: Language) -> Option<Language> {
    target.filter(|t| *t != spoken)
}

impl TargetLanguage {
    pub fn language(self) -> Option<Language> {
        match self {
            TargetLanguage::None => None,
            TargetLanguage::Translate(lang) => Some(lang),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_is_skipped_when_languages_match() {
        let target = TargetLanguage::Translate(Language::Es).language();
        assert_eq!(translation_target(target, Language::Es), None);
        assert_eq!(translation_target(target, Language::En), Some(Language::Es));
        assert_eq!(
            translation_target(TargetLanguage::None.language(), Language::En),
            None
        );
    }

    #[test]
    fn parses_language_tags() {
        assert_eq!(Language::from_tag("es-419"), Some(Language::Es));
        assert_eq!(Language::from_tag("en_US"), Some(Language::En));
        assert_eq!(Language::from_tag("fr"), None);
    }

    #[test]
    fn serializes_with_stable_wire_format() {
        let json = serde_json::to_string(&TargetLanguage::Translate(Language::Es)).unwrap();
        assert_eq!(json, r#"{"kind":"translate","language":"es"}"#);
        let json = serde_json::to_string(&SourceLanguage::Auto).unwrap();
        assert_eq!(json, r#"{"kind":"auto"}"#);
    }
}
