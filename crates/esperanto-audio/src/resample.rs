//! Converts captured audio to 16 kHz mono.

use esperanto_core::AudioSegment;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::AudioError;

pub struct ToMono16k {
    channels: usize,
    resampler: Option<Fft<f32>>,
    chunk: usize,
    pending: Vec<f32>,
    output: Vec<f32>,
}

impl ToMono16k {
    pub fn new(sample_rate: u32, channels: usize) -> Result<Self, AudioError> {
        let target = AudioSegment::SAMPLE_RATE;
        // 10 ms chunks keep the added delay small.
        let chunk = (sample_rate / 100).max(1) as usize;
        let resampler = (sample_rate != target)
            .then(|| {
                Fft::<f32>::new(
                    sample_rate as usize,
                    target as usize,
                    chunk,
                    1,
                    FixedSync::Input,
                )
            })
            .transpose()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        Ok(Self {
            channels: channels.max(1),
            resampler,
            chunk,
            pending: Vec::with_capacity(chunk * 2),
            output: Vec::new(),
        })
    }

    /// Converts what is still buffered (less than one chunk), padding it
    /// with silence. Call once at the end of a file.
    pub fn flush(&mut self) -> Result<&[f32], AudioError> {
        let remaining = self.pending.len();
        if remaining == 0 || self.resampler.is_none() {
            self.output.clear();
            return Ok(&self.output);
        }
        let padding = vec![0.0; (self.chunk - remaining) * self.channels];
        let target = self.output_len_for(remaining);
        self.process(&padding)?;
        let keep = target.min(self.output.len());
        self.output.truncate(keep);
        Ok(&self.output)
    }

    fn output_len_for(&self, input_frames: usize) -> usize {
        let ratio = f64::from(AudioSegment::SAMPLE_RATE) / (self.chunk as f64 * 100.0);
        (input_frames as f64 * ratio).round() as usize
    }

    /// Converts interleaved input; returns the 16 kHz mono samples available so far.
    pub fn process(&mut self, interleaved: &[f32]) -> Result<&[f32], AudioError> {
        self.output.clear();
        let channels = self.channels;
        let mono = interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32);

        let Some(resampler) = &mut self.resampler else {
            self.output.extend(mono);
            return Ok(&self.output);
        };

        self.pending.extend(mono);
        let mut offset = 0;
        let mut buffer = vec![0.0_f32; resampler.output_frames_max()];
        while self.pending.len() - offset >= self.chunk {
            let input = &self.pending[offset..offset + self.chunk];
            let input = InterleavedSlice::new(input, 1, self.chunk)
                .map_err(|e| AudioError::Stream(e.to_string()))?;
            let capacity = buffer.len();
            let mut out = InterleavedSlice::new_mut(&mut buffer, 1, capacity)
                .map_err(|e| AudioError::Stream(e.to_string()))?;
            let (_, written) = resampler
                .process_into_buffer(&input, &mut out, None)
                .map_err(|e| AudioError::Stream(e.to_string()))?;
            // The anti-aliasing filter can overshoot slightly on loud input.
            self.output
                .extend(buffer[..written].iter().map(|s| s.clamp(-1.0, 1.0)));
            offset += self.chunk;
        }
        self.pending.drain(..offset);
        Ok(&self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, secs: f32, channels: usize) -> Vec<f32> {
        let frames = (rate as f32 * secs) as usize;
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5;
                std::iter::repeat_n(v, channels)
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn passes_16k_mono_through() {
        let mut conv = ToMono16k::new(16_000, 1).unwrap();
        let input = sine(16_000, 440.0, 0.1, 1);
        assert_eq!(conv.process(&input).unwrap(), input.as_slice());
    }

    #[test]
    fn resamples_48k_stereo_to_16k_mono_preserving_level() {
        let mut conv = ToMono16k::new(48_000, 2).unwrap();
        let input = sine(48_000, 440.0, 1.0, 2);
        let mut out = Vec::new();
        // Feed in uneven pieces, like a real callback.
        for piece in input.chunks(2 * 333) {
            out.extend_from_slice(conv.process(piece).unwrap());
        }
        assert!((15_500..=16_000).contains(&out.len()), "got {}", out.len());
        let steady = &out[2_000..out.len() - 2_000];
        assert!((rms(steady) - 0.5 / 2_f32.sqrt()).abs() < 0.02);
    }

    #[test]
    fn removes_content_above_the_new_nyquist() {
        let mut conv = ToMono16k::new(48_000, 1).unwrap();
        let out = conv
            .process(&sine(48_000, 12_000.0, 1.0, 1))
            .unwrap()
            .to_vec();
        assert!(rms(&out[2_000..]) < 0.02);
    }
}
