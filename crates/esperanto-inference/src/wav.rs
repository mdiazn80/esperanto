//! Minimal WAV encoder: 16-bit PCM, mono. What audio-capable chat
//! endpoints accept as `input_audio` with `format: "wav"`.

use esperanto_core::AudioSegment;

pub fn encode_pcm16(audio: &AudioSegment) -> Vec<u8> {
    const HEADER_LEN: usize = 44;
    let data_len = audio.samples.len() * 2;
    let mut out = Vec::with_capacity(HEADER_LEN + data_len);

    let byte_rate = audio.sample_rate * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&audio.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in &audio.samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_valid_header_and_clamped_samples() {
        let audio = AudioSegment {
            samples: vec![0.0, 1.0, -1.0, 2.0],
            sample_rate: 16_000,
        };
        let wav = encode_pcm16(&audio);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8);
        let samples: Vec<i16> = wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        assert_eq!(samples, vec![0, i16::MAX, -i16::MAX, i16::MAX]);
    }
}
