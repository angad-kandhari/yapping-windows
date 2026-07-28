//! Transcript cleanup, ported from the macOS provider chain: Ollama or
//! any OpenAI-compatible endpoint, with the same guardrails. The raw
//! transcript always wins over a suspicious cleanup.

use serde_json::{json, Value};
use std::time::Duration;

/// Returns (text to insert, cleaned text if cleanup ran and passed).
pub fn apply(raw: &str) -> (String, Option<String>) {
    let settings = crate::config::get();
    let cleaned = match settings.cleanup.as_str() {
        "ollama" => ollama(raw, &settings),
        "custom" => custom(raw, &settings),
        _ => None,
    };
    match cleaned {
        Some(c) => (c.clone(), Some(c)),
        None => (raw.to_string(), None),
    }
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .ok()
}

fn ollama(raw: &str, s: &crate::config::Settings) -> Option<String> {
    let body = json!({
        "model": s.ollama_model,
        "stream": false,
        "messages": [
            {"role": "system", "content": s.prompt},
            {"role": "user", "content": raw}
        ]
    });
    let resp: Value = client()?
        .post(format!("{}/api/chat", s.ollama_url.trim_end_matches('/')))
        .json(&body)
        .send()
        .ok()?
        .json()
        .ok()?;
    let text = resp.get("message")?.get("content")?.as_str()?;
    sanitize(text, raw)
}

fn custom(raw: &str, s: &crate::config::Settings) -> Option<String> {
    if s.custom_url.is_empty() || s.custom_model.is_empty() {
        return None;
    }
    let body = json!({
        "model": s.custom_model,
        "messages": [
            {"role": "system", "content": s.prompt},
            {"role": "user", "content": raw}
        ]
    });
    let mut req = client()?
        .post(format!(
            "{}/chat/completions",
            s.custom_url.trim_end_matches('/')
        ))
        .json(&body);
    if !s.custom_key.is_empty() {
        req = req.bearer_auth(&s.custom_key);
    }
    let resp: Value = req.send().ok()?.json().ok()?;
    let text = resp
        .get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
        .as_str()?;
    sanitize(text, raw)
}

/// The hard-won guards: strip leaked reasoning, strip wrapping quotes,
/// no em dashes, and reject cleanups that shrank or grew implausibly
/// (a wrong-model response is worse than raw text).
fn sanitize(text: &str, raw: &str) -> Option<String> {
    let mut t = text.trim().to_string();
    if let (Some(start), Some(end)) = (t.find("<think>"), t.rfind("</think>")) {
        if start < end {
            t = format!("{}{}", &t[..start], &t[end + 8..]);
        }
    }
    t = t
        .replace(" \u{2014} ", ", ")
        .replace("\u{2014}", ", ")
        .replace(" \u{2013} ", ", ")
        .replace("\u{2013}", "-");
    let t = t
        .trim()
        .trim_matches(|c| c == '"' || c == '`')
        .trim()
        .to_string();
    if t.is_empty() {
        return None;
    }
    let lo = raw.len() as f32 * 0.3;
    let hi = raw.len() as f32 * 1.5 + 40.0;
    if (t.len() as f32) < lo || (t.len() as f32) > hi {
        return None;
    }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn strips_think_blocks() {
        let raw = "hello there this is a decent length sentence for the guard";
        let out = sanitize(
            "<think>reasoning goes here</think>Hello there, this is a decent length sentence for the guard.",
            raw,
        );
        assert_eq!(
            out.as_deref(),
            Some("Hello there, this is a decent length sentence for the guard.")
        );
    }

    #[test]
    fn rejects_too_short() {
        assert!(sanitize("ok", "a long dictation that clearly said much more than that").is_none());
    }

    #[test]
    fn replaces_em_dashes() {
        let raw = "some words some words some words";
        let out = sanitize("some words \u{2014} some words some words", raw).unwrap();
        assert!(!out.contains('\u{2014}'));
    }
}
