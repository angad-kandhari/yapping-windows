// Tray-only app: no window is ever created; the pipeline thread owns the
// hotkey -> record -> transcribe -> paste loop.
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod audio;
#[cfg(windows)]
mod engine;
#[cfg(windows)]
mod hotkey;
mod models;
#[cfg(windows)]
mod paste;
mod session;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

/// Tray menu status line, updatable from the pipeline thread.
pub struct StatusLine(pub MenuItem<tauri::Wry>);

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Yapping", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("bundle icon").clone())
                .menu(&menu)
                .tooltip("Yapping")
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;
            app.manage(StatusLine(status));
            let handle = app.handle().clone();
            std::thread::spawn(move || session::run(handle));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Yapping")
        .run(|_app, event| {
            // No windows exist, so an exit request with no code means the
            // runtime thinks we are done; only Quit (code 0) really exits.
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
