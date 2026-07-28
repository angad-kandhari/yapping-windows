//! Paste at the cursor: save clipboard, set text, SendInput Ctrl+V,
//! restore. Injected events carry LLKHF_INJECTED so hotkey.rs ignores
//! them. Modifier keyups are injected first in case the user still holds
//! part of the chord when the transcript lands.
//!
//! TODO(M2): exclude dictations from Win+V history and cloud clipboard
//! via the CanIncludeInClipboardHistory / ExcludeClipboardContentFrom-
//! MonitorProcessing formats (arboard has no API for them; needs raw
//! Win32 clipboard formats).

use anyhow::Context;
use std::thread::sleep;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_RWIN, VK_V,
};

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Copy-only mode: stage the text on the clipboard and leave it there.
pub fn copy_only(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("clipboard unavailable")?;
    clipboard
        .set_text(text.to_string())
        .context("could not copy text to the clipboard")?;
    Ok(())
}

pub fn paste(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("clipboard unavailable")?;
    let saved = clipboard.get_text().ok();
    clipboard
        .set_text(text.to_string())
        .context("could not stage text on the clipboard")?;
    sleep(Duration::from_millis(120));

    let sequence = [
        key(VK_LWIN, true),
        key(VK_RWIN, true),
        key(VK_CONTROL, true),
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe { SendInput(&sequence, std::mem::size_of::<INPUT>() as i32) };
    if sent != sequence.len() as u32 {
        anyhow::bail!("paste keystroke was blocked");
    }

    // Give the foreground app time to read the clipboard before restoring.
    sleep(Duration::from_millis(500));
    match saved {
        Some(previous) => {
            let _ = clipboard.set_text(previous);
        }
        None => {
            let _ = clipboard.clear();
        }
    }
    Ok(())
}
