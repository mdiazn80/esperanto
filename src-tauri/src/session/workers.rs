//! The async side of a session: a *partial* worker transcribes the latest
//! snapshot of the phrase being spoken (dropped under load), and a *final*
//! worker transcribes finished phrases in order and translates them.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use esperanto_core::{AudioSegment, SourceLanguage, Stabilizer, translation_target};
use esperanto_inference::InferenceError;
use tauri::ipc::Channel;
use tokio::sync::{mpsc, watch};

use super::Languages;
use super::engine::Engine;
use super::events::{PipelineStatus, SessionEvent};
use super::speakers::Speakers;
use crate::history::Recorder;

/// More finished segments than this waiting = falling behind.
const BEHIND_THRESHOLD: usize = 2;

pub struct PartialJob {
    pub segment_id: u32,
    pub audio: AudioSegment,
}

pub struct FinalJob {
    pub segment_id: u32,
    pub audio: AudioSegment,
    pub ended_at: Instant,
    /// Position of the phrase in the session, in ms from its start.
    pub start_ms: f64,
    pub end_ms: f64,
}

/// State shared by the capture handler and both workers.
#[derive(Clone)]
pub struct Shared {
    pub engine: Arc<Engine>,
    pub languages: Arc<RwLock<Languages>>,
    /// Highest segment id with a final result (partials for it are stale).
    pub finished_up_to: Arc<AtomicU32>,
    /// Finished segments waiting for (or in) the final worker.
    pub queued: Arc<AtomicUsize>,
    pub channel: Channel<SessionEvent>,
    /// Keeps the finished phrases for the History view.
    pub recorder: Arc<Mutex<Recorder>>,
    /// Speaker labels, when on.
    pub speakers: Option<Arc<Speakers>>,
}

impl Shared {
    fn languages(&self) -> Languages {
        *self
            .languages
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn send(&self, event: SessionEvent) {
        // The window may be gone; nothing else to do with the event then.
        let _ = self.channel.send(event);
    }

    fn record(&self, record: impl FnOnce(&mut Recorder)) {
        if let Ok(mut recorder) = self.recorder.lock() {
            record(&mut recorder);
        }
    }

    fn is_finished(&self, segment_id: u32) -> bool {
        self.finished_up_to.load(Ordering::Relaxed) >= segment_id
    }
}

pub async fn partial_worker(shared: Shared, mut jobs: watch::Receiver<Option<Arc<PartialJob>>>) {
    let mut stabilizer = Stabilizer::default();
    let mut stabilizer_segment = 0;
    while jobs.changed().await.is_ok() {
        let job = jobs.borrow_and_update().clone();
        let Some(job) = job else { continue };
        // Finals have priority: skip partials while segments are waiting.
        if shared.is_finished(job.segment_id) || shared.queued.load(Ordering::Relaxed) > 0 {
            continue;
        }
        let source = SourceLanguage::Fixed(shared.languages().source);
        let Ok(transcript) = shared.engine.transcribe(&job.audio, source).await else {
            continue; // A missed partial is harmless; the final pass follows.
        };
        if shared.is_finished(job.segment_id) {
            continue;
        }
        if stabilizer_segment != job.segment_id {
            stabilizer.reset();
            stabilizer_segment = job.segment_id;
        }
        let partial = stabilizer.update(&transcript.text);
        shared.send(SessionEvent::Partial {
            segment_id: job.segment_id,
            stable: partial.stable,
            unstable: partial.unstable,
        });
    }
}

pub async fn final_worker(shared: Shared, mut jobs: mpsc::UnboundedReceiver<FinalJob>) {
    let mut status = PipelineStatus::Live;
    while let Some(job) = jobs.recv().await {
        let now = if shared.queued.load(Ordering::Relaxed) > BEHIND_THRESHOLD {
            PipelineStatus::Behind
        } else {
            PipelineStatus::Live
        };
        if now != status {
            status = now;
            shared.send(SessionEvent::Status { status });
        }
        process_final(&shared, job).await;
    }
    if status == PipelineStatus::Behind {
        shared.send(SessionEvent::Status {
            status: PipelineStatus::Live,
        });
    }
}

async fn process_final(shared: &Shared, job: FinalJob) {
    let Languages { source, target } = shared.languages();
    let id = job.segment_id;
    // Who spoke is worked out on the CPU while the model transcribes.
    let speaker = shared.speakers.clone().map(|speakers| {
        let audio = job.audio.clone();
        tauri::async_runtime::spawn_blocking(move || speakers.assign(&audio))
    });
    let transcript = shared
        .engine
        .transcribe(&job.audio, SourceLanguage::Fixed(source))
        .await;
    let speaker = match speaker {
        Some(task) => task.await.ok().flatten(),
        None => None,
    };
    shared.finished_up_to.fetch_max(id, Ordering::Relaxed);
    shared.queued.fetch_sub(1, Ordering::Relaxed);

    let transcript = match transcript {
        Ok(t) => t,
        Err(InferenceError::InvalidAnswer(_)) => {
            shared.send(SessionEvent::Empty { segment_id: id });
            return;
        }
        Err(error) => {
            log::error!("transcription of segment {id} failed: {error}");
            shared.send(SessionEvent::InferenceError {
                segment_id: id,
                code: shared.engine.error_code(&error),
            });
            return;
        }
    };
    if let Some(speaker) = speaker {
        shared.send(SessionEvent::Speaker {
            segment_id: id,
            speaker,
        });
    }
    shared.record(|r| r.add_final(id, job.start_ms, job.end_ms, &transcript.text, source));
    shared.send(SessionEvent::Final {
        segment_id: id,
        text: transcript.text.clone(),
        language: source,
        latency_ms: job.ended_at.elapsed().as_secs_f64() * 1000.0,
        start_ms: job.start_ms,
        end_ms: job.end_ms,
    });

    let Some(target) = translation_target(target, source) else {
        return;
    };
    match shared
        .engine
        .translate(&transcript.text, source, target)
        .await
    {
        Ok(text) => {
            shared.record(|r| r.add_translation(id, &text, target));
            shared.send(SessionEvent::Translation {
                segment_id: id,
                language: target,
                text,
            });
        }
        Err(error) => {
            log::error!("translation of segment {id} failed: {error}");
            shared.send(SessionEvent::InferenceError {
                segment_id: id,
                code: shared.engine.error_code(&error),
            });
        }
    }
}
