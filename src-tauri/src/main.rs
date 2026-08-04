// Tray-first app: the pipeline thread owns hotkey -> record -> transcribe
// -> paste; the webview window holds setup, settings, history, listen,
// files, and updates, opened on demand from the tray.
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod audio;
mod cleanup;
mod config;
#[cfg(windows)]
mod engine;
#[cfg(windows)]
mod files;
#[cfg(windows)]
mod hotkey;
#[cfg(windows)]
mod listen;
mod history;
mod models;
#[cfg(windows)]
mod paste;
mod session;
mod state;
mod styles;
mod updates;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Tray menu status line, updatable from the pipeline thread.
pub struct StatusLine(pub MenuItem<tauri::Wry>);
pub struct ListenMenuItem(pub MenuItem<tauri::Wry>);

fn open_window(app: &AppHandle, tab: &str) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("Yapping")
            .inner_size(880.0, 620.0)
            .min_inner_size(680.0, 480.0)
            .build();
    }
    let _ = app.emit("goto-tab", tab.to_string());
}

fn set_listen_label(app: &AppHandle, active: bool) {
    if let Some(item) = app.try_state::<ListenMenuItem>() {
        let _ = item.0.set_text(if active {
            "Stop listening"
        } else {
            "Listen to system audio"
        });
    }
}

// ---- commands ----

#[tauri::command]
fn get_settings() -> config::Settings {
    config::get()
}

#[tauri::command]
fn set_settings(settings: config::Settings) {
    config::set(settings);
}

#[tauri::command]
fn default_prompt() -> String {
    config::DEFAULT_PROMPT.to_string()
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn model_ready() -> bool {
    state::MODEL_READY.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
fn get_history() -> Vec<history::Entry> {
    history::all()
}

#[tauri::command]
fn clear_history() {
    history::clear();
}

#[tauri::command]
async fn check_updates() -> Result<Vec<updates::Release>, String> {
    tauri::async_runtime::spawn_blocking(updates::newer_releases)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) {
    use tauri_plugin_opener::OpenerExt;
    if url.starts_with("https://") {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

#[tauri::command]
fn listen_start(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        set_listen_label(&app, true);
        listen::start(app.clone())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Listen mode needs Windows".into())
    }
}

#[tauri::command]
fn listen_stop(app: AppHandle) {
    #[cfg(windows)]
    listen::stop();
    set_listen_label(&app, false);
}

#[tauri::command]
fn pick_and_transcribe(app: AppHandle) {
    use tauri_plugin_dialog::DialogExt;
    #[cfg(not(windows))]
    {
        let _ = app.emit("file-error", "File transcription needs Windows".to_string());
        return;
    }
    #[cfg(windows)]
    app.dialog()
        .file()
        .add_filter(
            "Audio and video",
            &["wav", "mp3", "m4a", "mp4", "aac", "flac", "ogg", "mov", "webm", "mkv"],
        )
        .pick_file(move |picked| {
            if let Some(path) = picked {
                let _ = app.emit("file-started", path.to_string());
                files::transcribe(app.clone(), path.to_string());
            }
        });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            open_window(app, "settings");
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            default_prompt,
            app_version,
            model_ready,
            get_history,
            clear_history,
            check_updates,
            open_url,
            listen_start,
            listen_stop,
            pick_and_transcribe
        ])
        .setup(|app| {
            let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
            let open = MenuItem::with_id(app, "open", "Open Yapping", true, None::<&str>)?;
            let listen_item = MenuItem::with_id(
                app,
                "listen",
                "Listen to system audio",
                true,
                None::<&str>,
            )?;
            let transcribe =
                MenuItem::with_id(app, "transcribe", "Transcribe a file…", true, None::<&str>)?;
            let update = MenuItem::with_id(app, "updates", "Check for updates", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Yapping", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&status, &open, &listen_item, &transcribe, &update, &quit],
            )?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("bundle icon").clone())
                .menu(&menu)
                .tooltip("Yapping")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => open_window(app, "settings"),
                    "listen" => {
                        use std::sync::atomic::Ordering;
                        if state::LISTEN_ACTIVE.load(Ordering::SeqCst) {
                            #[cfg(windows)]
                            listen::stop();
                            set_listen_label(app, false);
                        } else {
                            open_window(app, "listen");
                        }
                    }
                    "transcribe" => {
                        open_window(app, "files");
                        pick_and_transcribe(app.clone());
                    }
                    "updates" => open_window(app, "updates"),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            app.manage(StatusLine(status));
            app.manage(ListenMenuItem(listen_item));

            let handle = app.handle().clone();
            std::thread::spawn(move || session::run(handle));

            if !config::get().onboarded {
                open_window(app.handle(), "setup");
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Yapping")
        .run(|_app, event| {
            // Closing the window must not kill the tray app; only Quit
            // (which exits with a code) really ends the process.
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
