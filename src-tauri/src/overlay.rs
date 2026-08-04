//! The recording overlay, ported from the macOS WaveformHUD: just the
//! six yapping logo bars floating above the taskbar while you talk. No
//! pill, no background. The window is transparent, click-through,
//! always on top, and never takes focus; the bars breathe on silence
//! and surge with the mic level (overlay.html does the drawing).

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const WIDTH: f64 = 180.0;
const HEIGHT: f64 = 56.0;
const BOTTOM_MARGIN: f64 = 12.0;

/// Bumped on every show and hide; a level-emitter thread exits as soon
/// as its generation goes stale, so rapid re-holds never race.
static GENERATION: AtomicUsize = AtomicUsize::new(0);

/// Show the bars and start feeding them mic levels (call on hold start).
/// `level` is the recorder's live RMS, stored as f32 bits.
pub fn show(app: &AppHandle, level: Arc<AtomicU32>) {
    if !crate::config::get().overlay {
        return;
    }
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    // Window creation belongs on the main thread; the pipeline thread
    // must never block on it.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let window = match handle.get_webview_window("overlay") {
            Some(w) => w,
            None => {
                let builder = WebviewWindowBuilder::new(
                    &handle,
                    "overlay",
                    WebviewUrl::App("overlay.html".into()),
                );
                // transparent() does not exist on macOS builds without the
                // private-api feature; this app only ships on Windows.
                #[cfg(not(target_os = "macos"))]
                let builder = builder.transparent(true);
                let built = builder
                    .title("Yapping")
                    .inner_size(WIDTH, HEIGHT)
                    .decorations(false)
                    .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .focused(false)
                .focusable(false)
                .visible(false)
                .build();
                match built {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("overlay window failed: {e}");
                        return;
                    }
                }
            }
        };
        let _ = window.set_ignore_cursor_events(true);
        position(&window);
        let _ = handle.emit_to("overlay", "overlay-reset", ());
        let _ = window.show();
    });

    let handle = app.clone();
    std::thread::spawn(move || {
        while GENERATION.load(Ordering::SeqCst) == generation {
            let level = f32::from_bits(level.load(Ordering::Relaxed));
            let _ = handle.emit_to("overlay", "mic-level", level);
            std::thread::sleep(Duration::from_millis(33));
        }
    });
}

/// Hide the bars (call on release, cancel, or mic error).
pub fn hide(app: &AppHandle) {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
}

/// Bottom-center of the primary monitor's work area, i.e. just above
/// the taskbar (the Windows stand-in for "above the Dock").
fn position(window: &tauri::WebviewWindow) {
    let monitor = match window.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let width = (WIDTH * scale) as i32;
    let height = (HEIGHT * scale) as i32;
    let x = area.position.x + (area.size.width as i32 - width) / 2;
    let y = area.position.y + area.size.height as i32 - height - (BOTTOM_MARGIN * scale) as i32;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}
