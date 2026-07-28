//! Honest update checking, ported from the macOS Updates window: hits
//! the GitHub releases API only when the user asks, lists releases
//! newer than this build with their notes.

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Serialize, Clone)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub notes: String,
    pub published: String,
    pub url: String,
}

fn parse_ver(tag: &str) -> Option<(u64, u64, u64)> {
    let t = tag.trim_start_matches('v');
    let mut parts = t.splitn(3, '.');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next().unwrap_or("0").parse().ok()?,
    ))
}

pub fn newer_releases() -> Result<Vec<Release>, String> {
    let current = parse_ver(env!("CARGO_PKG_VERSION")).unwrap_or((0, 0, 0));
    let resp: Value = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?
        .get("https://api.github.com/repos/angad-kandhari/yapping-windows/releases")
        .header("User-Agent", "yapping-windows")
        .send()
        .map_err(|_| "Could not reach GitHub. Are you online?".to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    let list = resp.as_array().cloned().unwrap_or_default();
    let mut out: Vec<Release> = list
        .iter()
        .filter_map(|r| {
            let tag = r.get("tag_name")?.as_str()?.to_string();
            if parse_ver(&tag)? <= current || r.get("draft")?.as_bool()? {
                return None;
            }
            Some(Release {
                name: r
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&tag)
                    .to_string(),
                notes: r
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                published: r
                    .get("published_at")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .take(10)
                    .collect(),
                url: r
                    .get("html_url")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                tag,
            })
        })
        .collect();
    out.sort_by(|a, b| parse_ver(&b.tag).cmp(&parse_ver(&a.tag)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse_ver;

    #[test]
    fn version_ordering() {
        assert!(parse_ver("v0.10.0") > parse_ver("v0.2.1"));
        assert!(parse_ver("v1.0.0") > parse_ver("v0.9.9"));
        assert_eq!(parse_ver("v0.2.0"), Some((0, 2, 0)));
    }
}
