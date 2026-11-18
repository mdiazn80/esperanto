//! Turns per-frame voice probabilities into speech segments.
//!
//! A segment starts after a few consecutive voiced frames (keeping a short
//! pre-roll so the first syllable isn't cut) and ends after a pause, or is
//! cut at a maximum length so latency stays bounded. Hysteresis (a lower
//! threshold to stay in speech than to enter it) avoids flapping.

use std::collections::VecDeque;

use crate::AudioSegment;

/// Samples per VAD frame: 16 ms at 16 kHz (what earshot expects).
pub const FRAME_SAMPLES: usize = 256;
const FRAME_MS: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmenterConfig {
    /// Probability needed to enter speech.
    pub start_threshold: f32,
    /// Probability below which a frame counts as silence once speaking.
    pub end_threshold: f32,
    /// Consecutive voiced frames needed to start a segment.
    pub start_frames: u32,
    /// Silence that ends a segment.
    pub end_silence_ms: u32,
    /// Segments are cut at this length to bound latency.
    pub max_segment_ms: u32,
    /// Audio kept from before speech was detected.
    pub pre_roll_ms: u32,
    /// Shorter speech (coughs, clicks) is discarded.
    pub min_speech_ms: u32,
    /// Silence kept at the end of a segment.
    pub tail_ms: u32,
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self {
            start_threshold: 0.5,
            end_threshold: 0.35,
            start_frames: 3,
            end_silence_ms: 400,
            max_segment_ms: 10_000,
            pre_roll_ms: 192,
            min_speech_ms: 250,
            tail_ms: 96,
        }
    }
}

const fn frames(ms: u32) -> usize {
    ms.div_ceil(FRAME_MS) as usize
}

/// Why a segment ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentEnd {
    /// The speaker paused.
    Pause,
    /// The segment reached the maximum length; speech continues in the next one.
    MaxLength,
    /// The session stopped while speaking.
    Flush,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpeechEvent {
    Started,
    Ended {
        segment: AudioSegment,
        reason: SegmentEnd,
    },
    /// Speech was too short to be useful.
    Discarded,
}

#[derive(Debug)]
enum State {
    Idle {
        pre_roll: VecDeque<Vec<f32>>,
        voiced_run: u32,
    },
    Speaking {
        samples: Vec<f32>,
        voiced_frames: usize,
        silence_frames: usize,
    },
}

#[derive(Debug)]
pub struct Segmenter {
    config: SegmenterConfig,
    state: State,
}

impl Segmenter {
    pub fn new(config: SegmenterConfig) -> Self {
        Self {
            config,
            state: State::Idle {
                pre_roll: VecDeque::new(),
                voiced_run: 0,
            },
        }
    }

    pub fn is_speaking(&self) -> bool {
        matches!(self.state, State::Speaking { .. })
    }

    /// Audio of the segment in progress, if speaking (for partial passes).
    pub fn current(&self) -> Option<&[f32]> {
        match &self.state {
            State::Speaking { samples, .. } => Some(samples),
            State::Idle { .. } => None,
        }
    }

    /// Feeds one frame of [`FRAME_SAMPLES`] samples and its voice probability.
    pub fn push(&mut self, frame: &[f32], probability: f32) -> Option<SpeechEvent> {
        let config = self.config;
        match &mut self.state {
            State::Idle {
                pre_roll,
                voiced_run,
            } => {
                pre_roll.push_back(frame.to_vec());
                while pre_roll.len() > frames(config.pre_roll_ms).max(1) {
                    pre_roll.pop_front();
                }
                *voiced_run = if probability >= config.start_threshold {
                    *voiced_run + 1
                } else {
                    0
                };
                if *voiced_run >= config.start_frames {
                    let samples: Vec<f32> = pre_roll.drain(..).flatten().collect();
                    self.state = State::Speaking {
                        samples,
                        voiced_frames: config.start_frames as usize,
                        silence_frames: 0,
                    };
                    return Some(SpeechEvent::Started);
                }
                None
            }
            State::Speaking {
                samples,
                voiced_frames,
                silence_frames,
            } => {
                samples.extend_from_slice(frame);
                if probability < config.end_threshold {
                    *silence_frames += 1;
                } else {
                    *silence_frames = 0;
                    *voiced_frames += 1;
                }

                if *silence_frames >= frames(config.end_silence_ms) {
                    return Some(self.finish(SegmentEnd::Pause));
                }
                if samples.len() >= frames(config.max_segment_ms) * FRAME_SAMPLES {
                    let segment = std::mem::take(samples);
                    *voiced_frames = 0;
                    *silence_frames = 0;
                    return Some(SpeechEvent::Ended {
                        segment: AudioSegment {
                            samples: segment,
                            sample_rate: AudioSegment::SAMPLE_RATE,
                        },
                        reason: SegmentEnd::MaxLength,
                    });
                }
                None
            }
        }
    }

    /// Ends the segment in progress, if any (e.g. when the session stops).
    pub fn flush(&mut self) -> Option<SpeechEvent> {
        self.is_speaking().then(|| self.finish(SegmentEnd::Flush))
    }

    fn finish(&mut self, reason: SegmentEnd) -> SpeechEvent {
        let previous = std::mem::replace(
            &mut self.state,
            State::Idle {
                pre_roll: VecDeque::new(),
                voiced_run: 0,
            },
        );
        let State::Speaking {
            mut samples,
            voiced_frames,
            silence_frames,
        } = previous
        else {
            unreachable!("finish is only called while speaking");
        };

        let trailing = silence_frames.saturating_sub(frames(self.config.tail_ms)) * FRAME_SAMPLES;
        samples.truncate(samples.len().saturating_sub(trailing));
        if voiced_frames < frames(self.config.min_speech_ms) {
            return SpeechEvent::Discarded;
        }
        SpeechEvent::Ended {
            segment: AudioSegment {
                samples,
                sample_rate: AudioSegment::SAMPLE_RATE,
            },
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: [f32; FRAME_SAMPLES] = [0.1; FRAME_SAMPLES];

    /// Runs `ms` of audio at a constant probability, collecting events.
    fn feed(seg: &mut Segmenter, ms: u32, probability: f32, events: &mut Vec<SpeechEvent>) {
        for _ in 0..frames(ms) {
            events.extend(seg.push(&FRAME, probability));
        }
    }

    fn durations_ms(events: &[SpeechEvent]) -> Vec<(u32, SegmentEnd)> {
        events
            .iter()
            .filter_map(|e| match e {
                SpeechEvent::Ended { segment, reason } => {
                    Some(((segment.duration_secs() * 1000.0).round() as u32, *reason))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn silence_produces_nothing() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 2_000, 0.1, &mut events);
        assert!(events.is_empty());
        assert!(!seg.is_speaking());
    }

    #[test]
    fn speech_then_pause_makes_one_segment_with_pre_roll_and_short_tail() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 500, 0.0, &mut events);
        feed(&mut seg, 1_600, 0.9, &mut events);
        feed(&mut seg, 600, 0.0, &mut events);

        assert_eq!(events.first(), Some(&SpeechEvent::Started));
        // 1600 ms of speech + 192 ms pre-roll (minus the 3 start frames already
        // counted in it) + 96 ms tail.
        assert_eq!(durations_ms(&events), vec![(1_840, SegmentEnd::Pause)]);
        assert!(!seg.is_speaking());
    }

    #[test]
    fn short_pauses_and_mid_probabilities_stay_in_the_segment() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 500, 0.9, &mut events);
        feed(&mut seg, 200, 0.0, &mut events); // shorter than end_silence_ms
        feed(&mut seg, 500, 0.4, &mut events); // between thresholds: still speech
        feed(&mut seg, 500, 0.0, &mut events);
        assert_eq!(durations_ms(&events).len(), 1);
    }

    #[test]
    fn long_speech_is_cut_at_max_length_and_continues() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 23_000, 0.9, &mut events);
        assert!(seg.is_speaking());
        let cuts = durations_ms(&events);
        assert_eq!(cuts.len(), 2);
        assert!(
            cuts.iter()
                .all(|(ms, reason)| *ms <= 10_016 && *reason == SegmentEnd::MaxLength)
        );
        assert!(seg.current().is_some_and(|s| !s.is_empty()));
    }

    #[test]
    fn blips_are_discarded() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 100, 0.9, &mut events);
        feed(&mut seg, 600, 0.0, &mut events);
        assert_eq!(events, vec![SpeechEvent::Started, SpeechEvent::Discarded]);
    }

    #[test]
    fn flush_ends_the_current_segment() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let mut events = vec![];
        feed(&mut seg, 1_000, 0.9, &mut events);
        events.extend(seg.flush());
        assert_eq!(durations_ms(&events)[0].1, SegmentEnd::Flush);
        assert_eq!(seg.flush(), None);
    }
}
