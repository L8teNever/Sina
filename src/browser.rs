//! Der echte Chromium (läuft im selben Container, sichtbar über die Browser-Seite) wird per CDP
//! ferngesteuert. Er löst Schutzseiten (DDoS-Guard) und lädt Hoster-Seiten wie ein normaler Besucher.
//! Muss eine Schutzseite von Hand gelöst werden, bleibt der Tab offen und du siehst ihn auf der Browser-Seite.

use std::{env, sync::Mutex, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{
    net::TcpStream,
    sync::Semaphore,
    time::{sleep, Instant},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug)]
pub enum BrowserError {
    /// Browser nicht erreichbar.
    Unavailable(String),
    /// Schutzseite, die du im Browser selbst lösen musst.
    NeedsHuman(String),
    /// Sonstiger Fehler (Timeout, kaputte Antwort).
    Failed(String),
}

impl BrowserError {
    pub fn message(&self) -> &str {
        match self {
            BrowserError::Unavailable(m) | BrowserError::NeedsHuman(m) | BrowserError::Failed(m) => m,
        }
    }
}

pub struct Loaded {
    pub url: String,
    pub title: String,
    pub html: String,
}

struct Tab {
    id: String,
    ws: Ws,
    n: u64,
}

pub struct Browser {
    base: String,
    http: reqwest::Client,
    sem: Semaphore,
    kept: Mutex<Vec<String>>,
}

fn unavailable(e: impl std::fmt::Display) -> BrowserError {
    BrowserError::Unavailable(format!("Browser nicht erreichbar: {e}"))
}

fn failed(e: impl std::fmt::Display) -> BrowserError {
    BrowserError::Failed(format!("Browser-Steuerung: {e}"))
}

const PROBE: &str = "JSON.stringify({u:location.href,t:document.title,s:document.readyState,h:document.readyState==='complete'&&document.documentElement?document.documentElement.outerHTML:''})";

fn is_challenge(title: &str) -> bool {
    let t = title.to_lowercase();
    t.contains("ddos-guard") || t.contains("just a moment") || t.contains("checking your browser") || t.contains("attention required")
}

impl Browser {
    pub fn from_env() -> Browser {
        let base = env::var("BROWSER_CDP").unwrap_or_else(|_| "http://127.0.0.1:9222".into());
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("HTTP-Client");
        Browser {
            base: base.trim().trim_end_matches('/').to_string(),
            http,
            sem: Semaphore::new(3),
            kept: Mutex::new(Vec::new()),
        }
    }

    pub async fn alive(&self) -> bool {
        match self.http.get(format!("{}/json/version", self.base)).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    async fn open(&self, url: &str) -> Result<Tab, BrowserError> {
        let target: Value = self
            .http
            .put(format!("{}/json/new?{}", self.base, urlencoding::encode(url)))
            .send()
            .await
            .map_err(unavailable)?
            .json()
            .await
            .map_err(unavailable)?;
        let Some(id) = target["id"].as_str().map(str::to_string) else {
            return Err(BrowserError::Unavailable("Browser hat keinen Tab geöffnet".into()));
        };
        let ws_base = self.base.replacen("http://", "ws://", 1);
        let (ws, _) = connect_async(format!("{ws_base}/devtools/page/{id}"))
            .await
            .map_err(unavailable)?;
        Ok(Tab { id, ws, n: 0 })
    }

    async fn close_id(&self, id: &str) {
        let _ = self.http.get(format!("{}/json/close/{}", self.base, id)).send().await;
    }

    /// Tab schließen. Wartet er auf einen Menschen, bleibt er offen (maximal 3 Stück).
    async fn finish(&self, tab: Tab, keep: bool) {
        let id = tab.id.clone();
        drop(tab);
        let mut to_close: Vec<String> = Vec::new();
        if keep {
            let mut k = self.kept.lock().unwrap_or_else(|e| e.into_inner());
            k.push(id);
            while k.len() > 3 {
                to_close.push(k.remove(0));
            }
        } else {
            to_close.push(id);
            let mut k = self.kept.lock().unwrap_or_else(|e| e.into_inner());
            to_close.append(&mut k);
        }
        for id in to_close {
            self.close_id(&id).await;
        }
    }

    /// Lädt `url` im Browser, bis `accept` die fertig geladene Seite annimmt.
    pub async fn load_html<F: Fn(&Loaded) -> bool>(
        &self,
        url: &str,
        accept: F,
        wait: Duration,
    ) -> Result<Loaded, BrowserError> {
        let _permit = self.sem.acquire().await.map_err(failed)?;
        let mut tab = self.open(url).await?;
        let res = poll_html(&mut tab, &accept, wait).await;
        let keep = matches!(res, Err(BrowserError::NeedsHuman(_)));
        self.finish(tab, keep).await;
        res
    }

    /// Lädt `url`, wartet bis die Seite fertig ist und führt dann `expr` darin aus (darf ein Promise sein).
    pub async fn eval_after_load(&self, url: &str, expr: &str, wait: Duration) -> Result<Value, BrowserError> {
        let _permit = self.sem.acquire().await.map_err(failed)?;
        let mut tab = self.open(url).await?;
        let res = async {
            poll_html(&mut tab, &|_: &Loaded| true, wait).await?;
            tab.eval(expr, true).await
        }
        .await;
        let keep = matches!(res, Err(BrowserError::NeedsHuman(_)));
        self.finish(tab, keep).await;
        res
    }
}

impl Tab {
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, BrowserError> {
        self.n += 1;
        let id = self.n;
        self.ws
            .send(Message::text(json!({"id": id, "method": method, "params": params}).to_string()))
            .await
            .map_err(failed)?;
        loop {
            let Some(m) = self.ws.next().await else {
                return Err(BrowserError::Failed("Browser hat die Verbindung geschlossen".into()));
            };
            let m = m.map_err(failed)?;
            if let Message::Text(t) = m {
                if let Ok(v) = serde_json::from_str::<Value>(&t) {
                    if v["id"].as_u64() == Some(id) {
                        return Ok(v);
                    }
                }
            }
        }
    }

    async fn eval(&mut self, expr: &str, await_promise: bool) -> Result<Value, BrowserError> {
        let r = self
            .call(
                "Runtime.evaluate",
                json!({"expression": expr, "awaitPromise": await_promise, "returnByValue": true}),
            )
            .await?;
        if let Some(ex) = r["result"]["exceptionDetails"].as_object() {
            let text = ex
                .get("exception")
                .and_then(|e| e["description"].as_str())
                .or_else(|| ex.get("text").and_then(Value::as_str))
                .unwrap_or("Fehler im Browser");
            return Err(BrowserError::Failed(text.to_string()));
        }
        if let Some(e) = r["error"]["message"].as_str() {
            return Err(BrowserError::Failed(e.to_string()));
        }
        Ok(r["result"]["result"]["value"].clone())
    }
}

async fn poll_html<F: Fn(&Loaded) -> bool>(tab: &mut Tab, accept: &F, wait: Duration) -> Result<Loaded, BrowserError> {
    let deadline = Instant::now() + wait;
    let mut challenge = false;
    loop {
        // Während einer Weiterleitung schlägt Runtime.evaluate kurz fehl: dann einfach nochmal.
        if let Ok(v) = tab.eval(PROBE, false).await {
            if let Ok(p) = serde_json::from_str::<Value>(v.as_str().unwrap_or("")) {
                let loaded = Loaded {
                    url: p["u"].as_str().unwrap_or("").to_string(),
                    title: p["t"].as_str().unwrap_or("").to_string(),
                    html: p["h"].as_str().unwrap_or("").to_string(),
                };
                challenge = is_challenge(&loaded.title);
                if p["s"].as_str() == Some("complete") && !challenge && loaded.url.starts_with("http") && accept(&loaded) {
                    return Ok(loaded);
                }
            }
        }
        if Instant::now() > deadline {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    if challenge {
        Err(BrowserError::NeedsHuman(
            "Die Seite zeigt eine Schutzprüfung. Öffne die Browser-Seite und löse sie dort, dann nochmal versuchen.".into(),
        ))
    } else {
        Err(BrowserError::Failed("Die Seite wurde im Browser nicht rechtzeitig (oder nicht richtig) geladen.".into()))
    }
}
