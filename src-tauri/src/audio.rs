//! Microphone capture with cpal. Samples accumulate for the whole hold
//! and are decoded once on release (matches the macOS release-to-text UX).

use anyhow::{bail, Context};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    rate: u32,
}

impl Recorder {
    pub fn start() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no microphone found — is one connected and allowed?")?;
        let config = device
            .default_input_config()
            .context("microphone has no usable input format")?;
        let channels = config.channels() as usize;
        let rate = config.sample_rate().0;
        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        let sink = buffer.clone();
        let on_err = |e| eprintln!("audio stream error: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| sink.lock().unwrap().extend_from_slice(data),
                on_err,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    sink.lock()
                        .unwrap()
                        .extend(data.iter().map(|s| *s as f32 / 32768.0))
                },
                on_err,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    sink.lock()
                        .unwrap()
                        .extend(data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0))
                },
                on_err,
                None,
            )?,
            other => bail!("unsupported microphone sample format {other}"),
        };
        stream.play().context("could not start microphone stream")?;

        Ok(Self {
            stream,
            buffer,
            channels,
            rate,
        })
    }

    /// Stops capture and returns (interleaved samples, channels, sample rate).
    pub fn stop(self) -> (Vec<f32>, usize, u32) {
        drop(self.stream);
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        (samples, self.channels, self.rate)
    }
}
