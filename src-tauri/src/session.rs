//! The pipeline thread: model bootstrap, then the hold-to-talk loop.

use tauri::{AppHandle, Emitter};

pub fn set_status(app: &AppHandle, text: &str) {
    use tauri::Manager;
    if let Some(line) = app.try_state::<crate::StatusLine>() {
        let _ = line.0.set_text(text);
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(format!("Yapping: {text}")));
    }
    let _ = app.emit("status", text.to_string());
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(not(windows))]
pub fn run(app: AppHandle) {
    set_status(&app, "This build only dictates on Windows");
}

#[cfg(windows)]
pub fn run(app: AppHandle) {
    if let Err(e) = run_pipeline(&app) {
        set_status(&app, &format!("Error: {e}"));
        notify(&app, "Yapping hit a problem", &e.to_string());
    }
}

#[cfg(windows)]
fn run_pipeline(app: &AppHandle) -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    use yapping_core::{resample_to_16k_mono, Action, Session};

    let ready = "Ready. Hold Ctrl+Win to talk";

    let model_dir = crate::models::ensure(|msg| set_status(app, msg))?;
    set_status(app, "Loading speech model…");
    let engine = crate::engine::Engine::new(&model_dir)?;
    crate::state::set_engine(engine);
    crate::state::MODEL_READY.store(true, Ordering::SeqCst);
    let _ = app.emit("model-ready", true);
    notify(
        app,
        "Yapping is ready",
        "Hold Ctrl+Win and talk. Release to paste. Esc cancels.",
    );
    set_status(app, ready);

    let events = crate::hotkey::spawn();
    let mut session = Session::default();
    let mut recorder: Option<crate::audio::Recorder> = None;

    for event in events {
        match session.handle(event) {
            Action::StartRecording => match crate::audio::Recorder::start() {
                Ok(r) => {
                    recorder = Some(r);
                    set_status(app, "Listening…");
                }
                Err(e) => {
                    session.handle(yapping_core::KeyEvent::Cancelled);
                    set_status(app, &format!("Microphone error: {e}"));
                }
            },
            Action::StopAndTranscribe => {
                if let Some(r) = recorder.take() {
                    set_status(app, "Transcribing…");
                    let (samples, channels, rate) = r.stop();
                    let mono = resample_to_16k_mono(&samples, channels, rate);
                    let raw = match crate::state::engine() {
                        Some(e) => e.lock().unwrap().transcribe(&mono),
                        None => String::new(),
                    };
                    let raw = raw.trim().to_string();
                    if !raw.is_empty() {
                        let settings = crate::config::get();
                        if settings.cleanup != "none" {
                            set_status(app, "Cleaning up…");
                        }
                        let (out, cleaned) = crate::cleanup::apply(&raw);
                        let text = if settings.trailing_space {
                            format!("{out} ")
                        } else {
                            out
                        };
                        let pasted = if settings.copy_only {
                            crate::paste::copy_only(&text)
                        } else {
                            crate::paste::paste(&text)
                        };
                        if let Err(e) = pasted {
                            set_status(app, &format!("Paste failed: {e}"));
                        }
                        crate::history::append(&raw, cleaned.as_deref());
                        let _ = app.emit("history-changed", ());
                    }
                }
                set_status(app, ready);
            }
            Action::StopAndDiscard => {
                if let Some(r) = recorder.take() {
                    let _ = r.stop();
                }
                set_status(app, ready);
            }
            Action::Ignore => {}
        }
    }
    Ok(())
}
