//! Low-level keyboard hook (WH_KEYBOARD_LL) on its own message-loop
//! thread, tracking the Ctrl+Win chord as a hold. Tauri's global-shortcut
//! plugin cannot do hold semantics, hence the raw hook.
//!
//! Rules learned from the macOS fn-key monitor, adapted:
//! - chord engages when BOTH Ctrl and Win are down, releases when EITHER
//!   goes up (the OS suppresses the Start menu because Ctrl counted as
//!   "another key" during the Win hold)
//! - Esc while the chord is engaged cancels, and we swallow that Esc so
//!   the foreground app never sees it
//! - injected events (our own SendInput Ctrl+V in paste.rs) are ignored
//!   via LLKHF_INJECTED, otherwise pasting would re-trigger the chord

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use yapping_core::KeyEvent;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LWIN, VK_RCONTROL, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();
static CTRL: AtomicBool = AtomicBool::new(false);
static WIN: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn send(event: KeyEvent) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(event);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let injected = kb.flags.0 & LLKHF_INJECTED.0 != 0;
        if !injected {
            let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
            let vk = kb.vkCode as u16;
            match vk {
                v if v == VK_LCONTROL.0 || v == VK_RCONTROL.0 || v == VK_CONTROL.0 => {
                    CTRL.store(down, Ordering::SeqCst)
                }
                v if v == VK_LWIN.0 || v == VK_RWIN.0 => WIN.store(down, Ordering::SeqCst),
                v if v == VK_ESCAPE.0 && down && ACTIVE.load(Ordering::SeqCst) => {
                    send(KeyEvent::Cancelled);
                    return LRESULT(1);
                }
                _ => {}
            }
            let chord = CTRL.load(Ordering::SeqCst) && WIN.load(Ordering::SeqCst);
            let active = ACTIVE.load(Ordering::SeqCst);
            if chord && !active {
                ACTIVE.store(true, Ordering::SeqCst);
                send(KeyEvent::Pressed);
            } else if !chord && active {
                ACTIVE.store(false, Ordering::SeqCst);
                send(KeyEvent::Released);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Installs the hook on a dedicated thread and returns the event stream.
pub fn spawn() -> Receiver<KeyEvent> {
    let (tx, rx) = channel();
    SENDER.set(tx).expect("hotkey::spawn called twice");
    std::thread::spawn(|| unsafe {
        let _hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
            .expect("failed to install keyboard hook");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    rx
}
