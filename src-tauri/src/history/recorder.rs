//! Records the running session for the History view, saving it after each
//! phrase so a crash loses at most the phrase in progress.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use esperanto_core::Language;

use super::model::{FORMAT_VERSION, HistorySegment, HistorySession};
use super::store::HistoryStore;
use crate::util::random_hex;

pub struct Recorder {
    session: HistorySession,
    /// Segment id of each entry in `session.segments`.
    ids: Vec<u32>,
    /// Where to save; `None` when history isn't kept (the session then
    /// lives in memory only).
    store: Option<Arc<HistoryStore>>,
}

impl Recorder {
    pub fn new(
        store: Option<Arc<HistoryStore>>,
        model_name: String,
        remote_host: Option<String>,
    ) -> Self {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_millis() as f64);
        Self {
            session: HistorySession {
                version: FORMAT_VERSION,
                id: format!("{}-{}", started_at as u64, random_hex(3)),
                started_at,
                model_name,
                remote_host,
                device_name: String::new(),
                segments: Vec::new(),
            },
            ids: Vec::new(),
            store,
        }
    }

    pub fn id(&self) -> &str {
        &self.session.id
    }

    pub fn session(&self) -> &HistorySession {
        &self.session
    }

    pub fn set_device(&mut self, name: &str) {
        self.session.device_name = name.to_owned();
        if !self.session.segments.is_empty() {
            self.save();
        }
    }

    pub fn add_final(
        &mut self,
        segment_id: u32,
        start_ms: f64,
        end_ms: f64,
        text: &str,
        language: Language,
    ) {
        if text.trim().is_empty() {
            return;
        }
        self.ids.push(segment_id);
        self.session.segments.push(HistorySegment {
            start_ms,
            end_ms,
            text: text.trim().to_owned(),
            language,
            translation: None,
            translation_language: None,
        });
        self.save();
    }

    pub fn add_translation(&mut self, segment_id: u32, text: &str, language: Language) {
        let Some(index) = self.ids.iter().rposition(|id| *id == segment_id) else {
            return;
        };
        let segment = &mut self.session.segments[index];
        segment.translation = Some(text.trim().to_owned());
        segment.translation_language = Some(language);
        self.save();
    }

    /// Stops saving this session and removes what was saved of it (the
    /// user turned history off or deleted it).
    pub fn stop_keeping(&mut self) {
        if let Some(store) = self.store.take()
            && let Err(error) = store.delete(&self.session.id)
        {
            log::warn!("could not delete history session: {error}");
        }
    }

    fn save(&self) {
        // Sessions without text aren't worth keeping.
        if self.session.segments.is_empty() {
            return;
        }
        if let Some(store) = &self.store
            && let Err(error) = store.save(&self.session)
        {
            log::error!("could not save history session: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_finals_and_their_translations() {
        let mut recorder = Recorder::new(None, "Model".into(), None);
        recorder.add_final(1, 0.0, 1_000.0, " Hello. ", Language::En);
        recorder.add_final(2, 1_500.0, 2_000.0, "   ", Language::En); // no text
        recorder.add_final(3, 2_500.0, 4_000.0, "Bye.", Language::En);
        recorder.add_translation(3, "Adiós.", Language::Es);
        recorder.add_translation(9, "Unknown.", Language::Es);

        let segments = &recorder.session().segments;
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello.");
        assert_eq!(segments[0].translation, None);
        assert_eq!(segments[1].translation.as_deref(), Some("Adiós."));
        assert_eq!(segments[1].translation_language, Some(Language::Es));
        assert!(super::super::store::is_valid_id(recorder.id()));
    }
}
