//! Runs on the capture's DSP thread: feeds the [`Listener`], numbers speech
//! segments and hands work to the workers — the latest snapshot of the
//! phrase being spoken (newer replaces older) and every finished phrase.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use esperanto_audio::{AudioError, CaptureHandler};
use esperanto_core::{
    AudioSegment, ListenEvent, Listener, SegmentEnd, SegmenterConfig, SpeechEvent,
};
use tokio::sync::{mpsc, watch};

use super::events::SessionEvent;
use super::workers::{FinalJob, PartialJob, Shared};
use crate::error::ErrorCode;

/// A partial pass is sent after at least this much new audio.
const PARTIAL_EVERY_SECS: f64 = 1.0;
/// Don't transcribe snapshots shorter than this.
const PARTIAL_MIN_SECS: f64 = 0.8;
/// Digital silence on system audio for this long triggers a hint (8 s).
const SILENT_HINT_SAMPLES: usize = 8 * AudioSegment::SAMPLE_RATE as usize;

pub struct Handler {
    shared: Shared,
    listener: Listener,
    events: Vec<ListenEvent>,
    segment_id: u32,
    /// Segment length when the last partial snapshot was taken.
    partial_at: usize,
    partials: watch::Sender<Option<Arc<PartialJob>>>,
    finals: mpsc::UnboundedSender<FinalJob>,
    /// Samples received since the session started (16 kHz), for timings.
    samples_seen: u64,
    /// Consecutive exactly-zero samples when capturing system audio
    /// (`None` for other sources, or once the hint was sent).
    silent_samples: Option<usize>,
}

impl Handler {
    pub fn new(
        shared: Shared,
        partials: watch::Sender<Option<Arc<PartialJob>>>,
        finals: mpsc::UnboundedSender<FinalJob>,
        system_audio: bool,
    ) -> Self {
        Self {
            shared,
            listener: Listener::new(SegmenterConfig::default()),
            events: Vec::new(),
            segment_id: 0,
            partial_at: 0,
            partials,
            finals,
            samples_seen: 0,
            silent_samples: system_audio.then_some(0),
        }
    }

    fn send(&self, event: SessionEvent) {
        self.shared.send(event);
    }

    fn start_segment(&mut self) {
        self.segment_id += 1;
        self.partial_at = 0;
        self.send(SessionEvent::SpeechStarted {
            segment_id: self.segment_id,
        });
    }

    fn forward(&mut self) {
        for event in std::mem::take(&mut self.events) {
            match event {
                ListenEvent::Level {
                    rms_db,
                    voice_probability,
                } => self.send(SessionEvent::Level {
                    rms_db,
                    voice_probability,
                }),
                ListenEvent::Speech(SpeechEvent::Started) => self.start_segment(),
                ListenEvent::Speech(SpeechEvent::Discarded) => {
                    self.send(SessionEvent::SpeechDiscarded {
                        segment_id: self.segment_id,
                    });
                }
                ListenEvent::Speech(SpeechEvent::Ended { segment, reason }) => {
                    self.shared.queued.fetch_add(1, Ordering::Relaxed);
                    let end_ms = ms(self.samples_seen);
                    let start_ms = (end_ms - ms(segment.samples.len() as u64)).max(0.0);
                    let _ = self.finals.send(FinalJob {
                        segment_id: self.segment_id,
                        audio: segment,
                        ended_at: Instant::now(),
                        start_ms,
                        end_ms,
                    });
                    // Speech goes on after a length cut: it's a new segment.
                    if reason == SegmentEnd::MaxLength {
                        self.start_segment();
                    }
                }
            }
        }
        self.maybe_send_partial();
    }

    fn maybe_send_partial(&mut self) {
        let Some(current) = self.listener.segmenter().current() else {
            return;
        };
        let rate = f64::from(AudioSegment::SAMPLE_RATE);
        let len = current.len();
        let new_secs = len.saturating_sub(self.partial_at) as f64 / rate;
        if len as f64 / rate >= PARTIAL_MIN_SECS && new_secs >= PARTIAL_EVERY_SECS {
            self.partial_at = len;
            self.partials.send_replace(Some(Arc::new(PartialJob {
                segment_id: self.segment_id,
                audio: AudioSegment {
                    samples: current.to_vec(),
                    sample_rate: AudioSegment::SAMPLE_RATE,
                },
            })));
        }
    }

    fn track_silence(&mut self, samples: &[f32]) {
        let Some(silent) = self.silent_samples.as_mut() else {
            return;
        };
        if samples.iter().all(|s| *s == 0.0) {
            *silent += samples.len();
            if *silent >= SILENT_HINT_SAMPLES {
                self.silent_samples = None;
                self.send(SessionEvent::SystemAudioSilent);
            }
        } else {
            *silent = 0;
        }
    }
}

/// Duration of `samples` at 16 kHz, in ms.
fn ms(samples: u64) -> f64 {
    samples as f64 * 1000.0 / f64::from(AudioSegment::SAMPLE_RATE)
}

impl CaptureHandler for Handler {
    fn samples(&mut self, samples: &[f32]) {
        self.samples_seen += samples.len() as u64;
        self.track_silence(samples);
        self.listener.push(samples, &mut self.events);
        self.forward();
    }

    fn error(&mut self, error: AudioError) {
        log::error!("audio capture failed: {error}");
        self.send(SessionEvent::Error {
            code: ErrorCode::from(&error),
        });
    }

    fn finished(&mut self) {
        self.listener.flush(&mut self.events);
        self.forward();
    }
}
