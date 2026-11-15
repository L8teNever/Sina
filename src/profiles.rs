//! Profile wie bei Netflix: Simon, Melanie, Eltern, Martin und Gast. Jedes Profil hat seinen eigenen Stand
//! (Weiter schauen, Fortschritt, gesehen, Sprache, Abspielposition). Der Stand liegt auf dem Server
//! (`/data/profiles/<name>.json`), damit er auf allen Geräten gleich ist. Gast wird nie gespeichert.

use std::{env, fs, io::Write, path::PathBuf, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::{ApiError, AppState};

/// (id, Name, Farbton)
const PROFILES: [(&str, &str, u16); 5] = [
    ("simon", "Simon", 186),
    ("melanie", "Melanie", 330),
    ("eltern", "Eltern", 40),
    ("martin", "Martin", 140),
    ("gast", "Gast", 220),
];

const MAX_BODY: usize = 512 * 1024;
const MAX_KEYS: usize = 3000;
const MAX_KEY_LEN: usize = 300;
const MAX_VALUE_LEN: usize = 20_000;

/// Ist das ein bekanntes Profil (auch Gast)?
pub fn valid(id: &str) -> bool {
    PROFILES.iter().any(|(p, _, _)| *p == id)
}

fn dir() -> PathBuf {
    PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "/data".into())).join("profiles")
}

fn saved_profile(id: &str) -> Result<&'static str, ApiError> {
    match PROFILES.iter().find(|(p, _, _)| *p == id) {
        Some((p, _, _)) if *p != "gast" => Ok(p),
        Some(_) => Err(ApiError::bad("Das Gast-Profil wird nicht gespeichert.")),
        None => Err(ApiError::not_found("Unbekanntes Profil")),
    }
}

/// Profilbilder, die man wählen kann (die Grafiken liegen im Frontend). Leer = Anfangsbuchstabe.
pub const AVATARS: [&str; 10] = ["katze", "fuchs", "panda", "pinguin", "eule", "roboter", "astronaut", "geist", "alien", "frosch"];

fn avatars_file() -> PathBuf {
    dir().join("avatars.json")
}

fn load_avatars() -> Map<String, Value> {
    fs::read_to_string(avatars_file())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

pub async fn list() -> Json<Value> {
    let av = load_avatars();
    Json(Value::Array(
        PROFILES
            .iter()
            .map(|(id, name, hue)| {
                let a = av.get(*id).and_then(|v| v.as_str()).filter(|a| AVATARS.contains(a)).unwrap_or("");
                json!({"id": id, "name": name, "hue": hue, "saved": *id != "gast", "avatar": a})
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct AvatarQ {
    p: String,
    a: Option<String>,
}

/// Profilbild setzen (für alle Geräte). `a` leer = Anfangsbuchstabe.
pub async fn set_avatar(Query(q): Query<AvatarQ>) -> Result<Json<Value>, ApiError> {
    if !valid(&q.p) {
        return Err(ApiError::not_found("Unbekanntes Profil"));
    }
    let a = q.a.unwrap_or_default();
    if !a.is_empty() && !AVATARS.contains(&a.as_str()) {
        return Err(ApiError::bad("Unbekanntes Profilbild"));
    }
    let mut av = load_avatars();
    if a.is_empty() {
        av.remove(&q.p);
    } else {
        av.insert(q.p.clone(), Value::String(a.clone()));
    }
    let d = dir();
    fs::create_dir_all(&d)
        .and_then(|_| fs::write(avatars_file(), serde_json::to_string(&av).unwrap_or_default()))
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Speichern fehlgeschlagen: {e}")))?;
    Ok(Json(json!({ "ok": true, "avatar": a })))
}

#[derive(Deserialize)]
pub struct Q {
    p: String,
}

pub async fn get_state(State(_st): State<Arc<AppState>>, Query(q): Query<Q>) -> Result<Json<Value>, ApiError> {
    let id = saved_profile(&q.p)?;
    let state = fs::read_to_string(dir().join(format!("{id}.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    Ok(Json(state))
}

/// Prüft den Stand: ein Objekt aus Text-Schlüsseln und Text-Werten, mit Größengrenzen.
pub fn validate(body: &str) -> Result<Map<String, Value>, String> {
    if body.len() > MAX_BODY {
        return Err("Der Stand ist zu groß.".into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "Kein gültiges JSON.".to_string())?;
    let Value::Object(map) = v else { return Err("Erwartet ein Objekt.".into()) };
    if map.len() > MAX_KEYS {
        return Err("Zu viele Einträge.".into());
    }
    for (k, v) in &map {
        let ok = k.len() <= MAX_KEY_LEN && v.as_str().map(|s| s.len() <= MAX_VALUE_LEN).unwrap_or(false);
        if !ok {
            return Err("Ungültiger Eintrag.".into());
        }
    }
    Ok(map)
}

pub async fn put_state(State(_st): State<Arc<AppState>>, Query(q): Query<Q>, body: String) -> Result<Json<Value>, ApiError> {
    let id = saved_profile(&q.p)?;
    let map = validate(&body).map_err(ApiError::bad)?;
    let d = dir();
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(&d)?;
        let tmp = d.join(format!(".{id}.tmp"));
        let mut f = fs::File::create(&tmp)?;
        f.write_all(Value::Object(map.clone()).to_string().as_bytes())?;
        f.sync_all()?;
        fs::rename(tmp, d.join(format!("{id}.json")))
    };
    write().map_err(|e| {
        eprintln!("[profile] speichern fehlgeschlagen: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Der Stand konnte nicht gespeichert werden.")
    })?;
    Ok(Json(json!({"ok": true, "keys": map.len()})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_and_unknown_are_not_saved() {
        assert!(saved_profile("simon").is_ok());
        assert!(saved_profile("gast").is_err());
        assert!(saved_profile("../etc").is_err());
    }

    #[test]
    fn validates_state() {
        assert!(validate(r#"{"prog":"{\"a\":1}","lang":"1"}"#).is_ok());
        assert!(validate(r#"[1,2]"#).is_err());
        assert!(validate(r#"{"a":5}"#).is_err());
        assert!(validate("kaputt").is_err());
        let long = format!(r#"{{"a":"{}"}}"#, "x".repeat(MAX_VALUE_LEN + 1));
        assert!(validate(&long).is_err());
    }
}
