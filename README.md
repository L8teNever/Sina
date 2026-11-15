# Sina

AniWorld im Look der NINA-Erweiterung: suchen, Serien und Folgen ansehen, anklicken und streamen, mit
demselben Android-16-Player (Material 3). Alles läuft in **einem Docker-Container** (Rust). Darin steckt ein
echter Chromium, den du über eine zweite Seite live sehen und bedienen kannst.

## Die zwei Seiten

| Seite | Port | Was |
|---|---|---|
| **Sina** | `8080` | Start, Alle Animes (A–Z, Genres, Filter), Beliebt, Kalender, Suche mit Vorschlägen, Serienseite, Folgenseite mit Player |
| **Browser** | `8081` | Der Chromium im Container (noVNC). Hier löst du Schutzprüfungen und siehst, was der Server tut |

Der Knopf „Browser“ in den Sina-Einstellungen führt auf die zweite Seite. Steht sie hinter einer anderen Adresse
(z. B. Cloudflare-Tunnel), setze `VIEW_URL`.

> **Wichtig:** Die Browser-Seite gibt volle Kontrolle über den Browser. Nur hinter Login (Cloudflare Access)
> freigeben und/oder `VIEW_PASSWORD` setzen.

## Profile

Beim Öffnen fragt Sina wie bei Netflix „Wer schaut?“: **Simon, Melanie, Eltern, Martin** und **Gast**. Jedes Profil hat
seinen eigenen Stand: Weiter schauen, Fortschritt, „gesehen“, Sprache und die Position in jeder Folge. Die Profile
sind voneinander getrennt. Der Stand liegt auf dem Server (`/data/profiles/<name>.json`), ist also auf Handy, PC und
Fernseher gleich. **Gast** speichert nichts und fängt jedes Mal leer an. Oben rechts wechselst du das Profil. Jedes
Profil hat seine eigene Farbe. Die Namen stehen in `src/profiles.rs`.

Die Profile haben keine PIN. Sie trennen nur den Stand und schützen nichts.
Unter „Wer schaut?“ → „Profilbilder ändern“ wählt jedes Profil ein **eigenes Bild** (wird quadratisch zugeschnitten und
verkleinert), einen von 6 Anime-Köpfen (Sonne, Nacht, Kirsche, Schnee, Herbst, Meer), eins von 10 Symbolen (Katze, Fuchs,
Panda, Pinguin, Eule, Roboter, Astronaut, Geist, Alien, Frosch) oder den Anfangsbuchstaben. Das Bild gilt auf allen Geräten.

## Kino (Kinogo)

- Die Startseite fragt, was du schauen willst: **Anime** (AniWorld, `#/anime`) oder **Kino** (Kinogo, `#/kinogo`).
  Darunter steht „Weiter schauen“ aus beiden Bereichen.
- Kino zeigt Filmlisten von kinogo.ec (Startseite, Neu, Filme, Top, Genres), eine Suche (russische Titel) und Filmseiten.
- kinogo.ec liegt hinter Cloudflare: Die Seiten holt Sina über den Browser im Container. Hängt die Prüfung, löst du sie
  einmal auf der Browser-Seite (⚙ Einstellungen → Browser).
- Abgespielt wird über den Player „nextembed“ (HLS mit mehreren Tonspuren). Sina wählt Englisch, wenn es das gibt,
  die Tonspur lässt sich unter dem Player wechseln. Serien von Kinogo werden noch nicht unterstützt.

## Geräte & Fernbedienung

- **Fernseher-Modus:** Auf Samsung (Tizen), LG (webOS), Sony, Fire TV, Android TV, Roku und ähnlichen Geräten schaltet
  Sina in den Fernseher-Modus: größere Oberfläche, deutlicher Fokusrahmen, Steuerung mit der Fernbedienung.
  Mit `?tv=1` an der Adresse oder unter ⚙ Einstellungen → „Dieses Gerät“ lässt sich der Modus erzwingen oder abschalten.
- **TV-Steuerkreuz:** Pfeiltasten bewegen den Fokus, OK öffnet, Zurück geht zurück. Im Player spulen links/rechts ±10 s,
  OK pausiert, die Medientasten (Play/Pause, Vor, Zurück) gehen auch.
- **Auf anderem Gerät weiter schauen:** Jedes offene Sina (Fernseher, Mac, PC, Tablet, Handy) meldet sich mit seinem
  gewählten Profil an. Oben, in der Player-Leiste und auf der Folgenseite gibt es immer den Knopf „Auf anderem Gerät weiter schauen“. Es werden nur Geräte
  mit **demselben Profil** gezeigt. Der Empfänger öffnet dieselbe Folge an derselben Stelle, der Sender pausiert.
- **Handy als Fernbedienung:** Nach dem Senden (oder über das Geräte-Symbol oben) öffnet sich auf dem Handy eine
  Fernbedienung für das andere Gerät: Play/Pause, Zeitleiste & ±10 s / ±30 s Vorspulen, Intro/Outro überspringen,
  Vorherige/Nächste Folge, Auto-Weiter ein/aus, Geschwindigkeit (0,5×–3×), Qualität (Auto/1080p/720p …), Lautstärke/Stumm,
  Sprachausgleich, Vollbild sowie „Auf diesem Gerät weiter schauen“ zum Zurückholen.
- Nach dem Senden spielt das Handy selbst nicht mehr: Der Player zeigt „Läuft auf …“ mit den Knöpfen „Fernbedienung“
  und „Hier abspielen“ (holt die Folge an der aktuellen Stelle zurück).
- Die Verbindung läuft über den Sina-Server (Long-Poll, nichts wird gespeichert). Beide Geräte müssen dieselbe
  Sina-Adresse öffnen.

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
| `VPN_DIR` | `/vpn` | Ordner für die hochgeladene VPN-Datei (geteilt mit gluetun) |
| `REQUIRE_ACCESS` | `0` | `1` = Einstellungen nur mit Cloudflare-Access-Kopfzeile |

Das Browser-Profil (Cookies, gelöste Prüfungen) liegt im Volume `/data`.

## VPN (z. B. Proton)

Der Hoster bindet die Stream-Adresse an die IP, die die Hoster-Seite geholt hat. Server und Chromium müssen deshalb
**über denselben Ausgang** ins Netz. Eine VPN-Erweiterung im Chromium reicht nicht: sie schützt nur den Browser,
das Video lädt aber der Server.

1. `docker-compose.vpn.example.yml` als `docker-compose.yml` nutzen und starten. Es startet einen gluetun-Container
   und setzt `HTTPS_PROXY`. Der Chromium nutzt automatisch denselben Proxy (oder `BROWSER_PROXY`).
2. In Sina oben auf das Zahnrad klicken (`/#/settings`) und die **WireGuard-`.conf`** hochladen oder den Inhalt
   einfügen. Schlüssel, Adresse und Server liest Sina selbst aus der Datei. Danach verbindet gluetun neu.
3. Proton: account.proton.me → Downloads → WireGuard-Konfiguration → Plattform „GNU/Linux“ → Server wählen → Herunterladen.

**Mehrere VPN-Configs & automatischer Wechsel:** Du kannst beliebig viele `.conf`-Dateien hinzufügen (auch mehrere auf
einmal). Sina prüft alle 20 s, ob die aktive Verbindung ins Netz kommt. Nach 3 Fehlschlägen hintereinander (oder wenn sie
länger als 6 s braucht) wechselt Sina zur nächsten Config, bevorzugt zu der mit den wenigsten Problemen bisher. Nach
einem Wechsel wartet Sina 75 s, bis gluetun neu verbunden hat. Den Auto-Wechsel kannst du abschalten und jede Config auch
von Hand aktivieren.

**Alles übers VPN:** Mit gesetztem `HTTPS_PROXY` gehen Seiten, Hoster, Streams, Bilder (auch die von Kinogo) und die
Namensauflösung (DNS-over-HTTPS) durch den Proxy. Ist das VPN weg, schlagen die Anfragen fehl, statt ohne VPN rauszugehen.

Die Einstellungsseite zeigt die IP von Server und Browser und ob sie gleich sind.
Der Private Key wird nie wieder an den Browser zurückgegeben. Mit `REQUIRE_ACCESS=1` ist die Einstellungsseite nur
über Cloudflare Access erreichbar.

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
