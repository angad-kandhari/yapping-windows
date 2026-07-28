//! Shared runtime state. One sherpa engine serves dictation, Listen,
//! and file transcription behind a mutex; decodes are short, so brief
//! contention beats doubling the model's memory footprint.

use std::sync::atomic::AtomicBool;

#[cfg(windows)]
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(windows)]
static ENGINE: OnceLock<Arc<Mutex<crate::engine::Engine>>> = OnceLock::new();

#[cfg(windows)]
pub fn set_engine(engine: crate::engine::Engine) {
    let _ = ENGINE.set(Arc::new(Mutex::new(engine)));
}

#[cfg(windows)]
pub fn engine() -> Option<Arc<Mutex<crate::engine::Engine>>> {
    ENGINE.get().cloned()
}

pub static LISTEN_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static MODEL_READY: AtomicBool = AtomicBool::new(false);
