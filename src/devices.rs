//! Geräte: Jedes offene Sina (Fernseher, Mac, PC, Handy, Tablet) meldet sich mit seinem gewählten Profil an (Long-Poll).
//! Die anderen Geräte mit demselben Profil sehen es und können ihm „weiter schauen“ schicken. Alles nur im Arbeitsspeicher, nichts wird gespeichert.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Notify;

use crate::{profiles, ApiError, AppState};

/// So lange gilt ein Gerät als „da“ (der Long-Poll meldet sich alle 20 s).
const ONLINE: Duration = Duration::from_secs(40);
const POLL: Duration = Duration::from_secs(20);
const FORGET: Duration = Duration::from_secs(600);
const MAX_QUEUE: usize = 5;

struct Device {
    name: String,
    kind: &'static str,
    profile: String,
    seen: Instant,
    queue: Vec<Value>,
    state: Option<Value>,
}

#[derive(Default)]
pub struct Devices {
    map: Mutex<HashMap<String, Device>>,
    notify: Notify,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn clean_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

fn clean_name(s: &str) -> String {
    let n = clean_text(s, 40).trim().to_string();
    if n.is_empty() { "Gerät".into() } else { n }
}

/// Prüft einen Befehl von einem anderen Gerät und gibt die bereinigte Form zurück.
pub fn validate_cmd(body: &str) -> Result<Value, String> {
    if body.len() > 4096 {
        return Err("Befehl zu groß.".into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "Kein gültiges JSON.".to_string())?;
    let from = clean_text(v["from"].as_str().unwrap_or(""), 40);
    match v["type"].as_str() {
        Some("open") => {
            let hash = v["hash"].as_str().unwrap_or("");
            if !hash.starts_with("#/") || hash.len() > 300 || hash.chars().any(|c| c.is_control()) {
                return Err("Ungültige Adresse.".into());
            }
            let num = |k: &str| v[k].as_f64().filter(|x| x.is_finite() && *x >= 0.0 && *x < 1e6).unwrap_or(0.0);
            Ok(json!({
                "type": "open",
                "hash": hash,
                "position": num("position"),
                "duration": num("duration"),
                "title": clean_text(v["title"].as_str().unwrap_or(""), 120),
                "from": from,
            }))
        }
        Some("control") => {
            let action = v["action"].as_str().unwrap_or("");
            let value = match action {
                "toggle" | "play" | "pause" | "dynAudio" | "skipIntro" | "skipOutro" | "next" | "prev" | "fullscreen" => Value::Null,
                "seek" => {
                    let x = v["value"].as_f64().filter(|x| x.is_finite() && *x >= 0.0 && *x < 1e6).ok_or("Ungültige Zeit.")?;
                    json!(x)
                }
                "seekBy" => {
                    let x = v["value"].as_f64().filter(|x| x.is_finite() && (-3600.0..=3600.0).contains(x)).ok_or("Ungültiger Sprung.")?;
                    json!(x)
                }
                "speed" => {
                    let x = v["value"].as_f64().filter(|x| x.is_finite() && (0.1..=16.0).contains(x)).ok_or("Ungültige Geschwindigkeit.")?;
                    json!(x)
                }
                "volume" => {
                    let x = v["value"].as_f64().filter(|x| x.is_finite() && (0.0..=6.0).contains(x)).ok_or("Ungültige Lautstärke.")?;
                    json!(x)
                }
                "mute" | "autoNext" => match v["value"].as_bool() {
                    Some(b) => json!(b),
                    None => Value::Null,
                },
                "quality" => match (v["value"].as_str(), v["value"].as_u64()) {
                    (Some("auto"), _) => json!("auto"),
                    (_, Some(h)) if (100..=4320).contains(&h) => json!(h),
                    _ => return Err("Ungültige Qualität.".into()),
                },
                "skip" => match v["value"].as_str() {
                    Some("intro") => json!("intro"),
                    Some("outro") => json!("outro"),
                    _ => Value::Null,
                },
                _ => return Err("Unbekannte Aktion.".into()),
            };
            Ok(json!({
                "type": "control",
                "action": action,
                "value": value,
                "from": from,
            }))
        }
        _ => Err("Unbekannter Befehl.".into()),
    }
}

/// Prüft den gemeldeten Wiedergabe-Stand eines Geräts.
pub fn validate_state(body: &str) -> Result<Option<Value>, String> {
    if body.len() > 2048 {
        return Err("Status zu groß.".into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "Kein gültiges JSON.".to_string())?;
    if !v.is_object() {
        return Err("Erwartet ein Objekt.".into());
    }
    if v["active"].as_bool() == Some(false) {
        return Ok(None);
    }
    let hash = v["hash"].as_str().unwrap_or("");
    if !hash.starts_with("#/") || hash.len() > 300 || hash.chars().any(|c| c.is_control()) {
        return Err("Ungültige Adresse.".into());
    }
    let num = |k: &str, lo: f64, hi: f64, def: f64| {
        v[k].as_f64().filter(|x| x.is_finite() && *x >= lo && *x <= hi).unwrap_or(def)
    };
    let skip_mode = match v["skipMode"].as_str() {
        Some(s @ ("aniskip_op" | "aniskip_ed" | "intro" | "outro")) => json!(s),
        _ => Value::Null,
    };
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Ok(Some(json!({
        "active": true,
        "hash": hash,
        "title": clean_text(v["title"].as_str().unwrap_or(""), 120),
        "epLabel": clean_text(v["epLabel"].as_str().unwrap_or(""), 80),
        "paused": v["paused"].as_bool().unwrap_or(false),
        "pos": num("pos", 0.0, 1e6, 0.0),
        "dur": num("dur", 0.0, 1e6, 0.0),
        "speed": num("speed", 0.1, 16.0, 1.0),
        "volume": num("volume", 0.0, 6.0, 1.0),
        "muted": v["muted"].as_bool().unwrap_or(false),
        "dynAudio": v["dynAudio"].as_bool().unwrap_or(false),
        "autoNext": v["autoNext"].as_bool().unwrap_or(true),
        "skipMode": skip_mode,
        "hasPrev": v["hasPrev"].as_bool().unwrap_or(false),
        "hasNext": v["hasNext"].as_bool().unwrap_or(false),
        "fullscreen": v["fullscreen"].as_bool().unwrap_or(false),
        "quality": match v["quality"].as_str() {
            Some(q) if q == "auto" || q.parse::<u32>().map_or(false, |h| (100..=4320).contains(&h)) => json!(q),
            _ => json!("auto"),
        },
        "qualities": v["qualities"].as_array().map(|a| {
            a.iter().filter_map(|x| x.as_u64()).filter(|h| (100..=4320).contains(h)).take(8).collect::<Vec<_>>()
        }).unwrap_or_default(),
        "at": at,
    })))
}

#[derive(Deserialize)]
pub struct PollQ {
    id: String,
    profile: String,
    name: Option<String>,
    /// tv, phone, tablet oder computer
    kind: Option<String>,
}

fn kind_of(s: Option<&str>) -> &'static str {
    match s {
        Some("tv") => "tv",
        Some("phone") => "phone",
        Some("tablet") => "tablet",
        _ => "computer",
    }
}

/// Geräte melden sich hier an und warten auf Befehle (bis zu 20 s).
pub async fn poll(State(st): State<Arc<AppState>>, Query(q): Query<PollQ>) -> Result<Json<Value>, ApiError> {
    if !valid_id(&q.id) || !profiles::valid(&q.profile) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let name = clean_name(q.name.as_deref().unwrap_or(""));
    let kind = kind_of(q.kind.as_deref());
    let deadline = Instant::now() + POLL;
    loop {
        // erst anmelden, dann prüfen, damit kein Befehl zwischen Prüfung und Warten verloren geht
        let notified = st.devices.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        {
            let mut map = st.devices.map.lock().unwrap_or_else(|e| e.into_inner());
            map.retain(|_, d| d.seen.elapsed() < FORGET);
            let d = map.entry(q.id.clone()).or_insert_with(|| Device {
                name: name.clone(),
                kind,
                profile: q.profile.clone(),
                seen: Instant::now(),
                queue: Vec::new(),
                state: None,
            });
            if d.profile != q.profile {
                d.state = None;
                d.queue.clear();
            }
            d.name = name.clone();
            d.kind = kind;
            d.profile = q.profile.clone();
            d.seen = Instant::now();
            if !d.queue.is_empty() {
                let cmds = std::mem::take(&mut d.queue);
                return Ok(Json(json!({"cmds": cmds})));
            }
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(Json(json!({"cmds": []})));
        }
        let _ = tokio::time::timeout(left, notified).await;
    }
}

#[derive(Deserialize)]
pub struct ListQ {
    profile: String,
    me: Option<String>,
}

/// Geräte, die gerade offen sind und dasselbe Profil gewählt haben (ohne das fragende).
pub async fn list(State(st): State<Arc<AppState>>, Query(q): Query<ListQ>) -> Json<Value> {
    let map = st.devices.map.lock().unwrap_or_else(|e| e.into_inner());
    let mut out: Vec<(&'static str, String, String, Option<Value>)> = map
        .iter()
        .filter(|(id, d)| d.profile == q.profile && d.seen.elapsed() < ONLINE && Some(id.as_str()) != q.me.as_deref())
        .map(|(id, d)| (d.kind, d.name.clone(), id.clone(), d.state.clone()))
        .collect();
    // Fernseher zuerst, dann aktiv spielende Geräte, dann nach Name
    out.sort_by(|a, b| (a.0 != "tv", a.3.is_none(), &a.1).cmp(&(b.0 != "tv", b.3.is_none(), &b.1)));
    Json(Value::Array(
        out.into_iter()
            .map(|(kind, name, id, state)| json!({"id": id, "name": name, "kind": kind, "state": state}))
            .collect(),
    ))
}

/// Ein Gerät meldet seinen aktuellen Wiedergabe-Stand (für die Fernbedienung auf anderen Geräten).
pub async fn update_state(State(st): State<Arc<AppState>>, Query(q): Query<PollQ>, body: String) -> Result<Json<Value>, ApiError> {
    if !valid_id(&q.id) || !profiles::valid(&q.profile) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let state = validate_state(&body).map_err(ApiError::bad)?;
    let name = clean_name(q.name.as_deref().unwrap_or(""));
    let kind = kind_of(q.kind.as_deref());
    {
        let mut map = st.devices.map.lock().unwrap_or_else(|e| e.into_inner());
        let d = map.entry(q.id).or_insert_with(|| Device {
            name: name.clone(),
            kind,
            profile: q.profile.clone(),
            seen: Instant::now(),
            queue: Vec::new(),
            state: None,
        });
        if !name.is_empty() && q.name.is_some() {
            d.name = name;
        }
        if q.kind.is_some() {
            d.kind = kind;
        }
        d.profile = q.profile;
        d.seen = Instant::now();
        d.state = state;
    }
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct SendQ {
    to: String,
    profile: String,
}

pub async fn send(State(st): State<Arc<AppState>>, Query(q): Query<SendQ>, body: String) -> Result<Json<Value>, ApiError> {
    if !valid_id(&q.to) || !profiles::valid(&q.profile) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let cmd = validate_cmd(&body).map_err(ApiError::bad)?;
    {
        let mut map = st.devices.map.lock().unwrap_or_else(|e| e.into_inner());
        let Some(d) = map.get_mut(&q.to).filter(|d| d.seen.elapsed() < ONLINE) else {
            return Err(ApiError::not_found("Das Gerät ist nicht mehr erreichbar."));
        };
        if d.profile != q.profile {
            return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "Das Gerät hat ein anderes Profil gewählt."));
        }
        if d.queue.len() >= MAX_QUEUE {
            d.queue.remove(0);
        }
        d.queue.push(cmd);
    }
    st.devices.notify.notify_waiters();
    Ok(Json(json!({"ok": true})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_open_command() {
        let c = validate_cmd(r##"{"type":"open","hash":"#/anime/stream/x/staffel-1/episode-2","position":120.5,"duration":1400,"title":"X"}"##).unwrap();
        assert_eq!(c["position"], 120.5);
        assert_eq!(c["hash"], "#/anime/stream/x/staffel-1/episode-2");
    }

    #[test]
    fn accepts_control_commands() {
        assert_eq!(validate_cmd(r#"{"type":"control","action":"toggle"}"#).unwrap()["action"], "toggle");
        assert_eq!(validate_cmd(r#"{"type":"control","action":"seekBy","value":30}"#).unwrap()["value"], 30.0);
        assert_eq!(validate_cmd(r#"{"type":"control","action":"speed","value":1.5}"#).unwrap()["value"], 1.5);
        assert_eq!(validate_cmd(r#"{"type":"control","action":"autoNext","value":false}"#).unwrap()["value"], false);
        assert_eq!(validate_cmd(r#"{"type":"control","action":"skip","value":"intro"}"#).unwrap()["value"], "intro");
        assert_eq!(validate_cmd(r#"{"type":"control","action":"quality","value":720}"#).unwrap()["value"], 720);
        assert_eq!(validate_cmd(r#"{"type":"control","action":"quality","value":"auto"}"#).unwrap()["value"], "auto");
        assert!(validate_cmd(r#"{"type":"control","action":"quality","value":"x"}"#).is_err());
    }

    #[test]
    fn rejects_bad_commands() {
        assert!(validate_cmd(r#"{"type":"open","hash":"http://evil"}"#).is_err());
        assert!(validate_cmd(r##"{"type":"eval","hash":"#/x"}"##).is_err());
        assert!(validate_cmd(r#"{"type":"control","action":"hack"}"#).is_err());
        assert!(validate_cmd(r#"{"type":"control","action":"speed","value":99}"#).is_err());
        assert!(validate_cmd(r#"{"type":"control","action":"seekBy","value":99999}"#).is_err());
        assert!(validate_cmd("kaputt").is_err());
        assert!(validate_cmd(&format!(r##"{{"type":"open","hash":"#/{}"}}"##, "a".repeat(400))).is_err());
    }

    #[test]
    fn validates_state() {
        let s = validate_state(r##"{"hash":"#/anime/stream/x/staffel-1/episode-1","title":"X","pos":12.5,"dur":1400,"speed":1.5,"paused":true,"skipMode":"aniskip_op"}"##).unwrap().unwrap();
        assert_eq!(s["speed"], 1.5);
        assert_eq!(s["paused"], true);
        assert_eq!(s["skipMode"], "aniskip_op");
        assert!(validate_state(r#"{"active":false}"#).unwrap().is_none());
        assert!(validate_state(r#"{"hash":"http://evil"}"#).is_err());
    }

    #[test]
    fn negative_position_becomes_zero() {
        let c = validate_cmd(r##"{"type":"open","hash":"#/","position":-5}"##).unwrap();
        assert_eq!(c["position"], 0.0);
    }

    #[test]
    fn kinds_default_to_computer() {
        assert_eq!(kind_of(Some("tv")), "tv");
        assert_eq!(kind_of(Some("quatsch")), "computer");
        assert_eq!(kind_of(None), "computer");
    }

    #[test]
    fn ids_are_checked() {
        assert!(valid_id("abc-123_x"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
