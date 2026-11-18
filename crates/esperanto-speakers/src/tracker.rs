/// Thresholds of the live clustering. Cosine similarities, from -1 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerConfig {
    /// A reliable print joins the most similar speaker at or above this;
    /// below it, it starts a new speaker.
    pub same_speaker: f32,
    /// A short phrase only joins a speaker at or above this (it never
    /// starts a new one).
    pub short_phrase: f32,
    /// Past this many speakers, prints join the closest one.
    pub max_speakers: usize,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            same_speaker: 0.5,
            short_phrase: 0.6,
            max_speakers: 8,
        }
    }
}

struct Speaker {
    /// Sum of the unit-length prints assigned to it; its direction is the
    /// speaker's centroid.
    sum: Vec<f32>,
}

/// Groups the voice prints of one session into speakers, numbered from 0
/// in order of appearance.
#[derive(Default)]
pub struct SpeakerTracker {
    config: TrackerConfig,
    speakers: Vec<Speaker>,
}

fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (na, nb) = (norm(a), norm(b));
    if na == 0.0 || nb == 0.0 || a.len() != b.len() {
        return -1.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() / (na * nb)
}

impl SpeakerTracker {
    pub fn new(config: TrackerConfig) -> Self {
        Self {
            config,
            speakers: Vec::new(),
        }
    }

    pub fn speaker_count(&self) -> usize {
        self.speakers.len()
    }

    /// The speaker of a phrase. `reliable`: the phrase is long enough for a
    /// trustworthy print. `None`: a short phrase that matches nobody well.
    pub fn assign(&mut self, print: &[f32], reliable: bool) -> Option<u32> {
        let length = norm(print);
        if length == 0.0 || !length.is_finite() {
            return None;
        }
        let unit: Vec<f32> = print.iter().map(|x| x / length).collect();
        let best = self
            .speakers
            .iter()
            .enumerate()
            .map(|(i, s)| (i, cosine(&s.sum, &unit)))
            .max_by(|a, b| a.1.total_cmp(&b.1));

        let full = self.speakers.len() >= self.config.max_speakers;
        let index = match best {
            Some((i, similarity)) if !reliable => {
                (similarity >= self.config.short_phrase).then_some(i)?
            }
            Some((i, similarity)) if similarity >= self.config.same_speaker || full => i,
            _ if !reliable => return None,
            _ => {
                self.speakers.push(Speaker {
                    sum: vec![0.0; unit.len()],
                });
                self.speakers.len() - 1
            }
        };
        let speaker = &mut self.speakers[index];
        if speaker.sum.len() == unit.len() {
            for (s, u) in speaker.sum.iter_mut().zip(&unit) {
                *s += u;
            }
        }
        Some(index as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A print near `base` (unit axis), nudged by `noise` on another axis.
    fn print(base: usize, noise: f32) -> Vec<f32> {
        let mut v = vec![0.0; 8];
        v[base] = 1.0;
        v[(base + 1) % 8] = noise;
        v
    }

    #[test]
    fn groups_similar_prints_and_numbers_new_speakers() {
        let mut tracker = SpeakerTracker::default();
        assert_eq!(tracker.assign(&print(0, 0.1), true), Some(0));
        assert_eq!(tracker.assign(&print(3, 0.2), true), Some(1));
        assert_eq!(tracker.assign(&print(0, -0.2), true), Some(0));
        assert_eq!(tracker.assign(&print(3, 0.0), true), Some(1));
        assert_eq!(tracker.speaker_count(), 2);
    }

    #[test]
    fn short_phrases_only_join_clear_matches() {
        let mut tracker = SpeakerTracker::default();
        // Nobody known yet: no label, and no speaker created.
        assert_eq!(tracker.assign(&print(0, 0.0), false), None);
        assert_eq!(tracker.speaker_count(), 0);
        tracker.assign(&print(0, 0.0), true);
        assert_eq!(tracker.assign(&print(0, 0.3), false), Some(0));
        assert_eq!(tracker.assign(&print(5, 0.0), false), None);
        assert_eq!(tracker.speaker_count(), 1);
    }

    #[test]
    fn stops_creating_speakers_at_the_limit() {
        let mut tracker = SpeakerTracker::new(TrackerConfig {
            max_speakers: 2,
            ..TrackerConfig::default()
        });
        assert_eq!(tracker.assign(&print(0, 0.0), true), Some(0));
        assert_eq!(tracker.assign(&print(3, 0.0), true), Some(1));
        let third = tracker.assign(&print(6, 0.0), true);
        assert!(matches!(third, Some(0 | 1)));
        assert_eq!(tracker.speaker_count(), 2);
    }

    #[test]
    fn ignores_empty_or_invalid_prints() {
        let mut tracker = SpeakerTracker::default();
        assert_eq!(tracker.assign(&[0.0; 8], true), None);
        assert_eq!(tracker.assign(&[f32::NAN; 8], true), None);
        assert_eq!(tracker.speaker_count(), 0);
    }
}
