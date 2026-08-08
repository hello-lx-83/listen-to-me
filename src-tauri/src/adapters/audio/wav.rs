use std::io::Cursor;

use crate::core::models::RecordedAudio;

pub fn encode_mono_pcm16(audio: &RecordedAudio) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let cursor = Cursor::new(&mut bytes);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::new(cursor, spec)
        .map_err(|error| format!("failed to create WAV encoder: {error}"))?;
    for sample in &audio.samples {
        writer
            .write_sample(*sample)
            .map_err(|error| format!("failed to encode WAV sample: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("failed to finalize WAV payload: {error}"))?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_valid_mono_pcm16_wav() {
        let audio = RecordedAudio {
            samples: vec![0, i16::MAX, i16::MIN, 0],
            sample_rate: 16_000,
        };

        let bytes = encode_mono_pcm16(&audio).expect("WAV should encode");
        let reader = hound::WavReader::new(Cursor::new(bytes)).expect("WAV should parse");

        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 16_000);
        assert_eq!(reader.duration(), 4);
    }
}
