# Sina

AniWorld mit eigener Oberfläche: suchen, Serien und Folgen ansehen, anklicken und streamen.
Alles läuft in **einem Docker-Container** (Rust). Darin steckt ein echter Chromium, den du über eine zweite Seite
live sehen und bedienen kannst.

## Die zwei Seiten

| Seite | Port | Was |
|---|---|---|
| **Sina** | `8080` | Suche, Serienseite mit Staffeln und Folgen, Player (Sprache und Hoster wählbar, Auto-Weiter, „Weiter schauen“) |
| **Browser** | `8081` | Der Chromium im Container (noVNC). Hier löst du Schutzprüfungen und siehst, was der Server tut |

Der Link „Browser“ oben in Sina führt auf die zweite Seite. Steht sie hinter einer anderen Adresse
(z. B. Cloudflare-Tunnel), setze `VIEW_URL`.

> **Wichtig:** Die Browser-Seite gibt volle Kontrolle über den Browser. Nur hinter Login (Cloudflare Access)
> freigeben und/oder `VIEW_PASSWORD` setzen.

## Start

```bash
docker compose up -d
```

Dann `http://localhost:8080` (Sina) und `http://localhost:8081` (Browser).

## Wie es funktioniert

1. **Suche, Serien, Folgen:** Der Server holt die AniWorld-Seiten per HTTP und liest sie aus. Zeigt AniWorld eine
   Schutzseite (DDoS-Guard), lädt der Chromium die Seite stattdessen. Kommt eine Prüfung, die nur ein Mensch lösen kann,
   bleibt der Tab offen. Du löst sie auf der Browser-Seite, danach klappt es wieder.
2. **Stream:** Ein Klick auf eine Folge öffnet den Hoster-Link (zuerst VOE), folgt den Weiterleitungen, liest die
   verschlüsselte Stream-Adresse aus (`src/voe.rs`) und gibt sie an den Player. Findet der Server nichts, versucht es der Browser.
3. **Abspielen:** HLS (hls.js) läuft über `/api/hls`. Der Hoster bindet die Stream-Adresse an die IP, die die Seite
   geholt hat. Deshalb muss auch das Video über den Server laufen. Der Proxy spricht nur öffentliche Adressen an.
4. **Fortschritt:** Position und „gesehen“ speichert der Player im Browser (localStorage).

## Umgebungsvariablen

| Variable | Standard | Bedeutung |
|---|---|---|
| `PORT` | `8080` | Port der Sina-Seite |
| `VIEW_PORT` | `8081` | Port der Browser-Seite |
| `VIEW_URL` | – | Adresse der Browser-Seite, wenn nicht `<Host>:8081` |
| `VIEW_PASSWORD` | – | Passwort für die Browser-Seite (noVNC fragt danach) |
| `ANIWORLD_URL` | `https://aniworld.to` | Adresse von AniWorld |
| `SCREEN_WIDTH` / `SCREEN_HEIGHT` | `1280` / `800` | Größe des Browser-Bildschirms |
| `START_BROWSER` | `1` | `0` = keinen eigenen Chromium starten |
| `BROWSER_CDP` | `http://127.0.0.1:9222` | Fernsteuer-Adresse des Chromium (bei externem Browser als IP, nicht Name) |
| `HTTPS_PROXY` | – | Ausgehender Proxy/VPN, falls Hoster die Server-IP sperren |

Das Browser-Profil (Cookies, gelöste Prüfungen) liegt im Volume `/data`.

## Entwickeln

```bash
cargo test                          # Parser, VOE-Entschlüsselung, Proxy-Schutz
START_BROWSER=0 cargo run           # ohne Chromium, reicht für Suche und Streams ohne Schutzseite
```

Das Image baut GitHub Actions bei jedem Push auf `main` nach `ghcr.io/l8tenever/sina:latest`.

## Grenzen

- Nur AniWorld (`/anime/stream/...`), keine Downloads.
- Hoster ändern ihre Seiten ab und zu. VOE und Hoster mit offener `.m3u8`/`.mp4` im Quelltext gehen.
  Geht ein Hoster nicht, probiert der Player die anderen automatisch, oder du wählst einen von Hand.
