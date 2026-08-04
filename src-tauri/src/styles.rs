//! Per-app dictation styles, ported from the macOS app: which apps a
//! style applies to and how cleanup should write. Styles are fully
//! user-editable; nothing about the rewriting is hidden. On Windows the
//! match runs against the foreground process's executable name
//! ("slack.exe", "Code.exe", ...) instead of a bundle id.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Style {
    pub name: String,
    /// Executable-name substrings, matched case-insensitively against
    /// the foreground process ("slack", "olk", "code.exe", ...).
    pub app_patterns: Vec<String>,
    /// Extra writing instructions appended to the cleanup prompt.
    pub prompt: String,
    /// Verbatim styles skip LLM cleanup entirely (terminals, editors).
    pub verbatim: bool,
}

/// The same four presets the Mac app ships, with the bundle-id patterns
/// swapped for Windows executable names.
pub fn defaults() -> Vec<Style> {
    vec![
        // Dictating prompts to AI assistants: terse, technical, no fluff.
        // (Terminal-based agents are covered by the verbatim Code style.)
        Style {
            name: "Prompt".into(),
            app_patterns: vec!["claude".into(), "chatgpt".into(), "perplexity".into()],
            prompt: "This is a prompt for an AI assistant. Keep it terse and imperative. \
                Preserve technical tokens exactly: file paths, code identifiers, flags, \
                URLs, version numbers. Remove filler but never soften or pad the request."
                .into(),
            verbatim: false,
        },
        Style {
            name: "Casual".into(),
            app_patterns: vec![
                "slack".into(),
                "discord".into(),
                "whatsapp".into(),
                "telegram".into(),
                "signal".into(),
                "messenger".into(),
            ],
            prompt: "Casual chat message: keep it light, contractions are fine, drop \
                trailing periods on short messages, never capitalize for formality's sake."
                .into(),
            verbatim: false,
        },
        Style {
            name: "Formal".into(),
            app_patterns: vec!["outlook".into(), "olk".into(), "thunderbird".into()],
            prompt: "Professional email prose: complete sentences, proper punctuation, \
                courteous tone. Do not add greetings or signoffs that were not spoken."
                .into(),
            verbatim: false,
        },
        Style {
            name: "Code".into(),
            app_patterns: vec![
                "windowsterminal".into(),
                "cmd.exe".into(),
                "powershell".into(),
                "pwsh".into(),
                "conhost".into(),
                "code.exe".into(),
                "cursor".into(),
                "warp".into(),
                "zed".into(),
                "alacritty".into(),
                "wezterm".into(),
            ],
            prompt: String::new(),
            verbatim: true,
        },
    ]
}

/// The style for a foreground executable name, or None for the built-in
/// default behavior.
pub fn matching(styles: &[Style], process: Option<&str>) -> Option<Style> {
    let process = process?.to_lowercase();
    styles
        .iter()
        .find(|style| {
            style
                .app_patterns
                .iter()
                .any(|p| !p.is_empty() && process.contains(&p.to_lowercase()))
        })
        .cloned()
}

/// Executable file name of the foreground window's process ("Code.exe"),
/// captured when the hold starts (the Windows stand-in for the Mac's
/// frontmost-app bundle id).
#[cfg(windows)]
pub fn foreground_process() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false.into(), pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let queried = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        queried.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::{defaults, matching};

    #[test]
    fn matches_case_insensitively() {
        let styles = defaults();
        assert_eq!(
            matching(&styles, Some("Code.exe")).map(|s| s.name),
            Some("Code".to_string())
        );
        assert_eq!(
            matching(&styles, Some("SLACK.EXE")).map(|s| s.name),
            Some("Casual".to_string())
        );
    }

    #[test]
    fn no_process_means_no_style() {
        assert!(matching(&defaults(), None).is_none());
        assert!(matching(&defaults(), Some("notepad.exe")).is_none());
    }

    #[test]
    fn empty_patterns_never_match() {
        let styles = vec![super::Style {
            name: "Broken".into(),
            app_patterns: vec![String::new()],
            prompt: String::new(),
            verbatim: false,
        }];
        assert!(matching(&styles, Some("anything.exe")).is_none());
    }
}
