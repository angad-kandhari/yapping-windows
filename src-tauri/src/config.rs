//! Persistent settings at %LOCALAPPDATA%\yapping\config.json.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

pub const DEFAULT_PROMPT: &str = "You clean up dictated speech. Fix punctuation and \
capitalization, remove filler words and false starts, and keep the meaning and wording \
otherwise untouched. Never use em dashes. Return only the cleaned text, nothing else.";

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub trailing_space: bool,
    pub copy_only: bool,
    /// "none" | "ollama" | "custom"
    pub cleanup: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub custom_url: String,
    pub custom_key: String,
    pub custom_model: String,
    pub prompt: String,
    /// Per-app dictation styles, chosen by the foreground app at the
    /// moment the hold starts.
    pub styles: Vec<crate::styles::Style>,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trailing_space: true,
            copy_only: false,
            cleanup: "none".into(),
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "gemma3:4b".into(),
            custom_url: String::new(),
            custom_key: String::new(),
            custom_model: String::new(),
            prompt: DEFAULT_PROMPT.into(),
            styles: crate::styles::defaults(),
            onboarded: false,
        }
    }
}

pub fn app_dir() -> PathBuf {
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".cache")
    };
    base.join("yapping")
}

fn config_path() -> PathBuf {
    app_dir().join("config.json")
}

static CURRENT: RwLock<Option<Settings>> = RwLock::new(None);

pub fn get() -> Settings {
    if let Some(s) = CURRENT.read().unwrap().clone() {
        return s;
    }
    let loaded = std::fs::read_to_string(config_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    *CURRENT.write().unwrap() = Some(loaded);
    CURRENT.read().unwrap().clone().unwrap()
}

pub fn set(settings: Settings) {
    let _ = std::fs::create_dir_all(app_dir());
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(config_path(), json);
    }
    *CURRENT.write().unwrap() = Some(settings);
}
