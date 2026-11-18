//! Listens to 16 kHz mono audio: measures the level, runs the VAD
//! (earshot) and segments speech.

use crate::segmenter::{FRAME_SAMPLES, Segmenter, SegmenterConfig, SpeechEvent};

/// Level updates are sent every this many frames (~20 per second).
const LEVEL_EVERY_FRAMES: u32 = 3;
/// Quietest level reported, in dBFS.
const LEVEL_FLOOR_DB: f32 = -80.0;

#[derive(Debug, Clone, PartialEq)]
pub enum ListenEvent {
    /// Input level (RMS, dBFS) and voice probability, for meters.
    Level {
        rms_db: f32,
        voice_probability: f32,
    },
    Speech(SpeechEvent),
}

pub struct Listener {
    vad: Box<earshot::Detector>,
    segmenter: Segmenter,
    pending: Vec<f32>,
    frames_since_level: u32,
    level_sum_squares: f64,
    level_samples: usize,
    level_max_probability: f32,
}

impl Listener {
    pub fn new(config: SegmenterConfig) -> Self {
        Self {
            vad: earshot::Detector::default_boxed(),
            segmenter: Segmenter::new(config),
            pending: Vec::with_capacity(FRAME_SAMPLES * 2),
            frames_since_level: 0,
            level_sum_squares: 0.0,
            level_samples: 0,
            level_max_probability: 0.0,
        }
    }

    pub fn segmenter(&self) -> &Segmenter {
        &self.segmenter
    }

    /// Feeds any number of samples; events are appended to `events`.
    /// Samples outside [-1, 1] (loud system audio, resampler overshoot) are
    /// clipped: the VAD requires that range.
    pub fn push(&mut self, samples: &[f32], events: &mut Vec<ListenEvent>) {
        self.pending.extend(samples.iter().map(|s| {
            if s.is_finite() {
                s.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        }));
        let mut offset = 0;
        while self.pending.len() - offset >= FRAME_SAMPLES {
            let frame: [f32; FRAME_SAMPLES] = self.pending[offset..offset + FRAME_SAMPLES]
                .try_into()
                .expect("slice has FRAME_SAMPLES samples");
            offset += FRAME_SAMPLES;
            self.process_frame(&frame, events);
        }
        self.pending.drain(..offset);
    }

    /// Ends any segment in progress.
    pub fn flush(&mut self, events: &mut Vec<ListenEvent>) {
        events.extend(self.segmenter.flush().map(ListenEvent::Speech));
    }

    fn process_frame(&mut self, frame: &[f32; FRAME_SAMPLES], events: &mut Vec<ListenEvent>) {
        let probability = self.vad.predict_f32(frame);

        self.level_sum_squares += frame.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
        self.level_samples += frame.len();
        self.level_max_probability = self.level_max_probability.max(probability);
        self.frames_since_level += 1;
        if self.frames_since_level >= LEVEL_EVERY_FRAMES {
            let rms = (self.level_sum_squares / self.level_samples as f64).sqrt() as f32;
            let rms_db = if rms > 0.0 {
                (20.0 * rms.log10()).max(LEVEL_FLOOR_DB)
            } else {
                LEVEL_FLOOR_DB
            };
            events.push(ListenEvent::Level {
                rms_db,
                voice_probability: self.level_max_probability,
            });
            self.frames_since_level = 0;
            self.level_sum_squares = 0.0;
            self.level_samples = 0;
            self.level_max_probability = 0.0;
        }

        events.extend(
            self.segmenter
                .push(frame, probability)
                .map(ListenEvent::Speech),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_levels_and_buffers_partial_frames() {
        let mut listener = Listener::new(SegmenterConfig::default());
        let mut events = vec![];
        // 2.5 frames: nothing yet.
        listener.push(&vec![0.5; FRAME_SAMPLES * 5 / 2], &mut events);
        assert!(events.is_empty());
        // Half a frame more completes the third frame: one level update.
        listener.push(&vec![0.5; FRAME_SAMPLES / 2], &mut events);
        let ListenEvent::Level { rms_db, .. } = events[0] else {
            panic!("expected a level event, got {events:?}");
        };
        assert!((rms_db - 20.0 * 0.5_f32.log10()).abs() < 0.01);
    }

    #[test]
    fn clips_out_of_range_and_invalid_samples() {
        // earshot asserts its input is within [-1, 1]: loud system audio
        // used to panic the capture thread in debug builds.
        let mut listener = Listener::new(SegmenterConfig::default());
        let mut events = vec![];
        let loud: Vec<f32> = (0..16_000)
            .map(|i| match i % 4 {
                0 => 1.8,
                1 => -2.5,
                2 => f32::NAN,
                _ => f32::INFINITY,
            })
            .collect();
        listener.push(&loud, &mut events);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ListenEvent::Level { .. }))
        );
    }

    #[test]
    fn silence_reports_the_floor_level_and_no_speech() {
        let mut listener = Listener::new(SegmenterConfig::default());
        let mut events = vec![];
        listener.push(&vec![0.0; 16_000], &mut events);
        assert!(events.iter().all(|e| matches!(
            e,
            ListenEvent::Level { rms_db, .. } if *rms_db == LEVEL_FLOOR_DB
        )));
        assert!(!listener.segmenter().is_speaking());
    }
}
