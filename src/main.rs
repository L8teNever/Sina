//! Sina: AniWorld mit eigener Suche und eigenem Player. Ein Chromium im selben Container wird per CDP gesteuert
//! und ist über eine zweite Seite (noVNC) sichtbar und bedienbar.

mod ani;
mod anipm;
mod browser;
mod cache;
mod devices;
mod kino;
mod pages;
mod profiles;
mod proxy;
mod supervisor;
mod voe;
mod vpn;

use std::{env, sync::Arc, time::Duration};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use tower_http::services::ServeDir;

pub const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub struct AppState {
    /// Mit Cookies und Weiterleitungen, für AniWorld selbst.
    pub http: reqwest::Client,
    /// Ohne Weiterleitungen und Cookies, für Hoster und Stream-Proxy.
    pub raw: reqwest::Client,
    pub browser: browser::Browser,
    pub cache: cache::Cache,
    pub devices: devices::Devices,
    /// z. B. https://aniworld.to (ohne Schrägstrich am Ende)
    pub base: String,
}

pub struct ApiError {
    status: StatusCode,
    msg: String,
    human: bool,
}

impl ApiError {
    pub fn new(status: StatusCode, msg: impl Into<String>) -> Self {
        Self { status, msg: msg.into(), human: false }
    }
    pub fn bad(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, msg: msg.into(), human: false }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, msg: msg.into(), human: false }
    }
    pub fn bad_gateway(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_GATEWAY, msg: msg.into(), human: false }
    }
    /// Der Nutzer muss im Browser etwas lösen.
    pub fn human(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_GATEWAY, msg: msg.into(), human: true }
    }
}

impl From<browser::BrowserError> for ApiError {
    fn from(e: browser::BrowserError) -> Self {
        match e {
            browser::BrowserError::NeedsHuman(m) => ApiError::human(m),
            browser::BrowserError::Unavailable(m) | browser::BrowserError::Failed(m) => ApiError::bad_gateway(m),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error": self.msg, "needsBrowser": self.human}))).into_response()
    }
}

/// Ausgehende IP des Servers und des Browsers. Beide müssen gleich sein, sonst lehnt der Hoster das Video ab
/// (der Stream-Link gilt nur für die IP, die die Hoster-Seite geholt hat).
async fn vpn(axum::extract::State(st): axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    let server = async {
        let r = st.raw.get("https://api.ipify.org?format=json").timeout(Duration::from_secs(10)).send().await.ok()?;
        let j: serde_json::Value = r.json().await.ok()?;
        j["ip"].as_str().map(str::to_string)
    }
    .await;
    let browser = match st
        .browser
        .eval_after_load(
            "https://api.ipify.org/",
            "fetch('https://api.ipify.org?format=json').then(r=>r.json()).then(j=>j.ip)",
            Duration::from_secs(25),
        )
        .await
    {
        Ok(v) => v.as_str().map(str::to_string),
        Err(_) => None,
    };
    let proxy = ["HTTPS_PROXY", "https_proxy", "BROWSER_PROXY"].iter().any(|k| env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false));
    Json(json!({"server": server, "browser": browser, "proxy": proxy, "same": server.is_some() && server == browser}))
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "static".into());
    let base = env::var("ANIWORLD_URL").unwrap_or_else(|_| "https://aniworld.to".into());
    let base = base.trim().trim_end_matches('/').to_string();
    let view_url = env::var("VIEW_URL").unwrap_or_default();
    let view_port = env::var("VIEW_PORT").unwrap_or_else(|_| "8081".into());

    let http = reqwest::Client::builder()
        .cookie_store(true)
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("HTTP-Client");
    let raw = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(60))
        .build()
        .expect("HTTP-Client");
    let state = Arc::new(AppState { http, raw, browser: browser::Browser::from_env(), cache: cache::Cache::default(), devices: devices::Devices::default(), base });

    supervisor::start();
    tokio::spawn(vpn::keep_endpoint_fresh());

    let st = state.clone();
    let config = move || {
        let st = st.clone();
        let view_url = view_url.clone();
        let view_port = view_port.clone();
        async move {
            Json(json!({
                "browserUp": st.browser.alive().await,
                "viewUrl": view_url,
                "viewPort": view_port,
                "site": st.base,
            }))
        }
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/config", get(config))
        .route("/api/vpn", get(vpn))
        .route("/api/profiles", get(profiles::list))
        .route("/api/devices", get(devices::list))
        .route("/api/devices/poll", get(devices::poll))
        .route("/api/devices/state", axum::routing::post(devices::update_state).layer(axum::extract::DefaultBodyLimit::max(4 * 1024)))
        .route("/api/devices/send", axum::routing::post(devices::send).layer(axum::extract::DefaultBodyLimit::max(8 * 1024)))
        .route(
            "/api/state",
            get(profiles::get_state).put(profiles::put_state).layer(axum::extract::DefaultBodyLimit::max(520 * 1024)),
        )
        .route(
            "/api/settings/vpn",
            get(vpn::status).post(vpn::upload).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .route("/api/search", get(ani::search))
        .route("/api/series", get(ani::series))
        .route("/api/season", get(ani::season))
        .route("/api/episode", get(ani::episode))
        .route("/api/stream", get(ani::stream))
        .route("/api/kino/list", get(kino::list))
        .route("/api/kino/search", get(kino::search))
        .route("/api/kino/film", get(kino::film))
        .route("/api/kino/stream", get(kino::stream))
        .route("/api/home", get(pages::home))
        .route("/api/list", get(pages::list))
        .route("/api/calendar", get(pages::calendar))
        .route("/api/aniskip", get(anipm::aniskip))
        .route("/api/anipm/check", get(anipm::check))
        .route("/api/anipm/stream", get(anipm::stream))
        .route("/api/hls", get(proxy::hls))
        .route("/api/img", get(proxy::img))
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port.parse::<u16>().unwrap_or(8080)))
        .await
        .expect("Port nicht frei");
    eprintln!("Sina läuft auf Port {port}");
    axum::serve(listener, app).await.expect("Server-Fehler");
}
