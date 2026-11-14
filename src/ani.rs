//! AniWorld: Suche, Serien, Staffeln, Folgen und Stream-Adressen.
//! Zuerst wird normal per HTTP geholt (schnell). Klappt das nicht (Schutzseite), springt der echte Browser ein.

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    Json,
};
use regex::Regex;
use reqwest::header::{ACCEPT_LANGUAGE, LOCATION, REFERER, USER_AGENT};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::LazyLock;
use url::Url;

use crate::{browser::BrowserError, proxy, voe, ApiError, AppState, UA};

type St = State<Arc<AppState>>;

const T_SEARCH: Duration = Duration::from_secs(600);
const T_SERIES: Duration = Duration::from_secs(1800);
const T_EPISODE: Duration = Duration::from_secs(300);
const T_STREAM: Duration = Duration::from_secs(600);

static JS_REDIRECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"window\.location\.href\s*=\s*['"](https?://[^'"]+)['"]"#).unwrap());
static SEASON_HREF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/(?:staffel-(\d+)|(filme))/?$").unwrap());
static NUM_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)/?$").unwrap());
static REDIRECT_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/redirect/(\d+)").unwrap());

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("Selektor")
}

fn text(e: ElementRef) -> String {
    e.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn valid_slug(s: &str) -> bool {
    !s.is_empty() && s.len() <= 120 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn valid_season(s: &str) -> bool {
    s == "filme" || (!s.is_empty() && s.len() <= 3 && s.chars().all(|c| c.is_ascii_digit()))
}

fn valid_num(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4 && s.chars().all(|c| c.is_ascii_digit())
}

// ---------- Parser (reine Funktionen, gut testbar) ----------

pub fn parse_search(body: &str) -> Vec<Value> {
    let Ok(arr) = serde_json::from_str::<Vec<Value>>(body) else { return vec![] };
    let strip = |h: &str| text(Html::parse_fragment(h).root_element());
    arr.iter()
        .filter_map(|x| {
            let link = x["link"].as_str()?;
            let path = Url::parse("https://x.invalid").ok()?.join(link).ok()?.path().to_string();
            let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
            if parts.len() != 3 || parts[0] != "anime" || parts[1] != "stream" || !valid_slug(parts[2]) {
                return None;
            }
            Some(json!({
                "slug": parts[2],
                "title": strip(x["title"].as_str().unwrap_or("")),
                "description": strip(x["description"].as_str().unwrap_or("")),
            }))
        })
        .collect()
}

pub fn parse_series(html: &str, slug: &str) -> Option<Value> {
    let doc = Html::parse_document(html);
    let h1 = doc.select(&sel(".series-title h1")).next()?;
    let title = h1.select(&sel("span")).next().map(text).unwrap_or_else(|| text(h1));
    let alt: Vec<String> = h1
        .value()
        .attr("data-alternativetitles")
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() > 1 && *s != title)
        .collect();
    let description = doc
        .select(&sel(".seri_des"))
        .next()
        .map(|d| d.value().attr("data-full-description").map(str::to_string).unwrap_or_else(|| text(d)))
        .unwrap_or_default();
    let cover = doc
        .select(&sel(".seriesCoverBox img"))
        .next()
        .and_then(|i| {
            let v = i.value();
            [v.attr("data-src"), v.attr("src")].into_iter().flatten().find(|s| !s.starts_with("data:")).map(str::to_string)
        })
        .unwrap_or_default();
    let genres: Vec<String> = doc.select(&sel(".genres a")).map(text).filter(|g| !g.is_empty()).collect();

    let mut seasons: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for a in doc.select(&sel("#stream ul li a")) {
        let href = a.value().attr("href").unwrap_or("");
        if !href.contains(&format!("/{slug}/")) {
            continue;
        }
        let Some(c) = SEASON_HREF.captures(href) else { continue };
        let key = c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "filme".into());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let label = if key == "filme" { "Filme".to_string() } else { format!("Staffel {key}") };
        seasons.push(json!({"key": key, "label": label}));
    }
    Some(json!({
        "slug": slug,
        "title": title,
        "alt": alt,
        "description": description,
        "cover": cover,
        "genres": genres,
        "seasons": seasons,
    }))
}

pub fn parse_season(html: &str) -> Vec<Value> {
    let doc = Html::parse_document(html);
    let mut out = Vec::new();
    for tr in doc.select(&sel("table.seasonEpisodesList tbody tr")) {
        let Some(a) = tr.select(&sel("a[href*='/episode-'], a[href*='/film-']")).next() else { continue };
        let href = a.value().attr("href").unwrap_or("");
        let Some(n) = NUM_END.captures(href).map(|c| c[1].to_string()) else { continue };
        let title = tr.select(&sel(".seasonEpisodeTitle strong")).next().map(text).unwrap_or_default();
        let title_en = tr.select(&sel(".seasonEpisodeTitle span")).next().map(text).unwrap_or_default();
        let hosters: Vec<String> = tr.select(&sel("i.icon")).filter_map(|i| i.value().attr("title")).map(str::to_string).collect();
        out.push(json!({"n": n.parse::<u32>().unwrap_or(0), "title": title, "titleEn": title_en, "hosters": hosters}));
    }
    out
}

pub fn parse_episode(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let langs: Vec<Value> = doc
        .select(&sel(".changeLanguageBox img[data-lang-key]"))
        .map(|i| {
            let v = i.value();
            let key = v.attr("data-lang-key").unwrap_or("");
            let label = v.attr("title").or_else(|| v.attr("alt")).unwrap_or("Sprache");
            json!({"key": key, "label": label, "on": v.attr("class").unwrap_or("").contains("selectedLanguage")})
        })
        .collect();
    let hosters: Vec<Value> = doc
        .select(&sel("ul.row li[data-link-target]"))
        .filter_map(|li| {
            let v = li.value();
            let id = REDIRECT_ID.captures(v.attr("data-link-target")?)?[1].to_string();
            let name = li.select(&sel("h4")).next().map(text).filter(|s| !s.is_empty()).unwrap_or_else(|| "Hoster".into());
            Some(json!({"id": id, "lang": v.attr("data-lang-key").unwrap_or(""), "name": name}))
        })
        .collect();
    json!({"langs": langs, "hosters": hosters})
}

// ---------- Seiten holen ----------

async fn get_html(st: &AppState, path: &str, marker: &str) -> Result<String, ApiError> {
    let url = format!("{}{}", st.base, path);
    if let Ok(resp) = st.http.get(&url).header(USER_AGENT, UA).timeout(Duration::from_secs(15)).send().await {
        let status = resp.status();
        if status.as_u16() == 404 {
            return Err(ApiError::not_found("Nicht gefunden"));
        }
        if status.is_success() {
            if let Ok(t) = resp.text().await {
                if t.contains(marker) {
                    return Ok(t);
                }
            }
        }
    }
    let l = st
        .browser
        .load_html(&url, |l| l.html.contains(marker), Duration::from_secs(45))
        .await?;
    Ok(l.html)
}

// ---------- Endpunkte ----------

#[derive(Deserialize)]
pub struct SearchQ {
    q: Option<String>,
}

pub async fn search(State(st): St, Query(p): Query<SearchQ>) -> Result<Json<Value>, ApiError> {
    let q = p.q.unwrap_or_default();
    let q = q.trim();
    if q.chars().count() < 2 || q.len() > 80 {
        return Ok(Json(json!([])));
    }
    let key = format!("search:{}", q.to_lowercase());
    if let Some(v) = st.cache.get(&key, T_SEARCH) {
        return Ok(Json(v));
    }
    let mut body: Option<String> = None;
    if let Ok(resp) = st
        .http
        .post(format!("{}/ajax/search", st.base))
        .header(USER_AGENT, UA)
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&[("keyword", q)])
        .timeout(Duration::from_secs(12))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(t) = resp.text().await {
                if t.trim_start().starts_with('[') {
                    body = Some(t);
                }
            }
        }
    }
    let body = match body {
        Some(b) => b,
        None => {
            let expr = format!(
                "fetch('/ajax/search',{{method:'POST',credentials:'include',headers:{{'Content-Type':'application/x-www-form-urlencoded; charset=UTF-8','X-Requested-With':'XMLHttpRequest'}},body:'keyword='+encodeURIComponent({})}}).then(r=>r.text())",
                serde_json::to_string(q).unwrap_or_else(|_| "\"\"".into())
            );
            let v = st
                .browser
                .eval_after_load(&format!("{}/", st.base), &expr, Duration::from_secs(45))
                .await?;
            v.as_str().unwrap_or("[]").to_string()
        }
    };
    let out = Value::Array(parse_search(&body));
    st.cache.put(key, out.clone());
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct SeriesQ {
    slug: String,
}

pub async fn series(State(st): St, Query(p): Query<SeriesQ>) -> Result<Json<Value>, ApiError> {
    if !valid_slug(&p.slug) {
        return Err(ApiError::bad("Ungültiger Name"));
    }
    let key = format!("series:{}", p.slug);
    if let Some(v) = st.cache.get(&key, T_SERIES) {
        return Ok(Json(v));
    }
    let html = get_html(&st, &format!("/anime/stream/{}", p.slug), "series-title").await?;
    let v = parse_series(&html, &p.slug).ok_or_else(|| ApiError::not_found("Serie nicht gefunden"))?;
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct SeasonQ {
    slug: String,
    season: String,
}

pub async fn season(State(st): St, Query(p): Query<SeasonQ>) -> Result<Json<Value>, ApiError> {
    if !valid_slug(&p.slug) || !valid_season(&p.season) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let key = format!("season:{}:{}", p.slug, p.season);
    if let Some(v) = st.cache.get(&key, T_SERIES) {
        return Ok(Json(v));
    }
    let part = if p.season == "filme" { "filme".to_string() } else { format!("staffel-{}", p.season) };
    let html = get_html(&st, &format!("/anime/stream/{}/{}", p.slug, part), "seasonEpisodesList").await?;
    let v = Value::Array(parse_season(&html));
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct EpisodeQ {
    slug: String,
    season: String,
    episode: String,
}

pub async fn episode(State(st): St, Query(p): Query<EpisodeQ>) -> Result<Json<Value>, ApiError> {
    if !valid_slug(&p.slug) || !valid_season(&p.season) || !valid_num(&p.episode) {
        return Err(ApiError::bad("Ungültige Angabe"));
    }
    let key = format!("episode:{}:{}:{}", p.slug, p.season, p.episode);
    if let Some(v) = st.cache.get(&key, T_EPISODE) {
        return Ok(Json(v));
    }
    let part = if p.season == "filme" {
        format!("filme/film-{}", p.episode)
    } else {
        format!("staffel-{}/episode-{}", p.season, p.episode)
    };
    let html = get_html(&st, &format!("/anime/stream/{}/{}", p.slug, part), "hosterSiteVideo").await?;
    let v = parse_episode(&html);
    if v["hosters"].as_array().map(|h| h.is_empty()).unwrap_or(true) {
        return Err(ApiError::not_found("Keine Hoster für diese Folge"));
    }
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct StreamQ {
    id: String,
}

/// Folgt der Weiterleitung des Hosters (auch JavaScript-Weiterleitungen) und liefert die Hoster-Seite.
async fn resolve_plain(st: &AppState, start: &str) -> Result<(String, String), String> {
    let mut url = start.to_string();
    let mut referer: Option<String> = None;
    for _ in 0..8 {
        proxy::check_public(&url).await?;
        let mut rq = st
            .raw
            .get(&url)
            .header(USER_AGENT, UA)
            .header(ACCEPT_LANGUAGE, "de-DE,de;q=0.9,en;q=0.8")
            .timeout(Duration::from_secs(20));
        if let Some(r) = &referer {
            rq = rq.header(REFERER, r.as_str());
        }
        let resp = rq.send().await.map_err(|e| format!("Hoster nicht erreichbar: {e}"))?;
        let status = resp.status();
        if status.is_redirection() {
            let loc = resp
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or("Weiterleitung ohne Ziel")?;
            let next = Url::parse(&url).map_err(|e| e.to_string())?.join(loc).map_err(|e| e.to_string())?;
            referer = Some(url.clone());
            url = next.to_string();
            continue;
        }
        if !status.is_success() {
            return Err(format!("Hoster antwortet mit HTTP {}", status.as_u16()));
        }
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if body.len() < 4000 {
            if let Some(c) = JS_REDIRECT.captures(&body) {
                referer = Some(url.clone());
                url = c[1].to_string();
                continue;
            }
        }
        return Ok((url, body));
    }
    Err("Zu viele Weiterleitungen".into())
}

fn origin_of(url: &str) -> String {
    Url::parse(url).map(|u| format!("{}/", u.origin().ascii_serialization())).unwrap_or_default()
}

pub async fn stream(State(st): St, Query(p): Query<StreamQ>) -> Result<Json<Value>, ApiError> {
    if p.id.is_empty() || p.id.len() > 12 || !p.id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::bad("Ungültige Hoster-ID"));
    }
    let key = format!("stream:{}", p.id);
    if let Some(v) = st.cache.get(&key, T_STREAM) {
        return Ok(Json(v));
    }
    let start = format!("{}/redirect/{}", st.base, p.id);

    let mut found: Option<(String, Value)> = None;
    let mut last_url = String::new();
    match resolve_plain(&st, &start).await {
        Ok((url, body)) => {
            if let Some(s) = voe::extract(&body) {
                found = Some((url, s));
            } else {
                last_url = url;
            }
        }
        Err(e) => eprintln!("[stream] direkt fehlgeschlagen ({e}), versuche den Browser"),
    }

    if found.is_none() {
        let base = st.base.clone();
        let l = st
            .browser
            .load_html(
                &start,
                |l| !l.url.starts_with(&base) && voe::extract(&l.html).is_some(),
                Duration::from_secs(50),
            )
            .await;
        match l {
            Ok(l) => {
                if let Some(s) = voe::extract(&l.html) {
                    found = Some((l.url, s));
                }
            }
            Err(BrowserError::NeedsHuman(m)) => return Err(ApiError::human(m)),
            Err(e) => eprintln!("[stream] auch der Browser fand nichts: {}", e.message()),
        }
    }

    let Some((url, mut s)) = found else {
        let hint = if last_url.is_empty() { String::new() } else { format!(" ({})", last_url) };
        return Err(ApiError::bad_gateway(format!(
            "In diesem Hoster habe ich keinen abspielbaren Stream gefunden{hint}. Anderen Hoster probieren."
        )));
    };
    s["referer"] = Value::String(origin_of(&url));
    s["hosterUrl"] = Value::String(url);
    st.cache.put(key, s.clone());
    Ok(Json(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_filters_and_cleans() {
        let body = r#"[{"title":"<em>Naruto<\/em> Shippuden","description":"Ein &#8230; Ninja","link":"\/anime\/stream\/naruto-shippuden"},
                       {"title":"Kaputt","description":"","link":"\/anime\/stream\/x\/staffel-1"}]"#;
        let r = parse_search(body);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["slug"], "naruto-shippuden");
        assert_eq!(r[0]["title"], "Naruto Shippuden");
        assert!(r[0]["description"].as_str().unwrap().contains('…'));
    }

    const SERIES: &str = r#"<div class="seriesCoverBox"><img src="data:image/png;base64,AAAA" data-src="/public/img/cover/one-piece.png"></div>
      <div class="series-title"><h1 data-alternativeTitles="One Piece, 海贼王"><span>One Piece</span></h1></div>
      <p class="seri_des" data-full-description="Piraten &amp; Schätze">kurz</p>
      <ul class="genres"><li><a>Action</a></li></ul>
      <div id="stream"><ul><li><span><strong>Staffeln:</strong></span></li>
        <li><a href="/anime/stream/one-piece/filme">Filme</a></li>
        <li><a href="/anime/stream/one-piece/staffel-1">1</a></li>
        <li><a href="/anime/stream/one-piece/staffel-2">2</a></li></ul>
      <ul><li><a href="/anime/stream/one-piece/staffel-1/episode-1">1</a></li></ul></div>"#;

    #[test]
    fn series_page() {
        let v = parse_series(SERIES, "one-piece").expect("Serie");
        assert_eq!(v["title"], "One Piece");
        assert_eq!(v["cover"], "/public/img/cover/one-piece.png");
        assert_eq!(v["description"], "Piraten & Schätze");
        assert_eq!(v["alt"][0], "海贼王");
        let keys: Vec<&str> = v["seasons"].as_array().unwrap().iter().map(|s| s["key"].as_str().unwrap()).collect();
        assert_eq!(keys, ["filme", "1", "2"]);
    }

    #[test]
    fn season_page() {
        let html = r##"<table class="seasonEpisodesList"><tbody>
          <tr><td><a href="/anime/stream/x/staffel-1/episode-7"> Folge 7 </a></td>
          <td class="seasonEpisodeTitle"><a href="#"><strong>Titel</strong> - <span>Title</span></a></td>
          <td><i class="icon VOE" title="VOE"></i><i class="icon Vidmoly" title="Vidmoly"></i></td></tr></tbody></table>"##;
        let v = parse_season(html);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["n"], 7);
        assert_eq!(v[0]["title"], "Titel");
        assert_eq!(v[0]["titleEn"], "Title");
        assert_eq!(v[0]["hosters"][1], "Vidmoly");
    }

    #[test]
    fn episode_page() {
        let html = r#"<div class="changeLanguageBox"><img class="selectedLanguage" data-lang-key="1" title="Deutsch"><img data-lang-key="3" title="mit Untertitel Deutsch"></div>
          <ul class="row"><li data-lang-key="1" data-link-id="55" data-link-target="/redirect/55"><h4>VOE</h4></li>
          <li data-lang-key="3" data-link-id="56" data-link-target="/redirect/56"><h4>Vidmoly</h4></li></ul>"#;
        let v = parse_episode(html);
        assert_eq!(v["langs"].as_array().unwrap().len(), 2);
        assert_eq!(v["langs"][0]["on"], true);
        assert_eq!(v["hosters"][1]["id"], "56");
        assert_eq!(v["hosters"][1]["lang"], "3");
        assert_eq!(v["hosters"][0]["name"], "VOE");
    }
}
