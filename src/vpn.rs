//! Einstellungen: WireGuard-Config (z. B. von Proton VPN) hochladen. Die Datei wird geprüft, alles Nötige
//! (Schlüssel, Adresse, Endpunkt) daraus gelesen und normalisiert in ein geteiltes Verzeichnis geschrieben,
//! das der gluetun-Container liest. Der Private Key wird nie wieder an den Browser zurückgegeben.

use std::{
    env, fs,
    io::Write,
    net::{IpAddr, Ipv4Addr},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::{json, Value};

use crate::{ApiError, AppState};

/// Datei, die gluetun liest (Endpunkt als IP).
const ACTIVE_FILE: &str = "wg0.conf";
/// Gleiche Config, aber mit Hostnamen als Endpunkt, damit die IP nachgezogen werden kann.
const SOURCE_FILE: &str = "wg-source.conf";

#[derive(Debug, PartialEq, Clone)]
pub struct WgConf {
    private_key: String,
    address: Ipv4Addr,
    public_key: String,
    preshared_key: Option<String>,
    endpoint_host: String,
    endpoint_port: u16,
    keepalive: u16,
}

fn vpn_dir() -> PathBuf {
    PathBuf::from(env::var("VPN_DIR").unwrap_or_else(|_| "/vpn".into()))
}

fn check_key(name: &str, v: &str) -> Result<String, String> {
    let v = v.trim();
    let b64 = v.strip_suffix('=').unwrap_or("");
    let ok = v.len() == 44
        && b64.len() == 43
        && b64.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/');
    if ok {
        Ok(v.to_string())
    } else {
        Err(format!(
            "{name} ist ungültig: Ein WireGuard-Key hat genau 44 Zeichen und endet auf \"=\" (hier: {} Zeichen). \
             Wahrscheinlich ist er abgeschnitten. Lade die komplette .conf-Datei hoch.",
            v.chars().count()
        ))
    }
}

/// Liest eine WireGuard-`.conf`. Unbekannte Zeilen (DNS, MTU, IPv6 …) werden ignoriert.
pub fn parse_conf(text: &str) -> Result<WgConf, String> {
    let (mut section, mut private, mut address, mut public) = (String::new(), None, None, None);
    let (mut preshared, mut endpoint, mut keepalive) = (None, None, 25u16);
    let mut peers = 0;

    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_ascii_lowercase();
            if section == "peer" {
                peers += 1;
            }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
        // Nur der erste Peer zählt.
        match (section.as_str(), k.as_str()) {
            ("interface", "privatekey") => private = Some(v.to_string()),
            ("interface", "address") => {
                address = v
                    .split(',')
                    .filter_map(|a| a.trim().split('/').next()?.parse::<Ipv4Addr>().ok())
                    .next();
            }
            ("peer", "publickey") if peers == 1 => public = Some(v.to_string()),
            ("peer", "presharedkey") if peers == 1 => preshared = Some(v.to_string()),
            ("peer", "endpoint") if peers == 1 => endpoint = Some(v.to_string()),
            ("peer", "persistentkeepalive") if peers == 1 => {
                keepalive = v.parse().unwrap_or(25).clamp(1, 600)
            }
            _ => {}
        }
    }

    if peers == 0 {
        return Err("Das sieht nicht nach einer WireGuard-Config aus (Abschnitt [Peer] fehlt).".into());
    }
    let private_key = check_key(
        "PrivateKey",
        private.as_deref().ok_or("PrivateKey fehlt im Abschnitt [Interface].")?,
    )?;
    let public_key = check_key(
        "PublicKey",
        public.as_deref().ok_or("PublicKey fehlt im Abschnitt [Peer].")?,
    )?;
    let preshared_key = match preshared.as_deref() {
        Some(p) => Some(check_key("PresharedKey", p)?),
        None => None,
    };
    let address = address.ok_or("Keine IPv4-Adresse im Abschnitt [Interface] (Address) gefunden.")?;
    let endpoint = endpoint.ok_or("Endpoint fehlt im Abschnitt [Peer].")?;
    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or("Endpoint muss die Form host:port haben.")?;
    let endpoint_port: u16 = port.parse().map_err(|_| "Der Port im Endpoint ist ungültig.".to_string())?;
    let endpoint_host = host.trim_matches(|c| c == '[' || c == ']').to_string();
    let host_ok = !endpoint_host.is_empty()
        && endpoint_host.len() <= 253
        && endpoint_host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':');
    if !host_ok {
        return Err("Der Host im Endpoint ist ungültig.".into());
    }

    Ok(WgConf { private_key, address, public_key, preshared_key, endpoint_host, endpoint_port, keepalive })
}

/// Normalisierte Config; der Endpunkt wird als `endpoint` eingesetzt (Hostname oder IP).
pub fn render(c: &WgConf, endpoint: &str) -> String {
    let mut s = format!("[Interface]\nPrivateKey = {}\nAddress = {}/32\n\n[Peer]\nPublicKey = {}\n", c.private_key, c.address, c.public_key);
    if let Some(p) = &c.preshared_key {
        s.push_str(&format!("PresharedKey = {p}\n"));
    }
    // IPv6-Adressen brauchen eckige Klammern (sonst ist "ip:port" nicht lesbar).
    let endpoint = if endpoint.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("[{endpoint}]")
    } else {
        endpoint.to_string()
    };
    s.push_str(&format!(
        "Endpoint = {endpoint}:{}\nAllowedIPs = 0.0.0.0/0\nPersistentKeepalive = {}\n",
        c.endpoint_port, c.keepalive
    ));
    s
}

/// Hostname -> IPv4 (bevorzugt), damit gluetun immer eine IP bekommt.
async fn resolve(host: &str, port: u16) -> Result<String, String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip.to_string());
    }
    let addrs: Vec<_> = tokio::time::timeout(Duration::from_secs(8), tokio::net::lookup_host((host, port)))
        .await
        .map_err(|_| format!("Die Adresse {host} konnte nicht aufgelöst werden (Zeitüberschreitung)."))?
        .map_err(|_| format!("Die Adresse {host} konnte nicht aufgelöst werden."))?
        .collect();
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or(addrs.first())
        .map(|a| a.ip().to_string())
        .ok_or_else(|| format!("Für {host} wurde keine IP gefunden."))
}

fn write_private(dir: &Path, name: &str, content: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{name}.tmp"));
    let mut f = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
    f.write_all(content.as_bytes())?;
    f.sync_all()?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    fs::rename(tmp, dir.join(name))
}

/// Nur Anfragen vom eigenen Frontend (Kopfzeile `x-sina`). Mit REQUIRE_ACCESS=1 zusätzlich nur über Cloudflare Access.
pub fn authorized(h: &HeaderMap) -> Result<(), ApiError> {
    if h.get("x-sina").is_none() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Anfrage nicht erlaubt."));
    }
    let need_access = env::var("REQUIRE_ACCESS").map(|v| v == "1").unwrap_or(false);
    let has_access = h.contains_key("cf-access-authenticated-user-email") || h.contains_key("cf-access-jwt-assertion");
    if need_access && !has_access {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Nur über den Cloudflare-Zugang erlaubt."));
    }
    Ok(())
}

pub async fn upload(State(_st): State<Arc<AppState>>, headers: HeaderMap, body: String) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    let conf = parse_conf(&body).map_err(ApiError::bad)?;
    let ip = resolve(&conf.endpoint_host, conf.endpoint_port).await.map_err(ApiError::bad_gateway)?;
    let dir = vpn_dir();
    let saved = write_private(&dir, SOURCE_FILE, &render(&conf, &conf.endpoint_host))
        .and_then(|_| write_private(&dir, ACTIVE_FILE, &render(&conf, &ip)));
    if let Err(e) = saved {
        eprintln!("[vpn] speichern in {} fehlgeschlagen: {e}", dir.display());
        return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Die Config konnte nicht gespeichert werden (Ordner /vpn nicht beschreibbar?)."));
    }
    Ok(Json(json!({
        "ok": true,
        "endpoint": format!("{}:{}", conf.endpoint_host, conf.endpoint_port),
        "address": format!("{}/32", conf.address),
        "preshared": conf.preshared_key.is_some(),
        "message": "Config gespeichert. Die VPN-Verbindung wird in wenigen Sekunden neu aufgebaut.",
    })))
}

pub async fn status(State(st): State<Arc<AppState>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    let dir = vpn_dir();
    let Some(conf) = fs::read_to_string(dir.join(SOURCE_FILE)).ok().and_then(|t| parse_conf(&t).ok()) else {
        return Ok(Json(json!({ "configured": false })));
    };
    let updated = fs::metadata(dir.join(ACTIVE_FILE))
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    // Die IP, mit der der Server im Netz gesehen wird (läuft über den Proxy, also durch das VPN).
    let ip = match st.raw.get("https://api.ipify.org").timeout(Duration::from_secs(8)).send().await {
        Ok(r) => r.text().await.ok().map(|t| t.trim().chars().take(45).collect::<String>()),
        Err(_) => None,
    };
    Ok(Json(json!({
        "configured": true,
        "endpoint": format!("{}:{}", conf.endpoint_host, conf.endpoint_port),
        "address": format!("{}/32", conf.address),
        "updated": updated,
        "ip": ip,
    })))
}

/// Löst den Endpunkt regelmäßig neu auf (Hostnamen ändern ihre IP) und schreibt die aktive Config nur bei einer
/// Änderung neu; ein Wächter-Container startet dann gluetun neu.
pub async fn keep_endpoint_fresh() {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;
        let dir = vpn_dir();
        let Ok(text) = fs::read_to_string(dir.join(SOURCE_FILE)) else { continue };
        let Ok(conf) = parse_conf(&text) else { continue };
        let Ok(ip) = resolve(&conf.endpoint_host, conf.endpoint_port).await else { continue };
        let wanted = render(&conf, &ip);
        if fs::read_to_string(dir.join(ACTIVE_FILE)).ok().as_deref() != Some(wanted.as_str()) {
            let _ = write_private(&dir, ACTIVE_FILE, &wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> String {
        format!("{}=", c.to_string().repeat(43))
    }

    fn sample(priv_key: &str) -> String {
        format!(
            "[Interface]\nPrivateKey = {priv_key}\nAddress = 192.168.178.128/24, fd00:a5c7:197f::128/64\nDNS = 192.168.178.1, fritz.box\n\n\
             [Peer]\nPublicKey = {}\nPresharedKey = {}\nAllowedIPs = 192.168.178.0/24, 0.0.0.0/0, ::/0\n\
             Endpoint = abc.myfritz.net:57098\nPersistentKeepalive = 25\n",
            key('B'),
            key('C')
        )
    }

    #[test]
    fn parses_fritzbox_config() {
        let c = parse_conf(&sample(&key('A'))).expect("gültig");
        assert_eq!(c.address, Ipv4Addr::new(192, 168, 178, 128));
        assert_eq!(c.endpoint_host, "abc.myfritz.net");
        assert_eq!(c.endpoint_port, 57098);
        assert_eq!(c.keepalive, 25);
        assert!(c.preshared_key.is_some());
    }

    #[test]
    fn rejects_truncated_key_with_hint() {
        let e = parse_conf(&sample("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")).unwrap_err();
        assert!(e.contains("PrivateKey") && e.contains("44"));
    }

    #[test]
    fn ipv6_endpoint_gets_brackets_and_roundtrips() {
        let c = parse_conf(&sample(&key('A'))).expect("gültig");
        let out = render(&c, "2001:9e8:205e:dbc7:3ea6:2fff:feac:2411");
        assert!(out.contains("Endpoint = [2001:9e8:205e:dbc7:3ea6:2fff:feac:2411]:57098"));
        let back = parse_conf(&out).expect("Rundlauf");
        assert_eq!(back.endpoint_host, "2001:9e8:205e:dbc7:3ea6:2fff:feac:2411");
        assert_eq!(back.endpoint_port, 57098);
    }

    #[test]
    fn parses_proton_config() {
        let text = format!(
            "[Interface]\n# Bouncing = 3\n# NetShield = 0\nPrivateKey = {}\nAddress = 10.2.0.2/32\nDNS = 10.2.0.1\n\n[Peer]\n# NL-FREE#116\nPublicKey = {}\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = 185.159.156.20:51820\n",
            key('A'),
            key('B')
        );
        let c = parse_conf(&text).expect("gültig");
        assert_eq!(c.address, Ipv4Addr::new(10, 2, 0, 2));
        assert_eq!(c.endpoint_host, "185.159.156.20");
        assert_eq!(c.endpoint_port, 51820);
        assert_eq!(c.keepalive, 25);
        assert!(c.preshared_key.is_none());
    }

    #[test]
    fn rejects_non_wireguard_text() {
        assert!(parse_conf("hallo welt").is_err());
        assert!(parse_conf("[Interface]\nPrivateKey = x\n").is_err());
    }

    #[test]
    fn render_is_normalized_and_ipv4_only() {
        let c = parse_conf(&sample(&key('A'))).expect("gültig");
        let out = render(&c, "1.2.3.4");
        assert!(out.contains("Address = 192.168.178.128/32"));
        assert!(out.contains("Endpoint = 1.2.3.4:57098"));
        assert!(out.contains("AllowedIPs = 0.0.0.0/0"));
        assert!(!out.contains("fd00") && !out.contains("DNS"));
        assert_eq!(parse_conf(&out).expect("Rundlauf").endpoint_host, "1.2.3.4");
    }
}
