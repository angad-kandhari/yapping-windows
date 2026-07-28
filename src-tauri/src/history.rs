//! Last 50 dictations, raw and cleaned side by side, in a local JSON file
//! the user can inspect and clear. Audio is never stored.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CAP: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub at: String,
    pub raw: String,
    pub cleaned: Option<String>,
}

fn path() -> PathBuf {
    crate::config::app_dir().join("history.json")
}

pub fn all() -> Vec<Entry> {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn append(raw: &str, cleaned: Option<&str>) {
    let mut entries = all();
    entries.insert(
        0,
        Entry {
            at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            raw: raw.to_string(),
            cleaned: cleaned.map(str::to_string),
        },
    );
    entries.truncate(CAP);
    let _ = std::fs::create_dir_all(crate::config::app_dir());
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(path(), json);
    }
}

pub fn clear() {
    let _ = std::fs::remove_file(path());
}
