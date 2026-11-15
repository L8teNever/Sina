//! Kinogo (kinogo.ec): Filmlisten, Suche, Filmseiten und Stream-Adressen.
//! Die Seiten liegen hinter Cloudflare und kommen deshalb meist über den echten Browser.
//! Abgespielt wird über den Player „nextembed“: der liefert eine normale HLS-Playlist (mit mehreren Tonspuren).

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    Json,
};
use regex::Regex;
use reqwest::header::{REFERER, USER_AGENT};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::LazyLock;
use url::Url;

use crate::{ApiError, AppState, UA};

type St = State<Arc<AppState>>;

pub const BASE: &str = "https://kinogo.ec";

const T_LIST: Duration = Duration::from_secs(1800);
const T_FILM: Duration = Duration::from_secs(3600);
const T_STREAM: Duration = Duration::from_secs(600);

static FILM_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/(\d+-[a-z0-9-]+)\.html").unwrap());
static HLS_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?:hls|file)\s*:\s*["'](https://[^"']+\.m3u8[^"']*)["']"#).unwrap());
static ANY_M3U8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https://[^"'\s\\]+/master\.m3u8[^"'\s\\]*"#).unwrap());
static YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\d{4})\)\s*$").unwrap());

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("Selektor")
}

fn text(e: ElementRef) -> String {
    e.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn abs(u: &str) -> String {
    if u.starts_with("http") {
        u.to_string()
    } else if u.starts_with("//") {
        format!("https:{u}")
    } else if u.starts_with('/') {
        format!("{BASE}{u}")
    } else {
        String::new()
    }
}

fn img_of(e: ElementRef) -> String {
    let v = e.value();
    [v.attr("data-src"), v.attr("src")]
        .into_iter()
        .flatten()
        .find(|s| !s.starts_with("data:") && !s.starts_with("./"))
        .map(abs)
        .unwrap_or_default()
}

/// "127609-serdce-zverja"
pub fn valid_id(s: &str) -> bool {
    s.len() <= 120
        && s.split_once('-').is_some_and(|(n, rest)| {
            !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

fn valid_cat(s: &str) -> bool {
    s.len() <= 40 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn split_title(t: &str) -> (String, String) {
    match YEAR.captures(t) {
        Some(c) => (t[..c.get(0).unwrap().start()].trim().to_string(), c[1].to_string()),
        None => (t.trim().to_string(), String::new()),
    }
}

// ---------- Parser ----------

/// Filmkarten einer Liste (Startseite, Kategorie, Suche).
pub fn parse_list(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let mut cards = Vec::new();
    for s in doc.select(&sel(".shortstory")) {
        let Some(a) = s.select(&sel(".shortstory__title a")).next() else { continue };
        let Some(id) = a.value().attr("href").and_then(|h| FILM_ID.captures(h)).map(|c| c[1].to_string()) else { continue };
        let (title, year) = split_title(&text(a));
        let img = s.select(&sel(".shortstory__poster img")).next().map(img_of).unwrap_or_default();
        let mut info = std::collections::HashMap::new();
        for sp in s.select(&sel(".shortstory__info span")) {
            if let Some(b) = sp.select(&sel("b")).next() {
                let k = text(b);
                let v = text(sp).trim_start_matches(k.as_str()).trim().to_string();
                info.insert(k.trim_end_matches(':').to_string(), v);
            }
        }
        let excerpt = s.select(&sel(".excerpt")).next().map(text).unwrap_or_default();
        let kp = s.select(&sel(".film__rating .kp")).next().map(text).unwrap_or_default();
        let imdb = s.select(&sel(".film__rating .imdb")).next().map(text).unwrap_or_default();
        cards.push(json!({
            "id": id,
            "title": title,
            "year": year,
            "img": img,
            "country": info.get("Страна").cloned().unwrap_or_default(),
            "genre": info.get("Жанр").cloned().unwrap_or_default(),
            "duration": info.get("Продолжительность").cloned().unwrap_or_default(),
            "quality": info.get("Качество").cloned().unwrap_or_default(),
            "excerpt": excerpt,
            "kp": kp.trim_start_matches("KP").trim(),
            "imdb": imdb.trim_start_matches("IMDB").trim(),
        }));
    }
    let next = doc.select(&sel(".navigation a, .bot-navigation a, .pagination a")).any(|a| {
        let t = text(a).to_lowercase();
        t.contains("далее") || t.contains("позже") || t.contains("вперед") || t.contains("следующ") || t == "»" || t == ">"
    });
    json!({ "cards": cards, "next": next })
}

/// Eine Filmseite: Daten + Player-Quellen.
pub fn parse_film(html: &str) -> Option<Value> {
    let doc = Html::parse_document(html);
    let h1 = doc.select(&sel(".fullstory__title h1, article h1")).next()?;
    let (title, year) = split_title(&text(h1));
    let ld: Value = doc
        .select(&sel(r#"script[type="application/ld+json"]"#))
        .filter_map(|s| serde_json::from_str::<Value>(&s.text().collect::<String>()).ok())
        .find(|v| v["@type"] == "Movie" || v["@type"] == "TVSeries")
        .unwrap_or(Value::Null);
    let alt = ld["alternateName"].as_str().unwrap_or("").to_string();
    let img = ld["image"].as_str().map(abs).filter(|s| !s.is_empty()).unwrap_or_else(|| {
        doc.select(&sel(".movie_poster img")).next().map(img_of).unwrap_or_default()
    });
    let description = doc
        .select(&sel(".description__block"))
        .next()
        .map(text)
        .filter(|s| !s.is_empty())
        .or_else(|| ld["description"].as_str().map(str::to_string))
        .unwrap_or_default();
    let mut info = serde_json::Map::new();
    for row in doc.select(&sel(".m_info > div")) {
        if let Some(b) = row.select(&sel("b")).next() {
            let k = text(b);
            let v = text(row).trim_start_matches(k.as_str()).trim().to_string();
            if !v.is_empty() {
                info.insert(k.trim_end_matches(':').to_string(), Value::String(v));
            }
        }
    }
    let get = |k: &str| info.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let genres: Vec<String> = ld["genre"].as_array().map(|a| a.iter().filter_map(|g| g.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let names = |k: &str| -> Vec<String> {
        ld[k].as_array().map(|a| a.iter().filter_map(|p| p["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    let rate = |c: &str| doc.select(&sel(&format!(".movie_poster .rate.{c}"))).next().map(text).unwrap_or_default();
    let players: Vec<Value> = doc
        .select(&sel(".video-tabs li[data-src]"))
        .filter_map(|li| {
            let src = abs(li.value().attr("data-src")?);
            let host = Url::parse(&src).ok()?.host_str()?.to_string();
            Some(json!({ "name": text(li), "src": src, "host": host, "ok": host.ends_with("nextembed.ws") }))
        })
        .collect();
    Some(json!({
        "title": title,
        "year": year,
        "alt": alt,
        "img": img,
        "description": description,
        "country": get("Сняли в"),
        "duration": get("Длительность"),
        "quality": get("Лучшее качество"),
        "voices": get("Озвучки для вас"),
        "genres": genres,
        "actors": names("actor"),
        "directors": names("director"),
        "kp": rate("kp").trim_start_matches("КП:").trim(),
        "imdb": rate("imdb").trim_start_matches("ИМДб:").trim(),
        "serial": ld["@type"] == "TVSeries",
        "players": players,
    }))
}

/// HLS-Adresse aus der nextembed-Playerseite.
pub fn extract_nextembed(html: &str) -> Option<String> {
    let raw = HLS_URL
        .captures(html)
        .map(|c| c[1].to_string())
        .or_else(|| ANY_M3U8.find(html).map(|m| m.as_str().to_string()))?;
    Some(raw.replace("\\u0026", "&").replace("\\/", "/"))
}

// ---------- Seiten holen ----------

async fn get_html(st: &AppState, path: &str, marker: &str) -> Result<String, ApiError> {
    let url = format!("{BASE}{path}");
    if let Ok(resp) = st.http.get(&url).header(USER_AGENT, UA).timeout(Duration::from_secs(12)).send().await {
        if resp.status().as_u16() == 404 {
            return Err(ApiError::not_found("Nicht gefunden"));
        }
        if resp.status().is_success() {
            if let Ok(t) = resp.text().await {
                if t.contains(marker) {
                    return Ok(t);
                }
            }
        }
    }
    let l = st.browser.load_html(&url, |l| l.html.contains(marker), Duration::from_secs(50)).await?;
    Ok(l.html)
}

// ---------- Endpunkte ----------

#[derive(Deserialize)]
pub struct ListQ {
    cat: Option<String>,
    page: Option<u32>,
}

pub async fn list(State(st): St, Query(p): Query<ListQ>) -> Result<Json<Value>, ApiError> {
    let cat = p.cat.unwrap_or_default();
    if !valid_cat(&cat) {
        return Err(ApiError::bad("Ungültige Kategorie"));
    }
    let page = p.page.unwrap_or(1).clamp(1, 500);
    let key = format!("kino:list:{cat}:{page}");
    if let Some(v) = st.cache.get(&key, T_LIST) {
        return Ok(Json(v));
    }
    let base = if cat.is_empty() { String::new() } else { format!("/{cat}") };
    let path = if page > 1 { format!("{base}/page/{page}/") } else { format!("{base}/") };
    let html = get_html(&st, &path, "shortstory").await?;
    let v = parse_list(&html);
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct SearchQ {
    q: Option<String>,
}

pub async fn search(State(st): St, Query(p): Query<SearchQ>) -> Result<Json<Value>, ApiError> {
    let q = p.q.unwrap_or_default().trim().to_string();
    if q.chars().count() < 3 || q.len() > 100 {
        return Ok(Json(json!({ "cards": [], "next": false })));
    }
    let key = format!("kino:search:{}", q.to_lowercase());
    if let Some(v) = st.cache.get(&key, T_LIST) {
        return Ok(Json(v));
    }
    let path = format!("/index.php?do=search&subaction=search&story={}", urlencoding(&q));
    let html = get_html(&st, &path, "dle-content").await?;
    let v = parse_list(&html);
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[derive(Deserialize)]
pub struct FilmQ {
    id: String,
}

pub async fn film(State(st): St, Query(p): Query<FilmQ>) -> Result<Json<Value>, ApiError> {
    if !valid_id(&p.id) {
        return Err(ApiError::bad("Ungültige Film-ID"));
    }
    let key = format!("kino:film:{}", p.id);
    if let Some(v) = st.cache.get(&key, T_FILM) {
        return Ok(Json(v));
    }
    let html = get_html(&st, &format!("/{}.html", p.id), "video-tabs").await?;
    let v = parse_film(&html).ok_or_else(|| ApiError::bad_gateway("Die Filmseite konnte ich nicht lesen."))?;
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct StreamQ {
    src: String,
}

pub async fn stream(State(st): St, Query(p): Query<StreamQ>) -> Result<Json<Value>, ApiError> {
    let ok = Url::parse(&p.src)
        .ok()
        .filter(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| h == "nextembed.ws" || h.ends_with(".nextembed.ws")));
    let Some(u) = ok else {
        return Err(ApiError::bad("Dieser Player wird (noch) nicht unterstützt."));
    };
    let key = format!("kino:stream:{}", u);
    if let Some(v) = st.cache.get(&key, T_STREAM) {
        return Ok(Json(v));
    }
    let resp = st
        .raw
        .get(u.as_str())
        .header(USER_AGENT, UA)
        .header(REFERER, format!("{BASE}/"))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| ApiError::bad_gateway(format!("Player nicht erreichbar: {e}")))?;
    if !resp.status().is_success() {
        return Err(ApiError::bad_gateway(format!("Der Player antwortet mit {}.", resp.status().as_u16())));
    }
    let body = resp.text().await.map_err(|_| ApiError::bad_gateway("Leere Antwort vom Player."))?;
    let m3u8 = extract_nextembed(&body).ok_or_else(|| ApiError::bad_gateway("Im Player habe ich keinen Stream gefunden."))?;
    let v = json!({ "m3u8": m3u8, "referer": format!("{}/", u.origin().ascii_serialization()) });
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(valid_id("127609-serdce-zverja"));
        assert!(valid_id("26060--mezhdu-nami-gory"));
        assert!(!valid_id("../etc"));
        assert!(!valid_id("abc-def"));
    }

    #[test]
    fn list_card() {
        let html = r#"<div id="dle-content"><div id="125888" class="shortstory"><div class="shortstory__title">
            <a href="https://kinogo.ec/125888-chelovek-pauk-novyi-den.html"><h2>Человек-паук: Новый день (2026)</h2></a></div>
            <div class="shortstory__poster"><img data-src="/uploads/mini/x.jpg" src="data:image/gif;base64,R0"></div>
            <div class="shortstory__info"><span><b>Страна:</b><a>США</a></span><span><b>Продолжительность:</b>1 ч 30 мин</span></div>
            <div class="excerpt">Text</div><div class="film__rating"><span class="kp">KP 8.0</span><span class="imdb">IMDB 8.1</span></div></div></div>"#;
        let v = parse_list(html);
        let c = &v["cards"][0];
        assert_eq!(c["id"], "125888-chelovek-pauk-novyi-den");
        assert_eq!(c["title"], "Человек-паук: Новый день");
        assert_eq!(c["year"], "2026");
        assert_eq!(c["img"], "https://kinogo.ec/uploads/mini/x.jpg");
        assert_eq!(c["country"], "США");
        assert_eq!(c["kp"], "8.0");
    }

    #[test]
    fn film_page() {
        let html = r#"<script type="application/ld+json">{"@type":"Movie","image":"https://kinogo.ec/p.jpg","alternateName":"Heart of the Beast","genre":["драма"],"actor":[{"name":"Брэд Питт"}]}</script>
            <article><div class="fullstory__title"><h1>Сердце зверя (2026)</h1></div>
            <div class="m_info"><div class="d-flex"><b>Длительность:</b>1 ч 45 мин</div></div>
            <div class="description__block"><div>Описание</div></div>
            <ul class="video-tabs"><li data-src="https://cinemar.cc/embed/1/+x">Смотреть онлайн</li><li data-src="https://api.nextembed.ws/embed/movie/91271">Плеер 1</li></ul></article>"#;
        let v = parse_film(html).unwrap();
        assert_eq!(v["title"], "Сердце зверя");
        assert_eq!(v["alt"], "Heart of the Beast");
        assert_eq!(v["duration"], "1 ч 45 мин");
        assert_eq!(v["players"][1]["ok"], true);
        assert_eq!(v["players"][0]["ok"], false);
    }

    #[test]
    fn nextembed_hls() {
        let html = r#"var a = {hls: "https://x.interkh.com/a/b.mp4/master.m3u8?t=1&x=2", other: 1}"#;
        assert_eq!(extract_nextembed(html).unwrap(), "https://x.interkh.com/a/b.mp4/master.m3u8?t=1&x=2");
    }
}
