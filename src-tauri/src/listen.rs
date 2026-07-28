//! Listen mode: transcribe what the PC is playing, via WASAPI loopback
//! (cpal opens an input stream on the output device). The Windows
//! sibling of the Mac's Core Audio process tap, without extra
//! permissions: loopback capture is just allowed on Windows.
//!
//! The offline engine decodes rolling chunks rather than streaming;
//! text arrives every few seconds, which matches the note-taking use.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const CHUNK_SECONDS: f64 = 7.0;

pub fn start(app: AppHandle) -> Result<(), String> {
    if crate::state::LISTEN_ACTIVE.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let engine = crate::state::engine().ok_or("The speech model is still loading")?;

    std::thread::spawn(move || {
        let result = capture_loop(&app, engine);
        crate::state::LISTEN_ACTIVE.store(false, Ordering::SeqCst);
        let _ = app.emit("listen-state", false);
        if let Err(e) = result {
            let _ = app.emit("listen-error", e);
        }
    });
    Ok(())
}

pub fn stop() {
    crate::state::LISTEN_ACTIVE.store(false, Ordering::SeqCst);
}

fn capture_loop(
    app: &AppHandle,
    engine: Arc<Mutex<crate::engine::Engine>>,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device found")?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("Output device has no usable format: {e}"))?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate().0;

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = buffer.clone();
    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| sink.lock().unwrap().extend_from_slice(data),
            |e| eprintln!("listen stream error: {e}"),
            None,
        )
        .map_err(|e| format!("Could not open loopback capture: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("Could not start loopback capture: {e}"))?;

    let _ = app.emit("listen-state", true);
    let chunk_len = (CHUNK_SECONDS * rate as f64) as usize * channels;

    while crate::state::LISTEN_ACTIVE.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(200));
        let take = {
            let mut b = buffer.lock().unwrap();
            if b.len() >= chunk_len {
                std::mem::take(&mut *b)
            } else {
                Vec::new()
            }
        };
        decode_and_emit(app, &engine, &take, channels, rate);
    }

    drop(stream);
    let rest = std::mem::take(&mut *buffer.lock().unwrap());
    decode_and_emit(app, &engine, &rest, channels, rate);
    Ok(())
}

fn decode_and_emit(
    app: &AppHandle,
    engine: &Arc<Mutex<crate::engine::Engine>>,
    samples: &[f32],
    channels: usize,
    rate: u32,
) {
    if samples.is_empty() {
        return;
    }
    let mono = yapping_core::resample_to_16k_mono(samples, channels, rate);
    // Skip silent chunks so the transcript doesn't fill with noise
    let rms = (mono.iter().map(|s| s * s).sum::<f32>() / mono.len().max(1) as f32).sqrt();
    if rms < 0.003 {
        return;
    }
    let text = engine.lock().unwrap().transcribe(&mono);
    let text = text.trim();
    if !text.is_empty() {
        let _ = app.emit("listen-text", text.to_string());
    }
}
