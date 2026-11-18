//! What a saved session looks like on disk (`history/<id>.json`) and in the
//! History view.

use esperanto_core::Language;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Bumped when the file format changes incompatibly.
pub const FORMAT_VERSION: u32 = 1;

/// One transcribed phrase. Times are in ms from the start of the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistorySegment {
    pub start_ms: f64,
    pub end_ms: f64,
    pub text: String,
    pub language: Language,
    pub translation: Option<String>,
    pub translation_language: Option<Language>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistorySession {
    pub version: u32,
    pub id: String,
    /// Unix time in ms.
    pub started_at: f64,
    pub model_name: String,
    /// Host of the remote gateway; `None` for the local model.
    pub remote_host: Option<String>,
    pub device_name: String,
    pub segments: Vec<HistorySegment>,
}

impl HistorySession {
    /// Length of the session up to its last phrase, in ms.
    pub fn duration_ms(&self) -> f64 {
        self.segments.last().map_or(0.0, |s| s.end_ms)
    }

    pub fn summary(&self) -> HistorySummary {
        let mut languages: Vec<Language> = self.segments.iter().map(|s| s.language).collect();
        languages.sort();
        languages.dedup();
        let mut translations: Vec<Language> = self
            .segments
            .iter()
            .filter_map(|s| s.translation_language)
            .collect();
        translations.sort();
        translations.dedup();
        HistorySummary {
            id: self.id.clone(),
            started_at: self.started_at,
            duration_ms: self.duration_ms(),
            languages,
            translation_languages: translations,
            model_name: self.model_name.clone(),
            remote_host: self.remote_host.clone(),
            segment_count: self.segments.len() as u32,
            preview: preview(&self.segments),
        }
    }
}

/// A session in the History list.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub id: String,
    pub started_at: f64,
    pub duration_ms: f64,
    /// Spoken languages, and languages translated into.
    pub languages: Vec<Language>,
    pub translation_languages: Vec<Language>,
    pub model_name: String,
    pub remote_host: Option<String>,
    pub segment_count: u32,
    /// The first words, to recognize the session.
    pub preview: String,
}

/// Longest preview, in characters.
const PREVIEW_CHARS: usize = 140;

fn preview(segments: &[HistorySegment]) -> String {
    let mut text = String::new();
    for segment in segments {
        if text.chars().count() >= PREVIEW_CHARS {
            break;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(segment.text.trim());
    }
    if text.chars().count() > PREVIEW_CHARS {
        let cut: String = text.chars().take(PREVIEW_CHARS).collect();
        format!("{}…", cut.trim_end())
    } else {
        text
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn segment(start_ms: f64, end_ms: f64, text: &str) -> HistorySegment {
        HistorySegment {
            start_ms,
            end_ms,
            text: text.into(),
            language: Language::En,
            translation: Some(format!("[es] {text}")),
            translation_language: Some(Language::Es),
        }
    }

    pub fn session(segments: Vec<HistorySegment>) -> HistorySession {
        HistorySession {
            version: FORMAT_VERSION,
            id: "1795015049473-abcdef".into(),
            started_at: 1_795_015_049_473.0,
            model_name: "Voxtral Mini 3B".into(),
            remote_host: None,
            device_name: "MacBook Pro Microphone".into(),
            segments,
        }
    }

    #[test]
    fn summarizes_a_session() {
        let summary = session(vec![
            segment(500.0, 2_000.0, "Good morning."),
            segment(2_500.0, 4_250.0, "How are you?"),
        ])
        .summary();
        assert_eq!(summary.duration_ms, 4_250.0);
        assert_eq!(summary.languages, vec![Language::En]);
        assert_eq!(summary.translation_languages, vec![Language::Es]);
        assert_eq!(summary.segment_count, 2);
        assert_eq!(summary.preview, "Good morning. How are you?");
    }

    #[test]
    fn previews_are_shortened() {
        let long = "word ".repeat(100);
        let summary = session(vec![segment(0.0, 1.0, &long)]).summary();
        assert!(summary.preview.ends_with('…'));
        assert!(summary.preview.chars().count() <= PREVIEW_CHARS + 1);
    }
}
