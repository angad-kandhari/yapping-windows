<p align="center">
  <img src="icon-pack/yapping-icon-1024.png" width="110" alt="yapping icon">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="icon-pack/wordmark/wordmark-dark.png">
    <img src="icon-pack/wordmark/wordmark-light.png" width="240" alt="yapping">
  </picture>
</p>

<p align="center">
  <b>Hold Ctrl+Win. Yap. Done.</b><br>
  Private, on-device dictation for Windows.
</p>

<p align="center">
  <a href="https://get-yapping.com"><b>get-yapping.com</b></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/angad-kandhari/yapping-windows/releases/latest">Download</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-10%2F11%20x64-0078D4" alt="Windows 10/11 x64">
  <img src="https://img.shields.io/badge/Rust-Tauri%202-orange" alt="Rust + Tauri 2">
  <img src="https://img.shields.io/badge/processing-100%25%20local-2ea44f" alt="100% local">
  <img src="https://img.shields.io/github/v/release/angad-kandhari/yapping-windows" alt="latest release">
  <img src="https://img.shields.io/github/license/angad-kandhari/yapping-windows" alt="Apache 2.0">
</p>

---

Hold **Ctrl+Win** anywhere on your PC, speak, release. Your words appear
at the cursor. Nothing leaves your machine. No cloud, no account, no
subscription, no word limits. Dictation should be a key you hold, not an
app you open.

## Status: beta

| | |
|---|---|
| **Hold to talk** | Hold Ctrl+Win anywhere, release, text pastes at the cursor |
| **Setup assistant** | First launch walks through the hotkey, mic access, and the model download |
| **Cleanup** | Optional polish via local Ollama or any OpenAI-compatible endpoint; raw words always win |
| **History** | Last 50 dictations, raw and cleaned side by side, local only |
| **Listen mode** | Transcribes what your PC is playing via loopback capture |
| **Transcribe files** | Audio and video files, decoded locally, no ffmpeg needed |
| **Honest updates** | Checks GitHub only when you click, shows release notes |

The recording overlay and per-app styles from the macOS app are still
on the way.

## Install

[Download Yapping-Setup.exe](https://github.com/angad-kandhari/yapping-windows/releases/latest/download/Yapping-Setup.exe)
and run it. It installs per-user, so no admin prompt.

The installer is not code-signed yet, so Windows SmartScreen will warn
on first run; choose "More info", then "Run anyway". Code signing is
planned. The code is right here if you would rather read it first.

On first launch, yapping downloads its speech model (~640 MB, one time)
into `%LOCALAPPDATA%\yapping\models` with progress in the tray menu, and
notifies you when it is ready.

## Use

1. Put your cursor in any text field.
2. Hold **Ctrl+Win** and speak.
3. Release. The transcript pastes at the cursor.

A quick tap does nothing. Esc while holding cancels the dictation.

## Privacy

- Speech recognition: [Parakeet-TDT 0.6B v2](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2)
  running locally via [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)
- Network connections: one download from Hugging Face on first run for
  the model files; nothing else
- Audio is never written to disk

## Building

Rust workspace, no Node build. `core/` holds the platform-independent
session logic (`cargo test -p yapping-core` runs anywhere); `src-tauri/`
holds the Windows pipeline. Build an installer on Windows with
`npx @tauri-apps/cli@2 build`, or grab the artifact from any green run
on the Actions tab.

## Project

- Website: [get-yapping.com](https://get-yapping.com)
- License: [Apache 2.0](LICENSE)

The speech model is NVIDIA Parakeet-TDT 0.6B v2, published under
CC-BY-4.0, converted to onnx by the sherpa-onnx project.
