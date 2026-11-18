//! Speaker labels for a session: the voice print of each
//! finished phrase, grouped into speakers as the session goes.

use std::path::Path;
use std::sync::Mutex;

use esperanto_core::AudioSegment;
use esperanto_speakers::{MIN_RELIABLE_SECS, SpeakerEmbedder, SpeakerTracker, SpeakersError};

pub struct Speakers {
    embedder: Mutex<SpeakerEmbedder>,
    tracker: Mutex<SpeakerTracker>,
}

impl Speakers {
    /// Loads the model. Blocking (reads and prepares ~30 MB).
    pub fn load(model: &Path) -> Result<Self, SpeakersError> {
        Ok(Self {
            embedder: Mutex::new(SpeakerEmbedder::new(model)?),
            tracker: Mutex::default(),
        })
    }

    /// The speaker of a phrase, if it can be told. Blocking (runs the
    /// model on the CPU, a few tens of ms).
    pub fn assign(&self, audio: &AudioSegment) -> Option<u32> {
        let print = self.embedder.lock().ok()?.embed(&audio.samples)?;
        let reliable = audio.duration_secs() >= MIN_RELIABLE_SECS;
        self.tracker.lock().ok()?.assign(&print, reliable)
    }
}
