# Sina

AniWorld im Look der NINA-Erweiterung: suchen, Serien und Folgen ansehen, anklicken und streamen, mit
demselben Android-16-Player (Material 3). Alles läuft in **einem Docker-Container** (Rust). Darin steckt ein
echter Chromium, den du über eine zweite Seite live sehen und bedienen kannst.

## Die zwei Seiten

| Seite | Port | Was |
|---|---|---|
| **Sina** | `8080` | Start, Alle Animes (A–Z, Genres, Filter), Beliebt, Kalender, Suche mit Vorschlägen, Serienseite, Folgenseite mit Player |
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
4. **Player:** Der Android-16-Player aus der NINA-Erweiterung (`static/player.js`, `player.css`) läuft unverändert bis auf
   eine kleine Schnittstelle (`static/shim.js`). Er hat Gesten und Tasten, Geschwindigkeit, Qualität, Bild-in-Bild,
   Intro/Outro überspringen (AniSkip), Auto-Weiter und merkt sich die Position.
5. **ani.pm:** Gibt es die Folge dort, erscheinen die Knöpfe „Eng-Dub“ und „Eng-Sub“. ani.pm blockt Server-Anfragen
   oft (Cloudflare), deshalb holt der Chromium im Container diese Daten.
6. **Fortschritt:** Position und „gesehen“ speichert der Player im Browser (localStorage).

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

## VPN (z. B. Proton)

Der Hoster bindet die Stream-Adresse an die IP, die die Hoster-Seite geholt hat. Server und Chromium müssen deshalb
**über denselben Ausgang** ins Netz. Eine VPN-Erweiterung im Chromium reicht nicht: sie schützt nur den Browser,
das Video lädt aber der Server.

`docker-compose.vpn.example.yml` startet einen gluetun-Container mit Proton VPN (WireGuard oder OpenVPN) und setzt
`HTTPS_PROXY`. Der Chromium nutzt automatisch denselben Proxy (oder `BROWSER_PROXY`). Der Knopf „VPN“ oben in Sina
zeigt die IP von Server und Browser und ob sie gleich sind.

## Entwickeln

```bash
cargo test                          # Parser, VOE-Entschlüsselung, Proxy-Schutz
START_BROWSER=0 cargo run           # ohne Chromium, reicht für Suche und Streams ohne Schutzseite
```

Das Image baut GitHub Actions bei jedem Push auf `main` nach `ghcr.io/l8tenever/sina:latest`.

## Grenzen

- Nur AniWorld (`/anime/stream/...`) und ani.pm, keine Downloads, keine Anbindung an „Meine Serien“ aus NINA.
  „Gesehen“ und Fortschritt speichert der Browser lokal.
- Der Teil für ani.pm (Session, Embed-Seite) läuft über den Chromium und ist nur mit echtem Browser prüfbar.
- Hoster ändern ihre Seiten ab und zu. VOE und Hoster mit offener `.m3u8`/`.mp4` im Quelltext gehen.
  Geht ein Hoster nicht, probiert der Player die anderen automatisch, oder du wählst einen von Hand.
