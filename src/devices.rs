//! Geräte: Fernseher melden sich mit ihrem gewählten Profil an (Long-Poll). Das Handy sieht die Fernseher mit
//! demselben Profil und kann ihnen „weiter schauen“ schicken. Alles nur im Arbeitsspeicher, nichts wird gespeichert.

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

/// So lange gilt ein Fernseher als „da“ (der Long-Poll meldet sich alle 20 s).
const ONLINE: Duration = Duration::from_secs(40);
const POLL: Duration = Duration::from_secs(20);
const FORGET: Duration = Duration::from_secs(600);
const MAX_QUEUE: usize = 5;

struct Device {
    name: String,
    profile: String,
    seen: Instant,
    queue: Vec<Value>,
}

#[derive(Default)]
pub struct Devices {
    map: Mutex<HashMap<String, Device>>,
    notify: Notify,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn clean_name(s: &str) -> String {
    let n: String = s.chars().filter(|c| !c.is_control()).take(40).collect();
    let n = n.trim().to_string();
    if n.is_empty() { "Fernseher".into() } else { n }
}

/// Prüft einen Befehl vom Handy und gibt die bereinigte Form zurück.
pub fn validate_cmd(body: &str) -> Result<Value, String> {
    if body.len() > 4096 {
        return Err("Befehl zu groß.".into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "Kein gültiges JSON.".to_string())?;
    if v["type"].as_str() != Some("open") {
        return Err("Unbekannter Befehl.".into());
    }
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
        "title": v["title"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(120).collect::<String>(),
    }))
}

#[derive(Deserialize)]
pub struct PollQ {
    id: String,
    profile: String,
    name: Option<String>,
}

/// Fernseher melden sich hier an und warten auf Befehle (bis zu 20 s).
pub async fn poll(State(st): State<Arc<AppState>>, Query(q): Query<PollQ>) -> Result<Json<Value>, ApiError> {
    if !valid_id(&q.id) || !profiles::valid(&q.profile) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let name = clean_name(q.name.as_deref().unwrap_or(""));
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
                profile: q.profile.clone(),
                seen: Instant::now(),
                queue: Vec::new(),
            });
            d.name = name.clone();
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

/// Fernseher, die gerade offen sind und dasselbe Profil gewählt haben.
pub async fn list(State(st): State<Arc<AppState>>, Query(q): Query<ListQ>) -> Json<Value> {
    let map = st.devices.map.lock().unwrap_or_else(|e| e.into_inner());
    let mut out: Vec<(String, String)> = map
        .iter()
        .filter(|(id, d)| d.profile == q.profile && d.seen.elapsed() < ONLINE && Some(id.as_str()) != q.me.as_deref())
        .map(|(id, d)| (d.name.clone(), id.clone()))
        .collect();
    out.sort();
    Json(Value::Array(out.into_iter().map(|(name, id)| json!({"id": id, "name": name})).collect()))
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
            return Err(ApiError::not_found("Der Fernseher ist nicht mehr erreichbar."));
        };
        if d.profile != q.profile {
            return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "Der Fernseher hat ein anderes Profil gewählt."));
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
    fn rejects_bad_commands() {
        assert!(validate_cmd(r#"{"type":"open","hash":"http://evil"}"#).is_err());
        assert!(validate_cmd(r##"{"type":"eval","hash":"#/x"}"##).is_err());
        assert!(validate_cmd("kaputt").is_err());
        assert!(validate_cmd(&format!(r##"{{"type":"open","hash":"#/{}"}}"##, "a".repeat(400))).is_err());
    }

    #[test]
    fn negative_position_becomes_zero() {
        let c = validate_cmd(r##"{"type":"open","hash":"#/","position":-5}"##).unwrap();
        assert_eq!(c["position"], 0.0);
    }

    #[test]
    fn ids_are_checked() {
        assert!(valid_id("abc-123_x"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
