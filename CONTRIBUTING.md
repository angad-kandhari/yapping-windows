# Contributing to yapping for Windows

Issues and pull requests are welcome. This is the Rust and Tauri port of the
macOS app, so it follows the same rules with a different toolchain.

- Build: `cargo build` from the repository root, or `cargo tauri dev` for the
  app with the webview attached
- Check before pushing: `cargo check`, `cargo test`, and
  `cargo clippy -- -D warnings`. CI runs all three on every pull request.
- Style: match the surrounding code. No em dashes anywhere in code, docs, or
  anything the user reads.
- Privacy is the product. The app may reach the user's configured cleanup
  provider (a local Ollama or an endpoint they entered), GitHub for update
  checks, and Hugging Face once to fetch the speech model. Nothing else. A pull
  request that adds a network destination needs to say so plainly.
- Test the real loop before submitting: hold Ctrl+Win, speak, release, and
  confirm the text lands at the cursor in a normal app.
- Most Win32 code sits behind `#[cfg(windows)]` so the crate still checks on
  other platforms, which keeps contributions possible from a Mac or Linux box.
  Anything touching the overlay or the foreground-window detection has to be
  verified on real Windows before it merges.
