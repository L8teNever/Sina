//! Reicht Streams (HLS-Playlists, Segmente, MP4) und Cover über den Server weiter.
//! Nötig, wenn der Hoster nur mit passendem Referer oder nur von der Server-Adresse aus antwortet.
//! Schutz: es werden nur öffentliche Adressen angesprochen (kein Zugriff aufs interne Netz).

use std::{
    net::IpAddr,
    sync::{Arc, LazyLock},
};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderMap, HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use regex::{Captures, Regex};
use serde::Deserialize;
use url::Url;

use crate::{AppState, UA};

static URI_ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"URI="([^"]+)""#).unwrap());

fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_documentation()
                || o[0] == 0
                || o[0] >= 240
                || (o[0] == 100 && (o[1] & 0xc0) == 64))
        }
        IpAddr::V6(v) => {
            if let Some(m) = v.to_ipv4_mapped() {
                return is_public(IpAddr::V4(m));
            }
            let s0 = v.segments()[0];
            !(v.is_loopback() || v.is_unspecified() || (s0 & 0xfe00) == 0xfc00 || (s0 & 0xffc0) == 0xfe80)
        }
    }
}

/// Prüft, dass die Adresse http(s) ist und nur auf öffentliche IPs zeigt.
/// Ist ein ausgehender Proxy (VPN) eingestellt?
pub fn proxy_active() -> bool {
    ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
        .iter()
        .any(|k| std::env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false))
}

/// Eigener Client für DNS-over-HTTPS (geht wie alles andere über den Proxy aus der Umgebung).
static DOH: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(8)).build().expect("DoH-Client")
});
type DnsCache = std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<IpAddr>)>>;
static DNS_CACHE: LazyLock<DnsCache> = LazyLock::new(Default::default);

async fn doh_lookup(host: &str) -> Result<Vec<IpAddr>, String> {
    if let Some((at, ips)) = DNS_CACHE.lock().ok().and_then(|m| m.get(host).cloned()) {
        if at.elapsed() < std::time::Duration::from_secs(300) {
            return Ok(ips);
        }
    }
    let mut ips = Vec::new();
    for ty in ["A", "AAAA"] {
        let url = format!("https://cloudflare-dns.com/dns-query?name={host}&type={ty}");
        let Ok(r) = DOH.get(&url).header(header::ACCEPT, "application/dns-json").send().await else { continue };
        let Ok(v) = r.json::<serde_json::Value>().await else { continue };
        for a in v["Answer"].as_array().into_iter().flatten() {
            if let Some(ip) = a["data"].as_str().and_then(|d| d.parse::<IpAddr>().ok()) {
                ips.push(ip);
            }
        }
    }
    if ips.is_empty() {
        return Err(format!("Name nicht auflösbar: {host}"));
    }
    if let Ok(mut m) = DNS_CACHE.lock() {
        if m.len() > 500 {
            m.clear();
        }
        m.insert(host.to_string(), (std::time::Instant::now(), ips.clone()));
    }
    Ok(ips)
}

pub async fn check_public(url: &str) -> Result<(), String> {
    let u = Url::parse(url).map_err(|_| "Ungültige Adresse".to_string())?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err("Nur http(s) erlaubt".into());
    }
    let host = u.host_str().ok_or("Adresse ohne Host")?;
    let port = u.port_or_known_default().unwrap_or(443);
    let ips: Vec<IpAddr> = if let Ok(ip) = host.trim_matches(|c| c == '[' || c == ']').parse::<IpAddr>() {
        vec![ip]
    } else if proxy_active() {
        // Mit VPN: Namen über DNS-over-HTTPS durch den Proxy auflösen, damit auch DNS nicht am VPN vorbei geht.
        doh_lookup(host).await?
    } else {
        tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| format!("Name nicht auflösbar: {e}"))?
            .map(|a| a.ip())
            .collect()
    };
    if ips.is_empty() || ips.iter().any(|ip| !is_public(*ip)) {
        return Err("Diese Adresse ist nicht erlaubt".into());
    }
    Ok(())
}

fn plain(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, msg.into()).into_response()
}

fn prox(base: &Url, target: &str, referer: &str) -> String {
    match base.join(target) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => format!(
            "/api/hls?u={}&r={}",
            urlencoding::encode(u.as_str()),
            urlencoding::encode(referer)
        ),
        _ => target.to_string(),
    }
}

/// Alle Adressen in der Playlist so umschreiben, dass sie wieder über /api/hls laufen.
pub fn rewrite_playlist(text: &str, base: &Url, referer: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim();
            if t.is_empty() {
                String::new()
            } else if t.starts_with('#') {
                URI_ATTR
                    .replace_all(t, |c: &Captures| format!("URI=\"{}\"", prox(base, &c[1], referer)))
                    .to_string()
            } else {
                prox(base, t, referer)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize)]
pub struct HlsQ {
    u: String,
    r: Option<String>,
}

pub async fn hls(State(st): State<Arc<AppState>>, headers: HeaderMap, Query(q): Query<HlsQ>) -> Response {
    let referer = q.r.clone().unwrap_or_default();
    let mut url = q.u.clone();
    for _ in 0..5 {
        if let Err(e) = check_public(&url).await {
            return plain(StatusCode::FORBIDDEN, e);
        }
        let mut rq = st.raw.get(&url).header(header::USER_AGENT, UA).header(header::ACCEPT, "*/*");
        if !referer.is_empty() {
            rq = rq.header(header::REFERER, referer.as_str());
            if let Ok(o) = Url::parse(&referer) {
                rq = rq.header(header::ORIGIN, o.origin().ascii_serialization());
            }
        }
        if let Some(range) = headers.get(header::RANGE) {
            rq = rq.header(header::RANGE, range.clone());
        }
        let resp = match rq.send().await {
            Ok(r) => r,
            Err(e) => return plain(StatusCode::BAD_GATEWAY, format!("Stream nicht erreichbar: {e}")),
        };
        if resp.status().is_redirection() {
            let Some(loc) = resp.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()) else {
                return plain(StatusCode::BAD_GATEWAY, "Weiterleitung ohne Ziel");
            };
            let Ok(next) = Url::parse(&url).and_then(|b| b.join(loc)) else {
                return plain(StatusCode::BAD_GATEWAY, "Ungültige Weiterleitung");
            };
            url = next.to_string();
            continue;
        }
        return forward(resp, &url, &referer).await;
    }
    plain(StatusCode::BAD_GATEWAY, "Zu viele Weiterleitungen")
}

async fn forward(resp: reqwest::Response, url: &str, referer: &str) -> Response {
    let status = resp.status();
    let ctype = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let path_is_m3u8 = Url::parse(url).map(|u| u.path().ends_with(".m3u8")).unwrap_or(false);
    let looks_playlist = ctype.contains("mpegurl") || path_is_m3u8;
    let small = resp.content_length().map(|l| l < 2_000_000).unwrap_or(false);
    let maybe = !looks_playlist
        && small
        && (ctype.is_empty() || ctype.starts_with("text/") || ctype.contains("octet-stream"));

    if status.is_success() && (looks_playlist || maybe) {
        let Ok(bytes) = resp.bytes().await else {
            return plain(StatusCode::BAD_GATEWAY, "Antwort abgebrochen");
        };
        let is_list = bytes.iter().find(|b| !b.is_ascii_whitespace()).is_some() && bytes.trim_ascii_start().starts_with(b"#EXTM3U");
        if is_list {
            let text = String::from_utf8_lossy(&bytes).to_string();
            let base = Url::parse(url).unwrap_or_else(|_| Url::parse("https://invalid.invalid/").unwrap());
            let body = rewrite_playlist(&text, &base, referer);
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
                .header(header::CACHE_CONTROL, "no-store")
                .body(Body::from(body))
                .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "Fehler"));
        }
        let mut b = Response::builder().status(status);
        if !ctype.is_empty() {
            b = b.header(header::CONTENT_TYPE, ctype);
        }
        return b
            .body(Body::from(bytes))
            .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "Fehler"));
    }

    let mut b = Response::builder().status(status);
    let names: [HeaderName; 4] = [header::CONTENT_TYPE, header::CONTENT_LENGTH, header::CONTENT_RANGE, header::ACCEPT_RANGES];
    for name in names {
        if let Some(v) = resp.headers().get(&name) {
            b = b.header(name, v.clone());
        }
    }
    b.body(Body::from_stream(resp.bytes_stream()))
        .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "Fehler"))
}

#[derive(Deserialize)]
pub struct ImgQ {
    p: Option<String>,
    u: Option<String>,
}

/// Bilder über den Server (und damit übers VPN): Cover von AniWorld (Pfade unter /public/) oder Bilder von Kinogo.
pub async fn img(State(st): State<Arc<AppState>>, Query(q): Query<ImgQ>) -> Response {
    let (url, referer) = if let Some(p) = q.p.as_deref() {
        if !p.starts_with("/public/") || p.contains("..") || p.len() > 300 {
            return plain(StatusCode::BAD_REQUEST, "Ungültiger Pfad");
        }
        (format!("{}{}", st.base, p), format!("{}/", st.base))
    } else if let Some(u) = q.u.as_deref() {
        let ok = u.len() <= 400
            && Url::parse(u).is_ok_and(|x| x.scheme() == "https" && x.host_str() == Url::parse(crate::kino::BASE).ok().as_ref().and_then(|b| b.host_str()));
        if !ok {
            return plain(StatusCode::BAD_REQUEST, "Ungültige Adresse");
        }
        (u.to_string(), format!("{}/", crate::kino::BASE))
    } else {
        return plain(StatusCode::BAD_REQUEST, "Pfad fehlt");
    };
    let resp = match st.http.get(&url).header(header::USER_AGENT, UA).header(header::REFERER, referer).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return plain(StatusCode::NOT_FOUND, "Kein Bild"),
    };
    let ctype = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    if !ctype.starts_with("image/") {
        return plain(StatusCode::NOT_FOUND, "Kein Bild");
    }
    match resp.bytes().await {
        Ok(b) => Response::builder()
            .header(header::CONTENT_TYPE, ctype)
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(Body::from(b))
            .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "Fehler")),
        Err(_) => plain(StatusCode::BAD_GATEWAY, "Bild abgebrochen"),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ips_are_blocked() {
        for ip in ["127.0.0.1", "10.1.2.3", "192.168.0.5", "172.16.0.1", "169.254.1.1", "100.64.0.1", "0.0.0.0", "::1", "fd00::1", "fe80::1", "::ffff:10.0.0.1"] {
            assert!(!is_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "138.201.31.179", "2606:4700::1111"] {
            assert!(is_public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn playlist_is_rewritten() {
        let base = Url::parse("https://cdn.example/hls/master.m3u8").unwrap();
        let src = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXT-X-STREAM-INF:BANDWIDTH=1\nlow/index.m3u8\n";
        let out = rewrite_playlist(src, &base, "https://voe.example/");
        assert!(out.contains("URI=\"/api/hls?u=https%3A%2F%2Fcdn.example%2Fhls%2Fkey.bin&r=https%3A%2F%2Fvoe.example%2F\""));
        assert!(out.contains("/api/hls?u=https%3A%2F%2Fcdn.example%2Fhls%2Flow%2Findex.m3u8"));
        assert!(out.starts_with("#EXTM3U"));
    }
}
