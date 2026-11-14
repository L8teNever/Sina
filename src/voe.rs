//! Stream-Adresse aus einer Hoster-Seite holen. VOE versteckt sie in einem verschlüsselten JSON-Block
//! (ROT13 → Trennzeichen raus → Base64 → Bytes −3 → umdrehen → Base64 → JSON). Andere Hoster: direkte .m3u8/.mp4 im Quelltext.

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;

static JSON_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<script[^>]*type="application/json"[^>]*>\s*\["(.*?)"\]\s*</script>"#).unwrap());
static M3U8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^'"\s\\<>]+?\.m3u8[^'"\s\\<>]*"#).unwrap());
static MP4: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^'"\s\\<>]+?\.mp4[^'"\s\\<>]*"#).unwrap());

const DELIMS: [&str; 7] = ["@$", "^^", "~@", "%?", "*~", "!!", "#&"];

fn rot13(c: char) -> char {
    match c {
        'a'..='z' => (((c as u8 - b'a') + 13) % 26 + b'a') as char,
        'A'..='Z' => (((c as u8 - b'A') + 13) % 26 + b'A') as char,
        _ => c,
    }
}

fn b64(s: &str) -> Option<Vec<u8>> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace() && *c != '=').collect();
    STANDARD_NO_PAD.decode(clean).ok()
}

pub fn decode(input: &str) -> Option<Value> {
    let mut s: String = input.chars().map(rot13).collect();
    for d in DELIMS {
        s = s.replace(d, "");
    }
    s = s.replace('_', "");
    let mut bytes = b64(&s)?;
    for b in bytes.iter_mut() {
        *b = b.wrapping_sub(3);
    }
    bytes.reverse();
    let inner = String::from_utf8(bytes).ok()?;
    let json = b64(&inner)?;
    serde_json::from_slice(&json).ok()
}

/// Was die Wiedergabe braucht.
pub fn extract(html: &str) -> Option<Value> {
    if let Some(c) = JSON_BLOCK.captures(html) {
        if let Some(d) = decode(&c[1]) {
            let src = d["source"].as_str().filter(|s| s.starts_with("http"));
            let direct = d["direct_access_url"].as_str().filter(|s| s.starts_with("http"));
            let (mut m3u8, mut mp4) = (None, None);
            if let Some(s) = src {
                if s.contains(".mp4") && !s.contains(".m3u8") {
                    mp4 = Some(s);
                } else {
                    m3u8 = Some(s);
                }
            }
            if mp4.is_none() {
                mp4 = direct;
            }
            if m3u8.is_some() || mp4.is_some() {
                let tracks = d["captions"].as_array().or_else(|| d["tracks"].as_array()).cloned().unwrap_or_default();
                return Some(json!({
                    "title": d["title"].as_str().unwrap_or("Stream"),
                    "m3u8": m3u8,
                    "mp4": mp4,
                    "thumbnail": d["thumbnail"].as_str(),
                    "tracks": tracks,
                }));
            }
        }
    }
    let m3u8 = M3U8.find(html).map(|m| m.as_str().to_string());
    let mp4 = MP4
        .find_iter(html)
        .map(|m| m.as_str())
        .find(|u| !u.contains("test-videos.co.uk") && !u.contains("sample"))
        .map(str::to_string);
    if m3u8.is_some() || mp4.is_some() {
        return Some(json!({"title": "Stream", "m3u8": m3u8, "mp4": mp4, "thumbnail": null, "tracks": []}));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gegenstück zu `decode`, nur für den Test.
    fn encode(v: &Value) -> String {
        let a = STANDARD_NO_PAD.encode(v.to_string());
        let mut bytes: Vec<u8> = a.bytes().rev().collect();
        for b in bytes.iter_mut() {
            *b = b.wrapping_add(3);
        }
        let b = STANDARD_NO_PAD.encode(bytes);
        let mut out = String::new();
        for (i, c) in b.chars().enumerate() {
            out.push(rot13(c));
            if i % 7 == 3 {
                out.push_str(DELIMS[i % DELIMS.len()]);
            }
        }
        out
    }

    #[test]
    fn roundtrip_voe() {
        let v = json!({"title": "Folge 1", "source": "https://cdn.example/hls/abc", "direct_access_url": "https://cdn.example/dl/abc"});
        let html = format!(r#"<html><script type="application/json">["{}"]</script></html>"#, encode(&v));
        let out = extract(&html).expect("gefunden");
        assert_eq!(out["title"], "Folge 1");
        assert_eq!(out["m3u8"], "https://cdn.example/hls/abc");
        assert_eq!(out["mp4"], "https://cdn.example/dl/abc");
    }

    #[test]
    fn plain_m3u8_and_decoy_mp4() {
        let html = r#"var a='https://x.example/v/master.m3u8?t=1'; var b="https://test-videos.co.uk/vids/x.mp4";"#;
        let out = extract(html).expect("gefunden");
        assert_eq!(out["m3u8"], "https://x.example/v/master.m3u8?t=1");
        assert!(out["mp4"].is_null());
    }

    #[test]
    fn nothing_found() {
        assert!(extract("<html>nichts</html>").is_none());
    }
}
