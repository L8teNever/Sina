//! Zusatzquellen: AniSkip (Intro/Outro-Zeiten) und ani.pm (englische Dubs/Subs, weitere Folgen).

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    Json,
};
use regex::Regex;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::LazyLock;

use crate::{ani, voe, ApiError, AppState, UA};

type St = State<Arc<AppState>>;

static SEASON_WORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*\(.*?\)|staffel\s*\d+|season\s*\d+").unwrap());

fn norm(s: &str) -> String {
    s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' }).collect::<String>().trim().to_string()
}

// ---------- AniSkip ----------

#[derive(Deserialize)]
pub struct SkipQ {
    series: String,
    season: Option<u32>,
    episode: u32,
    duration: Option<f64>,
}

pub async fn aniskip(State(st): St, Query(q): Query<SkipQ>) -> Json<Value> {
    let season = q.season.unwrap_or(1).max(1);
    let key = format!("skip:{}:{}:{}", q.series.to_lowercase(), season, q.episode);
    if let Some(v) = st.cache.get(&key, Duration::from_secs(6 * 3600)) {
        return Json(v);
    }
    let out = skip_lookup(&st, &q.series, season, q.episode, q.duration.unwrap_or(0.0)).await.unwrap_or(Value::Null);
    st.cache.put(key, out.clone());
    Json(out)
}

async fn skip_lookup(st: &AppState, series: &str, season: u32, episode: u32, duration: f64) -> Option<Value> {
    let clean = SEASON_WORDS.replace_all(series, "").trim().to_string();
    let mut terms: Vec<String> = Vec::new();
    if season > 1 {
        terms.push(format!("{clean} Season {season}"));
        terms.push(format!("{clean} {season}"));
    }
    terms.push(clean);
    let mut mal: Option<u64> = None;
    for t in terms {
        let body = json!({
            "query": "query ($search: String) { Media(search: $search, type: ANIME) { id idMal } }",
            "variables": {"search": t},
        });
        let Ok(r) = st.http.post("https://graphql.anilist.co").json(&body).timeout(Duration::from_secs(10)).send().await else { continue };
        let Ok(j) = r.json::<Value>().await else { continue };
        if let Some(id) = j["data"]["Media"]["idMal"].as_u64() {
            mal = Some(id);
            break;
        }
    }
    let mal = mal?;
    let url = format!(
        "https://api.aniskip.com/v2/skip-times/{mal}/{episode}?types[]=op&types[]=ed&types[]=mixed-op&types[]=mixed-ed&episodeLength={}",
        duration.round() as i64
    );
    let r = st.http.get(url).timeout(Duration::from_secs(10)).send().await.ok()?;
    let j: Value = r.json().await.ok()?;
    if j["found"].as_bool() != Some(true) {
        return None;
    }
    let mut out = serde_json::Map::new();
    for x in j["results"].as_array()? {
        let (Some(a), Some(b)) = (x["interval"]["startTime"].as_f64(), x["interval"]["endTime"].as_f64()) else { continue };
        let ty = x["skipType"].as_str().unwrap_or("").to_lowercase();
        let k = if ty.contains("op") { "op" } else if ty.contains("ed") { "ed" } else { continue };
        out.insert(k.into(), json!({"startTime": a, "endTime": b}));
    }
    (!out.is_empty()).then(|| Value::Object(out))
}

// ---------- ani.pm ----------

pub fn best_match(items: &[Value], series: &str, season: u32, episode: u64) -> Option<Value> {
    let clean_title = norm(series);
    let mut best: Option<(i32, &Value)> = None;
    for item in items {
        let mut titles: Vec<String> = vec![norm(item["title"].as_str().unwrap_or("")), norm(item["native"].as_str().unwrap_or(""))];
        if let Some(a) = item["providerTitles"].as_array() {
            titles.extend(a.iter().filter_map(Value::as_str).map(norm));
        }
        let kws = |n: u32| vec![format!("season {n}"), format!("{n}nd season"), format!("{n}rd season"), format!("{n}th season"), format!("{n}st season"), format!("part {n}"), format!("s{n}")];
        let has = |n: u32| kws(n).iter().any(|kw| titles.iter().any(|t| t.contains(kw.as_str())));
        let has_marker = has(season);
        let has_other = (1..=9).filter(|s| *s != season).any(has);
        let mut score = 0i32;
        if season > 1 {
            if has_marker {
                score += 50;
            } else if has_other {
                score -= 30;
            }
        } else if !has_other && !has_marker {
            score += 30;
        } else if has_other {
            score -= 40;
        }
        if item["dubCount"].as_u64().unwrap_or(0) >= episode {
            score += 20;
        }
        if item["subCount"].as_u64().unwrap_or(0) >= episode {
            score += 10;
        }
        if titles.iter().any(|t| !t.is_empty() && (t.contains(&clean_title) || clean_title.contains(t.as_str()))) {
            score += 20;
        }
        if best.map(|(s, _)| score > s).unwrap_or(true) {
            best = Some((score, item));
        }
    }
    best.filter(|(s, _)| *s > 0).map(|(_, v)| v.clone())
}

#[derive(Deserialize)]
pub struct CheckQ {
    series: String,
    season: Option<u32>,
    episode: u64,
    /// weitere Titel, mit „|“ getrennt
    alts: Option<String>,
}

pub async fn check(State(st): St, Query(q): Query<CheckQ>) -> Json<Value> {
    let season = q.season.unwrap_or(1).max(1);
    let key = format!("anipm:{}:{}:{}", q.series.to_lowercase(), season, q.episode);
    if let Some(v) = st.cache.get(&key, Duration::from_secs(1800)) {
        return Json(v);
    }
    let mut queries = vec![q.series.clone()];
    queries.extend(q.alts.as_deref().unwrap_or("").split('|').map(|s| s.trim().to_string()).filter(|s| s.len() > 2).take(4));
    let mut found: Option<Value> = None;
    for term in queries {
        let url = format!("https://ani.pm/api/anime/search?q={}", urlencoding::encode(&term));
        let Ok(j) = pm_json(&st, &url).await else { continue };
        if let Some(items) = j["items"].as_array() {
            if let Some(b) = best_match(items, &q.series, season, q.episode) {
                found = Some(b);
                break;
            }
        }
    }
    let out = match found {
        None => json!({"found": false}),
        Some(b) => json!({
            "found": true,
            "animeId": b["id"], "routeId": b["routeId"], "source": b["source"].as_str().unwrap_or("settlar"),
            "title": b["title"],
            "hasDub": b["dubCount"].as_u64().unwrap_or(0) >= q.episode,
            "hasSub": b["subCount"].as_u64().unwrap_or(0) >= q.episode,
        }),
    };
    st.cache.put(key, out.clone());
    Json(out)
}

#[derive(Deserialize)]
pub struct StreamQ {
    id: String,
    #[serde(rename = "routeId")]
    route_id: Option<String>,
    source: Option<String>,
    ep: u64,
    lang: Option<String>,
}

fn safe_key(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// JSON von ani.pm: erst normal per HTTP, bei Sperre (Cloudflare) über den Browser im Container.
async fn pm_json(st: &AppState, url: &str) -> Result<Value, String> {
    if let Ok(j) = get_json(st, url).await {
        return Ok(j);
    }
    let expr = format!("fetch({}).then(r=>r.text())", serde_json::to_string(url).unwrap_or_default());
    let v = st
        .browser
        .eval_after_load("https://ani.pm/", &expr, Duration::from_secs(45))
        .await
        .map_err(|e| e.message().to_string())?;
    serde_json::from_str(v.as_str().unwrap_or("")).map_err(|_| "ani.pm hat keine gültige Antwort geliefert".to_string())
}

async fn get_json(st: &AppState, url: &str) -> Result<Value, String> {
    let r = st.http.get(url).header(USER_AGENT, UA).timeout(Duration::from_secs(15)).send().await.map_err(|e| e.to_string())?;
    let status = r.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    let t = r.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&t).map_err(|_| "Keine JSON-Antwort (Schutzseite?)".to_string())
}

pub async fn stream(State(st): St, Query(q): Query<StreamQ>) -> Result<Json<Value>, ApiError> {
    let lang = if q.lang.as_deref() == Some("sub") { "sub" } else { "dub" };
    let first = q.source.clone().filter(|s| safe_key(s)).unwrap_or_else(|| "settlar".into());
    let keys: Vec<String> = [q.route_id.clone(), Some(q.id.clone())]
        .into_iter()
        .flatten()
        .filter(|k| safe_key(k) && k != "null")
        .collect();
    if keys.is_empty() {
        return Err(ApiError::bad("Ungültige ani.pm-ID"));
    }
    let cache_key = format!("anipm-stream:{}:{}:{}", keys.join(","), q.ep, lang);
    if let Some(v) = st.cache.get(&cache_key, Duration::from_secs(300)) {
        return Ok(Json(v));
    }

    // 1. Bootstrap (probiert routeId/id auf den Pfaden der Quellen)
    let mut boot: Option<Value> = None;
    'outer: for key in &keys {
        for src in [first.as_str(), "settlar", "anilist"] {
            let url = format!("https://ani.pm/api/anime/playback-bootstrap/{src}/{}?ep={}&lang={lang}", urlencoding::encode(key), q.ep);
            if let Ok(j) = pm_json(&st, &url).await {
                boot = Some(j);
                break 'outer;
            }
        }
    }
    let boot = boot.ok_or_else(|| ApiError::not_found(format!("Folge {} ist bei ani.pm nicht verfügbar ({})", q.ep, if lang == "dub" { "Dub" } else { "Sub" })))?;
    let selection = boot["settlarSelection"].as_str().ok_or_else(|| ApiError::not_found("ani.pm hat keine Auswahl für diese Folge"))?;

    // 2. Session (hinter Cloudflare: notfalls über den Browser)
    let sess_url = format!(
        "https://ani.pm/api/anime/settlar/session?selection={}&provider=anipm&ep={}&channel={lang}&telemetry=0",
        urlencoding::encode(selection),
        q.ep
    );
    let sess = pm_json(&st, &sess_url).await.map_err(|e| ApiError::bad_gateway(format!("ani.pm: {e}")))?;
    let embed = sess["embedUrl"].as_str().ok_or_else(|| ApiError::bad_gateway("Keine Embed-Adresse von ani.pm erhalten"))?.to_string();

    // 3. Aus der Embed-Seite die Stream-Adresse holen
    let mut media: Option<Value> = None;
    if let Ok((url, body)) = ani::resolve_plain(&st, &embed).await {
        if let Some(mut s) = voe::extract(&body) {
            s["referer"] = Value::String(ani::origin_of(&url));
            media = Some(s);
        }
    }
    if media.is_none() {
        let found = st.browser.find_media(&embed, Duration::from_secs(45)).await?;
        media = Some(json!({
            "title": "ani.pm", "m3u8": found.m3u8, "mp4": found.mp4, "thumbnail": null, "tracks": [],
            "referer": ani::origin_of(&found.url),
        }));
    }
    let mut out = media.unwrap_or(Value::Null);
    if out["m3u8"].is_null() && out["mp4"].is_null() {
        return Err(ApiError::bad_gateway("In der ani.pm-Seite habe ich keinen Stream gefunden."));
    }
    // Intro/Outro-Zeiten kommen gleich mit (Format wie bei AniSkip)
    let mut skip = serde_json::Map::new();
    for (k, v) in [("op", &boot["skip"]["op"]), ("ed", &boot["skip"]["ed"])] {
        if let (Some(a), Some(b)) = (v["start"].as_f64(), v["end"].as_f64()) {
            skip.insert(k.into(), json!({"startTime": a, "endTime": b}));
        }
    }
    out["skip"] = if skip.is_empty() { Value::Null } else { Value::Object(skip) };
    out["hosterUrl"] = Value::String(embed);
    st.cache.put(cache_key, out.clone());
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_matching_season() {
        let items: Vec<Value> = serde_json::from_str(
            r#"[{"id":1,"title":"Show","native":"","dubCount":12,"subCount":12},
                {"id":2,"title":"Show 2nd Season","native":"","dubCount":12,"subCount":12}]"#,
        )
        .unwrap();
        assert_eq!(best_match(&items, "Show", 1, 3).unwrap()["id"], 1);
        assert_eq!(best_match(&items, "Show", 2, 3).unwrap()["id"], 2);
    }
}
