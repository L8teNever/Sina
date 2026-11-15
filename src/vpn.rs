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

// ---------- Mehrere VPN-Configs ----------
//
// Im VPN-Ordner liegen:
//   profiles/<id>.conf   jede hochgeladene Config (normalisiert, Endpunkt als Hostname)
//   profiles.json        Liste, Namen, aktive Config, Auto-Wechsel an/aus
//   wg0.conf             die aktive Config für gluetun (Endpunkt als IP)
// Ein Wächter prüft laufend, ob das VPN Verbindung hat. Fällt es aus (oder wird sehr langsam),
// wechselt Sina automatisch zur nächsten Config, die zuletzt am besten lief.

const PROFILES_DIR: &str = "profiles";
const STATE_FILE: &str = "profiles.json";
/// So lange nach einem Wechsel nicht bewerten (gluetun baut neu auf).
const GRACE: Duration = Duration::from_secs(75);
const CHECK_EVERY: Duration = Duration::from_secs(20);
/// So viele Fehlschläge hintereinander, dann wird gewechselt.
const FAILS_TO_SWITCH: u32 = 3;
/// Langsamer als das gilt als Fehlschlag.
const SLOW_MS: u128 = 6000;

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
struct Profile {
    id: String,
    name: String,
    #[serde(default)]
    added: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct VpnState {
    #[serde(default)]
    list: Vec<Profile>,
    #[serde(default)]
    active: Option<String>,
    #[serde(default = "yes")]
    auto: bool,
}

fn yes() -> bool {
    true
}

impl Default for VpnState {
    fn default() -> Self {
        Self { list: Vec::new(), active: None, auto: true }
    }
}

#[derive(Clone, Default, serde::Serialize)]
struct Health {
    /// "ok", "fail", "slow" oder "" (noch nicht geprüft)
    state: String,
    latency: u64,
    fails: u32,
    ip: String,
    checked: u64,
    /// Fehlschläge insgesamt (für die Wahl der nächsten Config)
    bad: u32,
}

static HEALTH: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Health>>> =
    std::sync::LazyLock::new(Default::default);
static SWITCHED_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 24 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn load_state(dir: &Path) -> VpnState {
    let mut st: VpnState = fs::read_to_string(dir.join(STATE_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    st.list.retain(|p| valid_id(&p.id) && dir.join(PROFILES_DIR).join(format!("{}.conf", p.id)).exists());
    // Alte Einzel-Config übernehmen
    if st.list.is_empty() {
        if let Some(c) = fs::read_to_string(dir.join(SOURCE_FILE)).ok().and_then(|t| parse_conf(&t).ok()) {
            let id = new_id();
            if write_private(&dir.join(PROFILES_DIR), &format!("{id}.conf"), &render(&c, &c.endpoint_host)).is_ok() {
                st.list.push(Profile { id: id.clone(), name: c.endpoint_host.clone(), added: now_secs() });
                st.active = Some(id);
                let _ = save_state(dir, &st);
            }
        }
    }
    if st.active.as_ref().is_some_and(|a| !st.list.iter().any(|p| &p.id == a)) {
        st.active = st.list.first().map(|p| p.id.clone());
    }
    st
}

fn save_state(dir: &Path, st: &VpnState) -> std::io::Result<()> {
    write_private(dir, STATE_FILE, &serde_json::to_string_pretty(st).unwrap_or_default())
}

fn new_id() -> String {
    let n = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:x}", (n as u64) ^ ((n >> 64) as u64))
}

fn load_profile(dir: &Path, id: &str) -> Option<WgConf> {
    fs::read_to_string(dir.join(PROFILES_DIR).join(format!("{id}.conf"))).ok().and_then(|t| parse_conf(&t).ok())
}

/// Namen aus dem Kommentar im [Peer]-Abschnitt (Proton schreibt dort z. B. "# NL-FREE#116").
fn guess_name(text: &str, conf: &WgConf) -> String {
    let mut in_peer = false;
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_peer = t.eq_ignore_ascii_case("[peer]");
            continue;
        }
        if in_peer {
            if let Some(c) = t.strip_prefix('#') {
                let c = c.trim();
                if !c.is_empty() && c.len() <= 40 && !c.contains('=') {
                    return c.to_string();
                }
            }
        }
    }
    conf.endpoint_host.clone()
}

fn clean_name(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(40).collect::<String>().trim().to_string()
}

/// Schreibt die aktive Config für gluetun (Endpunkt als IP). Der Wächter-Container startet gluetun dann neu.
async fn apply(dir: &Path, id: &str) -> Result<(), String> {
    let conf = load_profile(dir, id).ok_or("Config nicht gefunden.")?;
    let ip = resolve(&conf.endpoint_host, conf.endpoint_port).await?;
    write_private(dir, SOURCE_FILE, &render(&conf, &conf.endpoint_host)).map_err(|e| e.to_string())?;
    let wanted = render(&conf, &ip);
    if fs::read_to_string(dir.join(ACTIVE_FILE)).ok().as_deref() != Some(wanted.as_str()) {
        write_private(dir, ACTIVE_FILE, &wanted).map_err(|e| e.to_string())?;
        if let Ok(mut t) = SWITCHED_AT.lock() {
            *t = Some(std::time::Instant::now());
        }
    }
    Ok(())
}

fn io_err(e: impl std::fmt::Display) -> ApiError {
    eprintln!("[vpn] {e}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Speichern fehlgeschlagen (Ordner /vpn nicht beschreibbar?).")
}

#[derive(serde::Deserialize)]
pub struct NameQ {
    name: Option<String>,
}

/// Neue Config hinzufügen. Gibt es noch keine aktive, wird sie gleich benutzt.
pub async fn upload(State(_st): State<Arc<AppState>>, headers: HeaderMap, axum::extract::Query(q): axum::extract::Query<NameQ>, body: String) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    let conf = parse_conf(&body).map_err(ApiError::bad)?;
    resolve(&conf.endpoint_host, conf.endpoint_port).await.map_err(ApiError::bad_gateway)?;
    let _g = LOCK.lock().await;
    let dir = vpn_dir();
    let mut st = load_state(&dir);
    if st.list.len() >= 30 {
        return Err(ApiError::bad("Höchstens 30 VPN-Configs."));
    }
    let id = new_id();
    write_private(&dir.join(PROFILES_DIR), &format!("{id}.conf"), &render(&conf, &conf.endpoint_host)).map_err(io_err)?;
    let name = q.name.as_deref().map(clean_name).filter(|n| !n.is_empty()).unwrap_or_else(|| guess_name(&body, &conf));
    st.list.push(Profile { id: id.clone(), name: name.clone(), added: now_secs() });
    let first = st.active.is_none();
    if first {
        st.active = Some(id.clone());
    }
    save_state(&dir, &st).map_err(io_err)?;
    if first {
        apply(&dir, &id).await.map_err(ApiError::bad_gateway)?;
    }
    Ok(Json(json!({
        "ok": true,
        "id": id,
        "name": name,
        "endpoint": format!("{}:{}", conf.endpoint_host, conf.endpoint_port),
        "message": if first { "Config gespeichert und aktiviert. Die VPN-Verbindung wird in wenigen Sekunden aufgebaut." } else { "Config hinzugefügt. Sie springt ein, wenn die aktive ausfällt." },
    })))
}

pub async fn status(State(_st): State<Arc<AppState>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    let dir = vpn_dir();
    let st = load_state(&dir);
    let health = HEALTH.lock().map(|m| m.clone()).unwrap_or_default();
    let profiles: Vec<Value> = st
        .list
        .iter()
        .map(|p| {
            let c = load_profile(&dir, &p.id);
            json!({
                "id": p.id,
                "name": p.name,
                "endpoint": c.as_ref().map(|c| format!("{}:{}", c.endpoint_host, c.endpoint_port)).unwrap_or_default(),
                "address": c.as_ref().map(|c| format!("{}/32", c.address)).unwrap_or_default(),
                "active": st.active.as_deref() == Some(p.id.as_str()),
                "health": health.get(&p.id).cloned().unwrap_or_default(),
            })
        })
        .collect();
    let switching = SWITCHED_AT.lock().ok().and_then(|t| *t).is_some_and(|t| t.elapsed() < GRACE);
    Ok(Json(json!({
        "configured": !st.list.is_empty(),
        "proxy": crate::proxy::proxy_active(),
        "auto": st.auto,
        "switching": switching,
        "profiles": profiles,
    })))
}

#[derive(serde::Deserialize)]
pub struct IdQ {
    id: String,
}

pub async fn activate(State(_st): State<Arc<AppState>>, headers: HeaderMap, axum::extract::Query(q): axum::extract::Query<IdQ>) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    if !valid_id(&q.id) {
        return Err(ApiError::bad("Ungültige ID"));
    }
    let _g = LOCK.lock().await;
    let dir = vpn_dir();
    let mut st = load_state(&dir);
    if !st.list.iter().any(|p| p.id == q.id) {
        return Err(ApiError::not_found("Config nicht gefunden."));
    }
    apply(&dir, &q.id).await.map_err(ApiError::bad_gateway)?;
    st.active = Some(q.id.clone());
    save_state(&dir, &st).map_err(io_err)?;
    if let Ok(mut h) = HEALTH.lock() {
        h.entry(q.id).or_default().fails = 0;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(State(_st): State<Arc<AppState>>, headers: HeaderMap, axum::extract::Query(q): axum::extract::Query<IdQ>) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    if !valid_id(&q.id) {
        return Err(ApiError::bad("Ungültige ID"));
    }
    let _g = LOCK.lock().await;
    let dir = vpn_dir();
    let mut st = load_state(&dir);
    st.list.retain(|p| p.id != q.id);
    let _ = fs::remove_file(dir.join(PROFILES_DIR).join(format!("{}.conf", q.id)));
    if st.active.as_deref() == Some(q.id.as_str()) {
        st.active = st.list.first().map(|p| p.id.clone());
        match st.active.clone() {
            Some(next) => apply(&dir, &next).await.map_err(ApiError::bad_gateway)?,
            None => {
                // Keine Config mehr: aktive Datei leeren, damit nichts Altes weiterläuft
                let _ = fs::remove_file(dir.join(SOURCE_FILE));
            }
        }
    }
    save_state(&dir, &st).map_err(io_err)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(serde::Deserialize)]
pub struct AutoQ {
    on: bool,
}

pub async fn set_auto(State(_st): State<Arc<AppState>>, headers: HeaderMap, axum::extract::Query(q): axum::extract::Query<AutoQ>) -> Result<Json<Value>, ApiError> {
    authorized(&headers)?;
    let _g = LOCK.lock().await;
    let dir = vpn_dir();
    let mut st = load_state(&dir);
    st.auto = q.on;
    save_state(&dir, &st).map_err(io_err)?;
    Ok(Json(json!({ "ok": true, "auto": st.auto })))
}

/// Nächste Config: die mit den wenigsten Fehlschlägen, bei Gleichstand die nächste in der Liste.
fn pick_next(st: &VpnState, health: &std::collections::HashMap<String, Health>) -> Option<String> {
    let cur = st.active.as_deref()?;
    let n = st.list.len();
    let start = st.list.iter().position(|p| p.id == cur).unwrap_or(0);
    (1..n)
        .map(|k| &st.list[(start + k) % n])
        .min_by_key(|p| health.get(&p.id).map(|h| h.bad).unwrap_or(0))
        .map(|p| p.id.clone())
}

/// Wächter: prüft das VPN, frischt den Endpunkt auf und wechselt bei Ausfall automatisch.
pub async fn monitor(st: Arc<AppState>) {
    let mut since_refresh = Duration::ZERO;
    loop {
        tokio::time::sleep(CHECK_EVERY).await;
        since_refresh += CHECK_EVERY;
        let dir = vpn_dir();
        let state = load_state(&dir);
        let Some(active) = state.active.clone() else { continue };

        // Hostnamen ändern ihre IP: alle 5 Minuten neu auflösen
        if since_refresh >= Duration::from_secs(300) {
            since_refresh = Duration::ZERO;
            let _g = LOCK.lock().await;
            let _ = apply(&dir, &active).await;
        }
        // Ohne Proxy geht der Server nicht durchs VPN: dann gibt es nichts zu prüfen
        if !crate::proxy::proxy_active() {
            continue;
        }
        let in_grace = SWITCHED_AT.lock().ok().and_then(|t| *t).is_some_and(|t| t.elapsed() < GRACE);

        let t0 = std::time::Instant::now();
        let res = st.raw.get("https://api.ipify.org").timeout(Duration::from_secs(10)).send().await;
        let ms = t0.elapsed().as_millis();
        let ip = match res {
            Ok(r) if r.status().is_success() => r.text().await.ok().map(|t| t.trim().chars().take(45).collect::<String>()),
            _ => None,
        };
        let verdict = match (&ip, ms > SLOW_MS) {
            (Some(_), false) => "ok",
            (Some(_), true) => "slow",
            (None, _) => "fail",
        };
        let fails = {
            let Ok(mut h) = HEALTH.lock() else { continue };
            let e = h.entry(active.clone()).or_default();
            e.checked = now_secs();
            e.latency = ms as u64;
            if let Some(ip) = &ip {
                e.ip = ip.clone();
            }
            if verdict == "ok" {
                e.fails = 0;
                e.state = "ok".into();
            } else if !in_grace {
                e.fails += 1;
                e.bad += 1;
                e.state = verdict.into();
            }
            e.fails
        };
        if in_grace || fails < FAILS_TO_SWITCH || !state.auto || state.list.len() < 2 {
            continue;
        }
        let _g = LOCK.lock().await;
        let mut state = load_state(&dir);
        let health = HEALTH.lock().map(|m| m.clone()).unwrap_or_default();
        let Some(next) = pick_next(&state, &health) else { continue };
        eprintln!("[vpn] {} antwortet nicht ({fails}× {verdict}), wechsle zu {next}", active);
        if apply(&dir, &next).await.is_ok() {
            state.active = Some(next.clone());
            let _ = save_state(&dir, &state);
            if let Ok(mut h) = HEALTH.lock() {
                h.entry(next).or_default().fails = 0;
                if let Some(e) = h.get_mut(&active) {
                    e.fails = 0;
                }
            }
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

    #[test]
    fn picks_least_failing_next() {
        let p = |id: &str| Profile { id: id.into(), name: id.into(), added: 0 };
        let st = VpnState { list: vec![p("a1"), p("b2"), p("c3")], active: Some("a1".into()), auto: true };
        let mut h = std::collections::HashMap::new();
        assert_eq!(pick_next(&st, &h).as_deref(), Some("b2"));
        h.insert("b2".to_string(), Health { bad: 5, ..Default::default() });
        assert_eq!(pick_next(&st, &h).as_deref(), Some("c3"));
    }

    #[test]
    fn name_from_proton_comment() {
        let text = format!("[Interface]\nPrivateKey = {}\nAddress = 10.2.0.2/32\n[Peer]\n# NL-FREE#116\nPublicKey = {}\nEndpoint = 1.2.3.4:51820\n", key('A'), key('B'));
        let c = parse_conf(&text).unwrap();
        assert_eq!(guess_name(&text, &c), "NL-FREE#116");
    }
}
