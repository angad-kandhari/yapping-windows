# yapping for Windows

Windows port of [yapping](https://github.com/angad-kandhari/yapping),
the hold-to-talk dictation app. Private while in development; goes
public when it ships.

Hold **Ctrl+Win**. Talk. Release. Text appears at your cursor.

## Status: Milestone 1 (core loop)

Tray app, Ctrl+Win hold-to-talk, on-device transcription with
Parakeet-TDT 0.6B v2 (int8, via sherpa-onnx), paste at cursor with
clipboard restore. No overlay, cleanup, or settings yet; those are M2.

## Stack

- Tauri v2 + Rust; no Node build (static `dist/` placeholder)
- `core/` is the platform-independent session state machine and
  resampler, unit tested on any OS (`cargo test -p yapping-core`)
- `src-tauri/` holds the Windows pipeline: WH_KEYBOARD_LL hotkey hook,
  cpal capture, sherpa-rs offline transducer decode, SendInput paste
- Model files download on first run (~640 MB) from Hugging Face into
  `%LOCALAPPDATA%\yapping\models`

## Building

CI (GitHub Actions, windows-latest) builds an NSIS installer on every
push; grab `Yapping-Setup` from the run's artifacts. Local build on a
Windows machine: `npx @tauri-apps/cli@2 build`.

## M1 test checklist

1. Installer runs; tray icon appears; launching a second copy does
   nothing (single instance).
2. First run shows download progress in the tray menu/tooltip and posts
   a notification when the model is ready.
3. In Notepad: hold Ctrl+Win, speak a sentence, release. The raw
   transcript pastes at the cursor with a trailing space.
4. Whatever was on the clipboard before dictating is still there after.
5. A quick tap of Ctrl+Win does nothing; Esc mid-hold discards.
6. Quit from the tray menu exits cleanly.
