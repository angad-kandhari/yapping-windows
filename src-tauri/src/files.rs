//! File transcription: decode audio (or a video's audio track) with
//! symphonia, no ffmpeg required, and run it through the engine in
//! chunks so long recordings never hold gigabytes of PCM in memory.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use tauri::{AppHandle, Emitter};

const FLUSH_SECONDS: u64 = 60;

pub fn transcribe(app: AppHandle, path: String) {
    std::thread::spawn(move || {
        let result = run(&app, &path);
        match result {
            Ok(text) => {
                let _ = app.emit("file-done", text);
            }
            Err(e) => {
                let _ = app.emit("file-error", e);
            }
        }
    });
}

fn run(app: &AppHandle, path: &str) -> Result<String, String> {
    if !crate::state::MODEL_READY.load(Ordering::SeqCst) {
        return Err("The speech model is still loading".into());
    }
    let engine = crate::state::engine().ok_or("The speech model is still loading")?;

    let file = std::fs::File::open(path).map_err(|e| format!("Could not open file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension() {
        hint.with_extension(&ext.to_string_lossy());
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|_| "Unsupported or unreadable media format".to_string())?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No audio track found in this file")?;
    let track_id = track.id;
    let total_frames = track.codec_params.n_frames;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|_| "No decoder available for this audio codec".to_string())?;

    let mut transcript = String::new();
    let mut pcm: Vec<f32> = Vec::new();
    let mut channels = 1usize;
    let mut rate = 16_000u32;
    let mut frames_seen = 0u64;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        channels = spec.channels.count().max(1);
        rate = spec.rate;
        frames_seen += decoded.frames() as u64;
        let buf = sample_buf.get_or_insert_with(|| {
            SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
        });
        if buf.capacity() < decoded.frames() * channels {
            *buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        }
        buf.copy_interleaved_ref(decoded);
        pcm.extend_from_slice(buf.samples());

        if pcm.len() as u64 >= FLUSH_SECONDS * rate as u64 * channels as u64 {
            flush(&engine, &mut pcm, channels, rate, &mut transcript);
            if let Some(total) = total_frames {
                let pct = (frames_seen * 100 / total.max(1)).min(99);
                let _ = app.emit("file-progress", pct);
            }
        }
    }
    flush(&engine, &mut pcm, channels, rate, &mut transcript);
    let _ = app.emit("file-progress", 100u64);

    let text = transcript.trim().to_string();
    if text.is_empty() {
        Err("No speech found in this file".into())
    } else {
        Ok(text)
    }
}

fn flush(
    engine: &Arc<Mutex<crate::engine::Engine>>,
    pcm: &mut Vec<f32>,
    channels: usize,
    rate: u32,
    transcript: &mut String,
) {
    if pcm.is_empty() {
        return;
    }
    let mono = yapping_core::resample_to_16k_mono(pcm, channels, rate);
    pcm.clear();
    let text = engine.lock().unwrap().transcribe(&mono);
    let text = text.trim();
    if !text.is_empty() {
        if !transcript.is_empty() {
            transcript.push(' ');
        }
        transcript.push_str(text);
    }
}
