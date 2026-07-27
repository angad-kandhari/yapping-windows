//! First-run model download: Parakeet-TDT 0.6B v2 int8 (CC-BY-4.0) from
//! Hugging Face into %LOCALAPPDATA%\yapping\models. ~640 MB total.

use anyhow::Context;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

const REPO: &str =
    "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/main";
const FILES: [&str; 4] = [
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt",
];

pub fn dir() -> PathBuf {
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".cache")
    };
    base.join("yapping")
        .join("models")
        .join("parakeet-tdt-0.6b-v2-int8")
}

/// Ensures all model files exist, downloading missing ones with progress
/// reported through `status`. Returns the model directory.
pub fn ensure(status: impl Fn(&str)) -> anyhow::Result<PathBuf> {
    let dir = dir();
    fs::create_dir_all(&dir).context("could not create model directory")?;

    let missing: Vec<&str> = FILES
        .iter()
        .copied()
        .filter(|f| {
            dir.join(f)
                .metadata()
                .map(|m| m.len() == 0)
                .unwrap_or(true)
        })
        .collect();
    if missing.is_empty() {
        return Ok(dir);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .context("could not build downloader")?;

    for (i, name) in missing.iter().enumerate() {
        let label = format!("Downloading speech model ({}/{})", i + 1, missing.len());
        status(&format!("{label}: {name}"));

        let mut resp = client
            .get(format!("{REPO}/{name}"))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("download failed for {name}"))?;
        let total = resp.content_length().unwrap_or(0);

        let part = dir.join(format!("{name}.part"));
        let mut out = fs::File::create(&part)
            .with_context(|| format!("could not write {}", part.display()))?;

        let mut done: u64 = 0;
        let mut last_pct: u64 = u64::MAX;
        let mut chunk = [0u8; 1 << 16];
        loop {
            let n = resp.read(&mut chunk).context("download interrupted")?;
            if n == 0 {
                break;
            }
            out.write_all(&chunk[..n]).context("disk write failed")?;
            done += n as u64;
            if let Some(pct) = (done * 100).checked_div(total) {
                if pct != last_pct && pct.is_multiple_of(5) {
                    last_pct = pct;
                    status(&format!("{label}: {name} — {pct}%"));
                }
            }
        }
        out.flush()?;
        drop(out);
        if total > 0 && done != total {
            let _ = fs::remove_file(&part);
            anyhow::bail!("download of {name} ended early ({done} of {total} bytes)");
        }
        fs::rename(&part, dir.join(name))
            .with_context(|| format!("could not finalize {name}"))?;
    }
    Ok(dir)
}
