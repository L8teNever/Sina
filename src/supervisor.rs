//! Startet und überwacht die Programme für den sichtbaren Browser im Container:
//! Xvfb (virtueller Bildschirm), openbox (Fenster), Chromium, x11vnc und websockify mit noVNC (Browser-Seite).
//! Stirbt eines, wird es nach zwei Sekunden neu gestartet.

use std::{env, fs, path::Path, process::Stdio, time::Duration};

use tokio::{process::Command, time::sleep};

fn var(name: &str, default: &str) -> String {
    env::var(name).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string())
}

fn keep(name: &'static str, program: String, args: Vec<String>, envs: Vec<(String, String)>) {
    tokio::spawn(async move {
        loop {
            let mut cmd = Command::new(&program);
            cmd.args(&args)
                .envs(envs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .kill_on_drop(true);
            match cmd.spawn() {
                Ok(mut child) => {
                    let status = child.wait().await;
                    eprintln!("[{name}] beendet: {status:?} – Neustart in 2 s");
                }
                Err(e) => eprintln!("[{name}] startet nicht ({program}): {e}"),
            }
            sleep(Duration::from_secs(2)).await;
        }
    });
}

pub fn start() {
    if var("START_BROWSER", "1") == "0" {
        eprintln!("START_BROWSER=0: Browser wird nicht gestartet (BROWSER_CDP muss extern erreichbar sein)");
        return;
    }
    tokio::spawn(async {
        let display = var("DISPLAY_NUM", ":99");
        let (w, h) = (var("SCREEN_WIDTH", "1280"), var("SCREEN_HEIGHT", "800"));
        let data = var("DATA_DIR", "/data");
        let profile = format!("{data}/chromium");
        let view_port = var("VIEW_PORT", "8081");
        let cdp_port = var("CDP_PORT", "9222");
        let x_env = vec![("DISPLAY".to_string(), display.clone()), ("HOME".to_string(), format!("{data}/home"))];
        let _ = fs::create_dir_all(format!("{data}/home"));
        let _ = fs::create_dir_all(&profile);

        keep(
            "xvfb",
            "Xvfb".into(),
            vec![display.clone(), "-screen".into(), "0".into(), format!("{w}x{h}x24"), "-nolisten".into(), "tcp".into()],
            vec![],
        );
        sleep(Duration::from_millis(1500)).await;
        keep("openbox", "openbox".into(), vec![], x_env.clone());

        // Reste einer früheren Sitzung entfernen, sonst fragt Chromium nach einem gesperrten Profil.
        for f in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
            let _ = fs::remove_file(Path::new(&profile).join(f));
        }
        keep(
            "chromium",
            var("CHROMIUM_BIN", "chromium"),
            vec![
                "--no-sandbox".into(),
                "--disable-gpu".into(),
                "--disable-dev-shm-usage".into(),
                "--no-first-run".into(),
                "--no-default-browser-check".into(),
                "--password-store=basic".into(),
                "--disable-blink-features=AutomationControlled".into(),
                "--lang=de-DE".into(),
                format!("--user-data-dir={profile}"),
                "--remote-debugging-address=127.0.0.1".into(),
                format!("--remote-debugging-port={cdp_port}"),
                "--window-position=0,0".into(),
                format!("--window-size={w},{h}"),
                "about:blank".into(),
            ],
            x_env.clone(),
        );

        let mut vnc = vec![
            "-display".into(),
            display.clone(),
            "-rfbport".into(),
            "5900".into(),
            "-localhost".into(),
            "-forever".into(),
            "-shared".into(),
            "-noxdamage".into(),
            "-quiet".into(),
        ];
        match env::var("VIEW_PASSWORD").ok().filter(|p| !p.is_empty()) {
            Some(pw) => {
                vnc.push("-passwd".into());
                vnc.push(pw);
            }
            None => vnc.push("-nopw".into()),
        }
        keep("x11vnc", "x11vnc".into(), vnc, x_env);

        keep(
            "websockify",
            "websockify".into(),
            vec!["--web".into(), var("NOVNC_DIR", "/app/novnc"), view_port, "127.0.0.1:5900".into()],
            vec![],
        );
    });
}
