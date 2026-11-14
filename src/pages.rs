//! Übersichtsseiten von AniWorld: Start, Alle Animes, Beliebt, Kalender (nach dem Vorbild des NINA-Skins).

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    Json,
};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::LazyLock;

use crate::{ani::get_html, ApiError, AppState};

const T_PAGE: Duration = Duration::from_secs(600);
const T_LIST: Duration = Duration::from_secs(1800);

static TITLE_TAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*(als\s+)?Stream\s+(anschauen|ansehen).*$").unwrap());
static CAL_HREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^/anime/stream/([^/]+)(?:/staffel-(\d+)/episode-(\d+)|/filme/film-(\d+))?").unwrap()
});
static TIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{1,2}:\d{2})").unwrap());
static SE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)S(\d+)\s*E(\d+)").unwrap());
static FILM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bFilm\s*(\d+)").unwrap());

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("Selektor")
}

fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sichtbarer Text (ohne noscript/script/style/Bilder).
pub fn text_of(el: ElementRef) -> String {
    fn walk(el: ElementRef, out: &mut String) {
        for c in el.children() {
            if let Some(t) = c.value().as_text() {
                out.push_str(t);
            } else if let Some(e) = ElementRef::wrap(c) {
                if !matches!(e.value().name(), "noscript" | "script" | "style" | "img") {
                    walk(e, out);
                }
            }
        }
    }
    let mut s = String::new();
    walk(el, &mut s);
    clean(&s)
}

pub fn img_of(el: ElementRef) -> String {
    el.select(&sel("img:not(.flag)"))
        .next()
        .map(|i| {
            let v = i.value();
            let src = v.attr("data-src").filter(|s| !s.is_empty()).or_else(|| v.attr("src")).unwrap_or("");
            if src.starts_with("data:") || src.contains("placeholder") || src.contains("blank.gif") {
                String::new()
            } else {
                src.to_string()
            }
        })
        .unwrap_or_default()
}

fn slug_of(href: &str) -> Option<String> {
    let parts: Vec<&str> = href.trim_matches('/').split('/').collect();
    (parts.len() >= 3 && parts[0] == "anime" && parts[1] == "stream" && !parts[2].is_empty()).then(|| parts[2].to_string())
}

fn inside_chrome(el: ElementRef) -> bool {
    el.ancestors().filter_map(ElementRef::wrap).any(|a| {
        let v = a.value();
        matches!(v.name(), "nav" | "header" | "footer") || v.id() == Some("stream") || v.attr("class").map(|c| c.contains("series-title")).unwrap_or(false)
    })
}

/// Eine Cover-Karte: { slug, title, img, sub }.
pub fn card_of(a: ElementRef) -> Option<Value> {
    let href = a.value().attr("href")?;
    let slug = slug_of(href)?;
    let title = a
        .select(&sel("h3"))
        .next()
        .map(|h| clean(&h.text().collect::<String>()))
        .filter(|t| !t.is_empty())
        .or_else(|| {
            let t = TITLE_TAIL.replace(a.value().attr("title").unwrap_or(""), "").to_string();
            let t = clean(&t);
            (!t.is_empty()).then_some(t)
        })
        .or_else(|| {
            a.select(&sel("img")).next().and_then(|i| i.value().attr("alt")).map(|s| clean(&s.replace(" Cover", ""))).filter(|t| !t.is_empty())
        })?;
    let sub = a.select(&sel("small, h4, .seriesListHorizontalYear")).next().map(text_of).unwrap_or_default();
    Some(json!({"slug": slug, "title": title, "img": img_of(a), "sub": sub}))
}

fn cards_from<'a>(anchors: impl Iterator<Item = ElementRef<'a>>) -> Vec<Value> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for a in anchors {
        if let Some(c) = card_of(a) {
            let slug = c["slug"].as_str().unwrap_or("").to_string();
            if seen.contains(&slug) {
                continue;
            }
            seen.push(slug);
            out.push(c);
        }
    }
    out
}

pub fn parse_home(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let title_sel = sel(".pageTitle15, .pageTitleB");
    let anchor_sel = sel("a[href^='/anime/stream/']");
    let cal_sel = sel(".tvSeriesCalendarNavigation");
    let mut sections: Vec<Value> = Vec::new();
    for t in doc.select(&title_sel) {
        let Some(h) = t.select(&sel("h2")).next() else { continue };
        let mut anchors: Vec<ElementRef> = Vec::new();
        let mut cal = false;
        for e in t.next_siblings().filter_map(ElementRef::wrap) {
            if title_sel.matches(&e) {
                break;
            }
            if cal_sel.matches(&e) || e.select(&cal_sel).next().is_some() {
                cal = true;
                continue;
            }
            anchors.extend(e.select(&anchor_sel));
        }
        if cal {
            continue;
        }
        let cards: Vec<Value> = cards_from(anchors.into_iter()).into_iter().filter(|c| !c["img"].as_str().unwrap_or("").is_empty()).collect();
        let wide = cards.iter().all(|c| c["img"].as_str().unwrap_or("").contains("/img/home/"));
        if cards.len() >= 3 && !wide {
            sections.push(json!({"title": text_of(h), "cards": cards}));
        }
    }
    let promo_sel = sel(".homeContentPromotionBoxPicture");
    let hero_anchors = doc.select(&anchor_sel).filter(|a| a.select(&promo_sel).next().is_some());
    let hero: Vec<Value> = cards_from(hero_anchors).into_iter().filter(|c| !c["img"].as_str().unwrap_or("").is_empty()).collect();
    json!({"hero": hero, "sections": sections})
}

/// Seiten, die nur aus Cover-Karten bestehen (z. B. „Beliebt“).
pub fn parse_cards(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let anchor_sel = sel("a[href^='/anime/stream/']");
    let anchors = doc.select(&anchor_sel).filter(|a| {
        let href = a.value().attr("href").unwrap_or("");
        href.trim_matches('/').split('/').count() == 3 && !inside_chrome(*a)
    });
    let cards: Vec<Value> = cards_from(anchors).into_iter().filter(|c| !c["img"].as_str().unwrap_or("").is_empty()).collect();
    let title = doc.select(&sel("h1, .pageTitle h2, h2")).next().map(text_of).filter(|t| !t.is_empty()).unwrap_or_else(|| "Beliebt".into());
    json!({"title": title, "cards": cards})
}

/// „Alle Animes“: Titel mit ihren Genres.
pub fn parse_all(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, (String, String, Vec<String>)> = std::collections::HashMap::new();
    for g in doc.select(&sel(".genre")) {
        let genre = g.select(&sel(".seriesGenreList h3")).next().map(text_of).unwrap_or_default();
        for a in g.select(&sel("li a[href]")) {
            let Some(slug) = a.value().attr("href").and_then(slug_of) else { continue };
            let title = text_of(a);
            if title.is_empty() {
                continue;
            }
            let alt: String = clean(a.value().attr("data-alternative-title").unwrap_or("")).chars().take(160).collect();
            let e = map.entry(slug.clone()).or_insert_with(|| {
                order.push(slug.clone());
                (title, alt, Vec::new())
            });
            if !genre.is_empty() && !e.2.contains(&genre) {
                e.2.push(genre.clone());
            }
        }
    }
    let items: Vec<Value> = order
        .iter()
        .filter_map(|s| map.get(s).map(|(t, a, g)| json!({"slug": s, "title": t, "alt": a, "genres": g})))
        .collect();
    json!({"title": "Alle Animes", "items": items})
}

pub fn parse_calendar(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let mut days: Vec<Value> = Vec::new();
    for s in doc.select(&sel("section.calendarList")) {
        let label = s
            .select(&sel("h3, h2"))
            .find(|h| !h.value().attr("class").unwrap_or("").contains("seriesTitle"))
            .map(text_of)
            .unwrap_or_default();
        let mut entries: Vec<Value> = Vec::new();
        for a in s.select(&sel("a[href^='/anime/stream/']")) {
            let href = a.value().attr("href").unwrap_or("");
            let Some(m) = CAL_HREF.captures(href) else { continue };
            let all = text_of(a);
            let time = TIME.captures(&all).map(|c| c[1].to_string()).unwrap_or_default();
            let flag = a.select(&sel("img.flag")).next();
            let ft = flag.map(|f| f.value().attr("title").or_else(|| f.value().attr("alt")).unwrap_or("").to_string()).unwrap_or_default();
            let ftl = ft.to_lowercase();
            let lang = if ftl.contains("englisch") || ftl.contains("english") {
                "Eng-Sub"
            } else if ftl.contains("untertitel") || ftl.contains("sub") {
                "Ger-Sub"
            } else if !ft.is_empty() {
                "Deutsch"
            } else {
                ""
            };
            let se = SE.captures(&all);
            let fl = FILM.captures(&all);
            let num = |i: usize| m.get(i).and_then(|x| x.as_str().parse::<u32>().ok());
            let season = num(2).or_else(|| se.as_ref().and_then(|c| c[1].parse().ok())).unwrap_or(0);
            let episode = num(3)
                .or_else(|| num(4))
                .or_else(|| se.as_ref().and_then(|c| c[2].parse().ok()))
                .or_else(|| fl.as_ref().and_then(|c| c[1].parse().ok()))
                .unwrap_or(0);
            let film = m.get(4).is_some() || (m.get(2).is_none() && se.is_none() && fl.is_some());
            let title = a
                .select(&sel("h3, .seriesTitle"))
                .next()
                .map(text_of)
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| clean(a.value().attr("title").unwrap_or("")));
            entries.push(json!({
                "slug": &m[1], "title": title, "season": season, "episode": episode, "film": film,
                "time": time, "lang": lang, "out": a.select(&sel(".fa-check")).next().is_some(), "img": img_of(a),
            }));
        }
        if !entries.is_empty() {
            days.push(json!({"label": label, "today": label.to_lowercase().contains("heute"), "entries": entries}));
        }
    }
    json!({"days": days})
}

type St = State<Arc<AppState>>;

pub async fn home(State(st): St) -> Result<Json<Value>, ApiError> {
    if let Some(v) = st.cache.get("page:home", T_PAGE) {
        return Ok(Json(v));
    }
    let html = get_html(&st, "/", "pageTitle15").await?;
    let v = parse_home(&html);
    st.cache.put("page:home".into(), v.clone());
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct ListQ {
    kind: String,
}

pub async fn list(State(st): St, Query(q): Query<ListQ>) -> Result<Json<Value>, ApiError> {
    let key = format!("page:list:{}", q.kind);
    if let Some(v) = st.cache.get(&key, T_LIST) {
        return Ok(Json(v));
    }
    let v = match q.kind.as_str() {
        "all" => parse_all(&get_html(&st, "/animes", "seriesContainer").await?),
        "popular" => parse_cards(&get_html(&st, "/beliebte-animes", "/anime/stream/").await?),
        _ => return Err(ApiError::bad("Unbekannte Liste")),
    };
    st.cache.put(key, v.clone());
    Ok(Json(v))
}

pub async fn calendar(State(st): St) -> Result<Json<Value>, ApiError> {
    if let Some(v) = st.cache.get("page:calendar", T_PAGE) {
        return Ok(Json(v));
    }
    let html = get_html(&st, "/animekalender", "calendarList").await?;
    let v = parse_calendar(&html);
    st.cache.put("page:calendar".into(), v.clone());
    Ok(Json(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_and_hero() {
        let html = r#"<div class="pageTitle15"><h2>Neue Animes</h2></div>
          <div class="previews"><div class="coverListItem"><a href="/anime/stream/a" title="A als Stream anschauen"><div><img data-src="/public/img/cover/a.png" src="data:image/png;base64,AA"></div><h3> A </h3></a></div>
          <a href="/anime/stream/b" title="B Stream anschauen"><img data-src="/public/img/cover/b.png"></a>
          <a href="/anime/stream/c" title="C Stream ansehen"><img data-src="/public/img/cover/c.png"></a></div>
          <div class="pageTitle15"><h2>Promo</h2></div>
          <a href="/anime/stream/p"><div class="homeContentPromotionBoxPicture"><img data-src="/public/img/home/p.png"><span><h3>PROMO</h3></span></div></a>"#;
        let v = parse_home(html);
        let s = &v["sections"][0];
        assert_eq!(s["title"], "Neue Animes");
        assert_eq!(s["cards"].as_array().unwrap().len(), 3);
        assert_eq!(s["cards"][0]["title"], "A");
        assert_eq!(s["cards"][1]["title"], "B");
        assert_eq!(s["cards"][0]["img"], "/public/img/cover/a.png");
        assert_eq!(v["hero"][0]["slug"], "p");
    }

    #[test]
    fn all_list() {
        let html = r#"<div class="genre"><div class="seriesGenreList"><h3>Action</h3></div><ul>
          <li><a data-alternative-title="Alt A" href="/anime/stream/a" title="x">A</a></li><li><a href="/anime/stream/b">B</a></li></ul></div>
          <div class="genre"><div class="seriesGenreList"><h3>Comedy</h3></div><ul><li><a href="/anime/stream/a">A</a></li></ul></div>"#;
        let v = parse_all(html);
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["alt"], "Alt A");
        assert_eq!(items[0]["genres"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn calendar_entries() {
        let html = r#"<section class="calendarList"><h3>Samstag, 14.11.2026 (heute)</h3><div>
          <a href="/anime/stream/keroro/staffel-1/episode-7"><img data-src="/public/img/cover/k.png"><h3 class="seriesTitle">KERORO</h3>
          <small>S01E07 <img class="flag" title="Episode 7 auf Englisch"></small> 21:30</a></div></section>"#;
        let v = parse_calendar(html);
        let e = &v["days"][0]["entries"][0];
        assert_eq!(v["days"][0]["today"], true);
        assert_eq!(e["slug"], "keroro");
        assert_eq!(e["season"], 1);
        assert_eq!(e["episode"], 7);
        assert_eq!(e["time"], "21:30");
        assert_eq!(e["lang"], "Eng-Sub");
        assert_eq!(e["title"], "KERORO");
    }
}
