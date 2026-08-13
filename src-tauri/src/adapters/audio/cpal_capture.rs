use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};

use crate::core::{
    models::RecordedAudio,
    ports::{AudioCapture, PortFuture, PortResult},
};

pub struct CpalAudioCapture {
    active: Mutex<Option<ActiveCapture>>,
    level: Arc<AtomicU32>,
}

impl Default for CpalAudioCapture {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
            level: Arc::new(AtomicU32::new(0)),
        }
    }
}

struct ActiveCapture {
    _stream: Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
}

impl CpalAudioCapture {
    const TARGET_SAMPLE_RATE: u32 = 16_000;
    const MAX_RECORDING_SECONDS: usize = 120;

    pub fn input_level(&self) -> f32 {
        self.level.load(Ordering::Relaxed) as f32 / 1_000.0
    }

    fn start_capture(&self) -> PortResult<()> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "audio capture state is unavailable".to_owned())?;
        if active.is_some() {
            return Err("audio capture is already active".to_owned());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device is available".to_owned())?;
        let supported = device
            .default_input_config()
            .map_err(|error| format!("failed to read input configuration: {error}"))?;
        let config: StreamConfig = supported.clone().into();
        let channels = usize::from(config.channels);
        let samples = Arc::new(Mutex::new(Vec::new()));
        let captured = samples.clone();
        let level = self.level.clone();
        level.store(0, Ordering::Relaxed);
        let max_samples = config.sample_rate as usize * Self::MAX_RECORDING_SECONDS;
        let error_callback = |error| eprintln!("audio input stream error: {error}");

        let stream = match supported.sample_format() {
            SampleFormat::I8 => build_stream::<i8>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::I32 => build_stream::<i32>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::U8 => build_stream::<u8>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::U32 => build_stream::<u32>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            SampleFormat::F64 => build_stream::<f64>(
                &device,
                &config,
                channels,
                captured,
                level,
                max_samples,
                error_callback,
            ),
            format => Err(format!("unsupported input sample format: {format}")),
        }?;

        stream
            .play()
            .map_err(|error| format!("failed to start input stream: {error}"))?;
        *active = Some(ActiveCapture {
            _stream: stream,
            samples,
            sample_rate: config.sample_rate,
        });
        Ok(())
    }

    fn stop_capture(&self) -> PortResult<RecordedAudio> {
        let capture = self
            .active
            .lock()
            .map_err(|_| "audio capture state is unavailable".to_owned())?
            .take()
            .ok_or_else(|| "audio capture is not active".to_owned())?;
        let samples = capture
            .samples
            .lock()
            .map_err(|_| "captured audio buffer is unavailable".to_owned())?
            .clone();
        self.level.store(0, Ordering::Relaxed);

        Ok(RecordedAudio {
            samples: resample_mono_pcm16(&samples, capture.sample_rate, Self::TARGET_SAMPLE_RATE),
            sample_rate: Self::TARGET_SAMPLE_RATE,
        })
    }

    fn cancel_capture(&self) -> PortResult<()> {
        let capture = self
            .active
            .lock()
            .map_err(|_| "audio capture state is unavailable".to_owned())?
            .take();
        drop(capture);
        self.level.store(0, Ordering::Relaxed);
        Ok(())
    }
}

impl AudioCapture for CpalAudioCapture {
    fn start(&self) -> PortFuture<'_, ()> {
        Box::pin(async { self.start_capture() })
    }

    fn stop(&self) -> PortFuture<'_, RecordedAudio> {
        Box::pin(async { self.stop_capture() })
    }

    fn cancel(&self) -> PortFuture<'_, ()> {
        Box::pin(async { self.cancel_capture() })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    captured: Arc<Mutex<Vec<i16>>>,
    level: Arc<AtomicU32>,
    max_samples: usize,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> PortResult<Stream>
where
    T: Sample + SizedSample,
    i16: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |input: &[T], _| {
                append_mono_samples(input, channels, &captured, &level, max_samples)
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("failed to build input stream: {error}"))
}

fn append_mono_samples<T>(
    input: &[T],
    channels: usize,
    target: &Arc<Mutex<Vec<i16>>>,
    level: &AtomicU32,
    max_samples: usize,
) where
    T: Sample,
    i16: FromSample<T>,
{
    let Ok(mut output) = target.lock() else {
        return;
    };
    let mut peak = 0i32;

    for frame in input.chunks(channels.max(1)) {
        if output.len() >= max_samples {
            break;
        }
        let sum = frame
            .iter()
            .map(|sample| i32::from(i16::from_sample(*sample)))
            .sum::<i32>();
        let mono = sum / frame.len() as i32;
        peak = peak.max(mono.abs());
        output.push(mono as i16);
    }

    let measured = ((peak as f32 / i16::MAX as f32) * 1_000.0) as u32;
    let previous = level.load(Ordering::Relaxed);
    level.store((previous * 3 + measured) / 4, Ordering::Relaxed);
}

pub(crate) fn resample_mono_pcm16(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    if samples.is_empty() || source_rate == 0 || target_rate == 0 {
        return samples.to_vec();
    }
    if source_rate == target_rate {
        return samples.to_vec();
    }

    let output_len =
        (samples.len() as u64 * u64::from(target_rate) / u64::from(source_rate)) as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = position - left as f64;
            (samples[left] as f64 * (1.0 - fraction) + samples[right] as f64 * fraction).round()
                as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_48khz_audio_to_16khz() {
        let input = (0..48_000).map(|value| value as i16).collect::<Vec<_>>();
        let output = resample_mono_pcm16(&input, 48_000, 16_000);
        assert_eq!(output.len(), 16_000);
        assert_eq!(output[1], input[3]);
    }

    #[test]
    fn resampling_empty_audio_is_safe() {
        assert!(resample_mono_pcm16(&[], 48_000, 16_000).is_empty());
    }
}
