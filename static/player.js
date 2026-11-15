/**
 * NINA Custom Video Player (Material 3 Expressive)
 * Styled exactly to match NINA's look & feel ("Meine Serien").
 * Strictly restricted to AniWorld & S.to (never active on unrelated websites).
 */

(function () {
  'use strict';

  // 1. Strict Domain & Frame Guard: NEVER run on external sites (YouTube, Netflix, etc.)
  const ANI = /(^|\.)(aniworld\.to|s\.to|serienstream\.to|filmo\.to|ani\.pm|settlar\.io)$/i;
  const isTop = window.self === window.top;
  const hostOf = (u) => { try { return new URL(u).hostname; } catch (_) { return ''; } };
  const topHost = () => {
    try { const o = location.ancestorOrigins; if (o && o.length) return hostOf(o[o.length - 1]); } catch (_) {}
    return hostOf(document.referrer);
  };
  const onSite = ANI.test(location.hostname);
  const inEmbed = !isTop && ANI.test(topHost());
  

  // Prevent multiple initializations in the same frame
  if (window.__A16_PLAYER_INITIALIZED__) return;
  window.__A16_PLAYER_INITIALIZED__ = true;

  // Material 3 Symbols (NINA Style)
  const ICONS = {
    play: '<svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>',
    pause: '<svg viewBox="0 0 24 24"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>',
    replay10: '<svg viewBox="0 0 24 24"><path d="M12 5V1L7 6l5 5V7c3.31 0 6 2.69 6 6s-2.69 6-6 6-6-2.69-6-6H4c0 4.42 3.58 8 8 8s8-3.58 8-8-3.58-8-8-8zm-1.1 11H10v-3.3l-.9.6-.5-.7 1.6-1.1h.7v4.5zm3.6-2.3c0 .5-.1.9-.3 1.2-.2.3-.5.5-.9.5s-.7-.2-.9-.5c-.2-.3-.3-.7-.3-1.2v-1c0-.5.1-.9.3-1.2.2-.3.5-.5.9-.5s.7.2.9.5c.2.3.3.7.3 1.2v1zm-.8-1.1c0-.4 0-.7-.1-.9s-.2-.3-.4-.3-.3.1-.4.3-.1.5-.1.9v1.2c0 .4 0 .7.1.9s.2.3.4.3.3-.1.4-.3.1-.5.1-.9v-1.2z"/></svg>',
    forward10: '<svg viewBox="0 0 24 24"><path d="M12 5V1l5 5-5 5V7c-3.31 0-6 2.69-6 6s2.69 6 6 6 6-2.69 6-6h2c0 4.42-3.58 8-8 8s-8-3.58-8-8 3.58-8 8-8zm-1.1 11H10v-3.3l-.9.6-.5-.7 1.6-1.1h.7v4.5zm3.6-2.3c0 .5-.1.9-.3 1.2-.2.3-.5.5-.9.5s-.7-.2-.9-.5c-.2-.3-.3-.7-.3-1.2v-1c0-.5.1-.9.3-1.2.2-.3.5-.5.9-.5s.7.2.9.5c.2.3.3.7.3 1.2v1zm-.8-1.1c0-.4 0-.7-.1-.9s-.2-.3-.4-.3-.3.1-.4.3-.1.5-.1.9v1.2c0 .4 0 .7.1.9s.2.3.4.3.3-.1.4-.3.1-.5.1-.9v-1.2z"/></svg>',
    volumeHigh: '<svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/></svg>',
    volumeLow: '<svg viewBox="0 0 24 24"><path d="M18.5 12c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM5 9v6h4l5 5V4L9 9H5z"/></svg>',
    volumeMute: '<svg viewBox="0 0 24 24"><path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"/></svg>',
    speed: '<svg viewBox="0 0 24 24"><path d="M20.38 8.57l-1.23 1.85a8 8 0 0 1-.22 7.58H5.07A8 8 0 0 1 15.58 6.85l1.85-1.23A10 10 0 0 0 3.35 19a2 2 0 0 0 1.72 1h13.85a2 2 0 0 0 1.74-1 10 10 0 0 0-.27-10.44zm-9.79 6.84a2 2 0 0 0 2.83 0l5.66-8.49-8.49 5.66a2 2 0 0 0 0 2.83z"/></svg>',
    pip: '<svg viewBox="0 0 24 24"><path d="M19 11h-8v6h8v-6zm4 8V4.98C23 3.88 22.1 3 21 3H3c-1.1 0-2 .88-2 1.98V19c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2zm-2 .02H3V4.97h18v14.05z"/></svg>',
    fullscreen: '<svg viewBox="0 0 24 24"><path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z"/></svg>',
    fullscreenExit: '<svg viewBox="0 0 24 24"><path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z"/></svg>',
    close: '<svg viewBox="0 0 24 24"><path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>'
  };

  function formatTime(seconds) {
    if (isNaN(seconds) || !isFinite(seconds) || seconds < 0) return '0:00';
    const s = Math.floor(seconds % 60);
    const m = Math.floor((seconds / 60) % 60);
    const h = Math.floor(seconds / 3600);
    const pad = (n) => String(n).padStart(2, '0');
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
  }

  // Phones / tablets: no hover, finger instead of mouse
  const TOUCH = (() => {
    try {
      return matchMedia('(hover: none) and (pointer: coarse)').matches ||
             ('ontouchstart' in window) ||
             (typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0);
    } catch (_) { return false; }
  })();

  // Changes only the text node's value: no childList mutation, so the page's MutationObservers
  // (scraper, site.js, the video scan) don't wake up four times a second during playback.
  function setText(el, s) {
    if (!el) return;
    const n = el.firstChild;
    if (n && n.nodeType === 3 && !n.nextSibling) { if (n.nodeValue !== s) n.nodeValue = s; }
    else el.textContent = s;
  }

  // Neuter hoster players (JWPlayer, Plyr, VideoJS)
  function suppressHosterElements(container) {
    if (!container) return;
    const selectors = [
      '.jw-controls', '.jw-controlbar', '.jw-overlays', '.jw-preview',
      '.jw-title', '.jw-dock', '.jw-display-icon-container', '.jw-display',
      '.jw-nextup-container', '.jw-settings-menu', '.jw-rightclick', '.jw-logo',
      '.plyr__controls', '.plyr__poster', '.vjs-control-bar', '.vjs-big-play-button',
      '.glassplayer-controls', '.glassplayer-ui', '.p-controls', '.p-toolbar', '.click-layer'
    ];
    selectors.forEach(sel => {
      const els = container.querySelectorAll ? container.querySelectorAll(sel) : [];
      els.forEach(el => {
        try { el.remove(); } catch (e) { el.style.setProperty('display', 'none', 'important'); }
      });
    });
  }

  // ani.pm / settlar.io ship their own React player with its own control bar. Wrapping only the <video>
  // leaves that bar as a sibling drawn over our dock, so there we mount into the real player container
  // and hide every native control around the video.
  const FOREIGN = false;
  const CONTROL_SEL = 'button, input, select, [role="button"], [role="slider"], [role="menu"], [role="toolbar"], svg';

  function findForeignRoot(video) {
    const vr = video.getBoundingClientRect();
    let el = video.parentElement;
    let fallback = el;
    for (let i = 0; el && i < 6 && el !== document.body && el !== document.documentElement; i++, el = el.parentElement) {
      const r = el.getBoundingClientRect();
      if (vr.width && (r.width > vr.width * 1.1 + 24 || r.height > vr.height * 1.25 + 48)) break;
      fallback = el;
      if (el.querySelector(CONTROL_SEL)) return el;
    }
    return fallback;
  }

  // Hides control-looking siblings along the video's ancestor chain; subtitle layers (no controls) stay.
  function suppressForeignControls(root, video, ui) {
    (function sweep(node) {
      for (const child of Array.from(node.children)) {
        if (child === ui || child === video || child.classList.contains('a16-controls-root')) continue;
        if (child.contains(video) || child.contains(ui)) { sweep(child); continue; }
        if (child.dataset.a16Hidden) continue;
        const tag = child.tagName;
        if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'TRACK' || tag === 'SOURCE') continue;
        if (child.matches(CONTROL_SEL) || child.querySelector(CONTROL_SEL)) {
          child.dataset.a16Hidden = '1';
          child.style.setProperty('display', 'none', 'important');
        }
      }
    })(root);
  }

  // ========================================================
  // MOUNT NINA VIDEO PLAYER
  // ========================================================
  function mountAndroid16Player(video) {
    if (!video || video.__A16_MOUNTED__) return;
    video.__A16_MOUNTED__ = true;
    video.classList.add('a16-managed-video');
    video.controls = false;
    video.playsInline = true;
    video.setAttribute('playsinline', '');
    video.setAttribute('webkit-playsinline', '');

    // Find parent container or create dedicated wrapper
    let wrapper = video.closest('.a16-player-wrapper') || video.closest('.jwplayer') || video.closest('.plyr') || video.closest('.watch-player') || video.closest('.glassplayer');
    if (FOREIGN && !video.closest('.a16-player-wrapper')) {
      wrapper = findForeignRoot(video);
      wrapper.classList.add('a16-foreign-root');
    } else if (!wrapper || !wrapper.classList.contains('a16-player-wrapper')) {
      if (wrapper && (wrapper.classList.contains('jwplayer') || wrapper.classList.contains('plyr'))) {
        wrapper.classList.add('a16-player-wrapper');
      } else {
        wrapper = document.createElement('div');
        wrapper.className = 'a16-player-wrapper';
        video.parentNode.insertBefore(wrapper, video);
        wrapper.appendChild(video);
      }
    }

    // Sync seed hue from NINA
    try {
      chrome.storage.local.get(['ninaHue'], (res) => {
        if (res && typeof res.ninaHue === 'number') {
          wrapper.style.setProperty('--h', res.ninaHue);
        }
      });
    } catch (_) {}

    // Setting "Pause: Bild verwischen": while paused the picture is blurred beyond recognition
    // (nobody walking by sees what is streamed); playing again clears it.
    const applyBlur = (on) => wrapper.classList.toggle('a16-pause-blur', !!on);
    try {
      chrome.storage.local.get(['aw_settings'], (r) => applyBlur(((r && r.aw_settings) || {}).pauseBlur));
      chrome.storage.onChanged.addListener((c, area) => {
        if (area === 'local' && c.aw_settings) applyBlur((c.aw_settings.newValue || {}).pauseBlur);
      });
    } catch (_) {}

    suppressHosterElements(wrapper);
    if (TOUCH) wrapper.classList.add('a16-touch');
    // small player (phone upright): tighter controls
    if (typeof ResizeObserver === 'function') {
      new ResizeObserver((ents) => {
        const r = ents[ents.length - 1].contentRect;
        wrapper.classList.toggle('a16-compact', r.height > 0 && (r.height < 320 || r.width < 480));
      }).observe(wrapper);
    }

    // Build NINA Material 3 Expressive UI DOM (exact match to screenshot)
    const ui = document.createElement('div');
    ui.className = 'a16-controls-root';
    ui.innerHTML = `
      <!-- Double-Click Skip Ripples -->
      <div class="a16-ripple-overlay left" id="rippleLeft">
        <div class="a16-ripple-content">
          ${ICONS.replay10} <span>10s</span>
        </div>
      </div>
      <div class="a16-ripple-overlay right" id="rippleRight">
        <div class="a16-ripple-content">
          <span>10s</span> ${ICONS.forward10}
        </div>
      </div>

      <!-- Top Bar: NINA Badge + Series Title + 1080p FHD Quality Badge + Close Button -->
      <div class="a16-top-bar">
        <div class="a16-top-left">
          <div class="a16-brand-chip">
            <span class="a16-brand-logo">N</span>
            <span class="a16-brand-name">NINA</span>
          </div>
          <div class="a16-video-title" id="a16Title">Video Stream</div>
        </div>
        <div class="a16-top-right">
          <div class="a16-quality-chip" id="qualityChip" title="Stream-Qualität">1080p FHD</div>
          <button class="a16-close-btn" id="btnA16Close" title="Schließen / Vollbild">${ICONS.close}</button>
        </div>
      </div>

      <!-- Phones only: big play + 10 s back / forward in the middle -->
      <div class="a16-center-actions">
        <button class="a16-secondary-fab" id="btnCenterBack" title="10 s zurück">${ICONS.replay10}</button>
        <button class="a16-center-fab" id="btnCenterPlay" title="Play / Pause">${ICONS.play}</button>
        <button class="a16-secondary-fab" id="btnCenterFwd" title="10 s vor">${ICONS.forward10}</button>
      </div>

      <!-- NINA Circular Loading Ring -->
      <div class="a16-loading-overlay" id="a16LoadingSpinner">
        <div class="a16-spinner-ring">
          <svg class="a16-spinner-svg" viewBox="0 0 50 50">
            <circle class="a16-spinner-track" cx="25" cy="25" r="20" fill="none" stroke-width="4"></circle>
            <circle class="a16-spinner-arc" cx="25" cy="25" r="20" fill="none" stroke-width="4"></circle>
          </svg>
        </div>
        <div class="a16-loading-text" id="a16LoadingText">Stream wird geladen...</div>
      </div>

      <!-- small spinner: buffering / seeking while playing -->
      <div class="a16-buffer-spin" id="a16BufferSpin" aria-hidden="true">
        <svg viewBox="0 0 50 50"><circle class="bs-track" cx="25" cy="25" r="20" fill="none" stroke-width="5"></circle><circle class="bs-arc" cx="25" cy="25" r="20" fill="none" stroke-width="5"></circle></svg>
      </div>

      <!-- Auto-Next Episode Card (Bottom Right) -->
      <div class="a16-next-episode-card hidden" id="a16NextCard">
        <div class="a16-next-top">
          <small class="a16-next-label">NÄCHSTE FOLGE IN <span id="a16NextSec">7</span> S</small>
          <b class="a16-next-title" id="a16NextTitle">Nächste Folge</b>
        </div>
        <div class="a16-next-progress-bar">
          <div class="a16-next-progress-fill" id="a16NextFill"></div>
        </div>
        <div class="a16-next-buttons">
          <button class="a16-next-cancel" id="btnCancelNext">Abbrechen</button>
          <button class="a16-next-confirm" id="btnConfirmNext">${ICONS.play} Jetzt</button>
        </div>
      </div>

      <!-- Bottom Floating Island Dock (Slim Single Row) -->
      <div class="a16-bottom-dock">
        <!-- Left: Play FAB & Volume (up to 600%) -->
        <div class="a16-dock-left">
          <button class="a16-btn-play-dock" id="btnPlay" title="Play / Pause">
            ${ICONS.play}
          </button>
          <div class="a16-volume-wrapper">
            <button class="a16-btn" id="btnVolume" title="Stumm / Ton">
              ${ICONS.volumeHigh}
            </button>
            <div class="a16-volume-slider-box">
              <input type="range" class="a16-volume-slider" id="volumeSlider" min="0" max="6" step="0.05" value="1">
              <span class="a16-volume-text" id="volumeText">100%</span>
            </div>
          </div>
          <button class="a16-btn a16-dyn-audio-btn" id="btnDynAudio" title="Dynamische Lautstärke (Leise Sprache lauter, laute Effekte leiser)">
            <svg viewBox="0 0 24 24"><path d="M3 17h2v-4H3v4zm4 4h2V7H7v14zm4-8h2V3h-2v10zm4 4h2V9h-2v8zm4-2h2v-4h-2v4z"/></svg>
          </button>
        </div>

        <!-- Center: Current Time + Seek Bar + Duration (In between Volume and Speed) -->
        <div class="a16-dock-center">
          <span class="a16-time-cur" id="timeCur">0:00</span>
          <div class="a16-timeline-track" id="a16Timeline">
            <div class="a16-timeline-bg"></div>
            <div class="a16-timeline-buffer" id="a16Buffer"></div>
            <div class="a16-timeline-played" id="a16Played">
              <div class="a16-timeline-thumb"></div>
            </div>
            <div class="a16-prev" id="a16Prev" aria-hidden="true"><canvas width="224" height="126"></canvas></div>
            <div class="a16-timeline-tooltip" id="a16Tooltip">0:00</div>
          </div>
          <span class="a16-time-dur" id="timeDur">0:00</span>
        </div>

        <!-- Right: Speed + Fullscreen (PiP removed, Quality moved to top) -->
        <div class="a16-dock-right">
          <div class="a16-speed-chip" id="speedChip" title="Geschwindigkeit">${ICONS.speed} 1x</div>
          <button class="a16-btn a16-cast-btn" id="btnCast" title="Auf anderem Gerät weiter schauen">
            <svg viewBox="0 0 24 24"><path d="M21 3H3c-1.1 0-2 .9-2 2v3h2V5h18v14h-7v2h7c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11z"/></svg>
          </button>
          <button class="a16-btn" id="btnFullscreen" title="Vollbild">
            ${ICONS.fullscreen}
          </button>
        </div>
      </div>

      <!-- Speed Popover Menu (NINA Sync) -->
      <div class="a16-menu-popover a16-speed-popover hidden" id="speedPopover">
        <div class="a16-menu-header">Wiedergabegeschwindigkeit</div>
        <div class="a16-speed-grid">
          <div class="a16-menu-item" data-speed="0.25">0.25x</div>
          <div class="a16-menu-item" data-speed="0.5">0.5x</div>
          <div class="a16-menu-item" data-speed="0.75">0.75x</div>
          <div class="a16-menu-item active" data-speed="1.0">1.0x (Normal)</div>
          <div class="a16-menu-item" data-speed="1.25">1.25x</div>
          <div class="a16-menu-item" data-speed="1.5">1.5x</div>
          <div class="a16-menu-item" data-speed="1.75">1.75x</div>
          <div class="a16-menu-item" data-speed="2.0">2.0x</div>
          <div class="a16-menu-item" data-speed="2.5">2.5x</div>
          <div class="a16-menu-item" data-speed="3.0">3.0x</div>
          <div class="a16-menu-item" data-speed="3.5">3.5x</div>
          <div class="a16-menu-item" data-speed="4.0">4.0x</div>
        </div>
      </div>

      <!-- Quality Popover Menu -->
      <div class="a16-menu-popover a16-quality-popover hidden" id="qualityPopover">
        <div class="a16-menu-header">Videoqualität</div>
        <div class="a16-quality-grid" id="qualityGrid">
          <div class="a16-menu-item active" data-quality="auto">Automatisch (Auto)</div>
          <div class="a16-menu-item" data-quality="1080">1080p FHD</div>
          <div class="a16-menu-item" data-quality="720">720p HD</div>
          <div class="a16-menu-item" data-quality="480">480p SD</div>
          <div class="a16-menu-item" data-quality="360">360p SD</div>
        </div>
      </div>

      <!-- Skip Intro / Outro Floating Button -->
      <button class="a16-skip-btn hidden" id="btnSkipIntro" title="Überspringen">
        <svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>
        <span id="txtSkipBtn">Intro überspringen (+85s)</span>
      </button>
    `;

    wrapper.appendChild(ui);

    // References
    const btnPlay = ui.querySelector('#btnPlay');
    const btnCenterPlay = ui.querySelector('#btnCenterPlay');
    const btnCenterBack = ui.querySelector('#btnCenterBack');
    const btnCenterFwd = ui.querySelector('#btnCenterFwd');
    const volumeWrapper = ui.querySelector('.a16-volume-wrapper');
    const btnVolume = ui.querySelector('#btnVolume');
    const volumeSlider = ui.querySelector('#volumeSlider');
    const volumeText = ui.querySelector('#volumeText');
    const btnDynAudio = ui.querySelector('#btnDynAudio');
    const timeCur = ui.querySelector('#timeCur');
    const timeDur = ui.querySelector('#timeDur');
    const timeline = ui.querySelector('#a16Timeline');
    const playedBar = ui.querySelector('#a16Played');
    const bufferBar = ui.querySelector('#a16Buffer');
    const tooltip = ui.querySelector('#a16Tooltip');
    const btnFullscreen = ui.querySelector('#btnFullscreen');
    const speedChip = ui.querySelector('#speedChip');
    const qualityChip = ui.querySelector('#qualityChip');
    const speedPopover = ui.querySelector('#speedPopover');
    const qualityPopover = ui.querySelector('#qualityPopover');
    const qualityGrid = ui.querySelector('#qualityGrid');
    const btnSkipIntro = ui.querySelector('#btnSkipIntro');
    const txtSkipBtn = ui.querySelector('#txtSkipBtn');
    const rippleLeft = ui.querySelector('#rippleLeft');
    const rippleRight = ui.querySelector('#rippleRight');

    // --- Quality Management (1080p FHD, 720p HD, 480p SD, 360p SD, Auto) ---
    let currentQualityPref = 'auto';
    try {
      const savedQ = localStorage.getItem('nina_preferred_quality');
      if (savedQ) currentQualityPref = savedQ;
    } catch (_) {}

    function formatQualityLabel(h, isAuto) {
      if (!h || h <= 0) return isAuto ? 'Auto' : '1080p FHD';
      if (h >= 1080) return isAuto ? `${h}p FHD` : `${h}p FHD`;
      if (h >= 720) return isAuto ? `${h}p HD` : `${h}p HD`;
      return isAuto ? `${h}p SD` : `${h}p SD`;
    }

    function updateQualityBadge(customText) {
      if (!qualityChip) return;
      if (customText) {
        qualityChip.textContent = customText;
        return;
      }
      if (currentQualityPref !== 'auto') {
        const h = parseInt(currentQualityPref, 10);
        qualityChip.textContent = formatQualityLabel(h, false);
      } else {
        const h = video.videoHeight > 0 ? video.videoHeight : 1080;
        qualityChip.textContent = formatQualityLabel(h, false);
      }
    }

    function renderQualityMenu() {
      if (!qualityGrid) return;
      qualityGrid.innerHTML = '';

      // 1. Automatisch (Auto)
      const autoItem = document.createElement('div');
      autoItem.className = 'a16-menu-item' + (currentQualityPref === 'auto' ? ' active' : '');
      autoItem.textContent = 'Automatisch (Auto)';
      autoItem.dataset.quality = 'auto';
      autoItem.onclick = (e) => {
        e.stopPropagation();
        selectQuality('auto');
      };
      qualityGrid.appendChild(autoItem);

      // 2. Discover available resolutions from HLS levels or standard suite
      const hls = video.__hlsInstance || window.__A16_HLS__;
      const hlsLevels = (hls && hls.levels && hls.levels.length > 0) ? hls.levels : (wrapper.__hlsLevels || video.__hlsLevels || []);

      const heights = [1080, 720, 480, 360];
      if (Array.isArray(hlsLevels)) {
        hlsLevels.forEach(lvl => {
          const h = lvl.height || 0;
          if (h > 0 && !heights.includes(h)) heights.push(h);
        });
      }
      heights.sort((a, b) => b - a);

      heights.forEach(h => {
        const item = document.createElement('div');
        item.className = 'a16-menu-item' + (currentQualityPref === String(h) ? ' active' : '');
        item.textContent = formatQualityLabel(h, false);
        item.dataset.quality = String(h);
        item.onclick = (e) => {
          e.stopPropagation();
          selectQuality(String(h));
        };
        qualityGrid.appendChild(item);
      });
    }

    function selectQuality(qualityKey) {
      currentQualityPref = qualityKey;
      try {
        localStorage.setItem('nina_preferred_quality', qualityKey);
      } catch (_) {}

      // Update active classes
      if (qualityGrid) {
        qualityGrid.querySelectorAll('.a16-menu-item').forEach(it => {
          if (it.dataset.quality === qualityKey) {
            it.classList.add('active');
          } else {
            it.classList.remove('active');
          }
        });
      }

      const hls = video.__hlsInstance || window.__A16_HLS__;
      const hlsLevels = (hls && hls.levels && hls.levels.length > 0) ? hls.levels : (wrapper.__hlsLevels || video.__hlsLevels || []);

      if (qualityKey === 'auto') {
        if (hls) {
          hls.autoLevelEnabled = true;
          hls.currentLevel = -1;
          hls.loadLevel = -1;
          hls.nextLevel = -1;
        }
        updateQualityBadge();
        if (typeof wrapper.__onSelectQuality === 'function') {
          wrapper.__onSelectQuality(-1);
        }
      } else {
        const targetH = parseInt(qualityKey, 10);
        updateQualityBadge(formatQualityLabel(targetH, false));

        if (hls && hlsLevels.length > 0) {
          let bestIdx = 0;
          let minDiff = Infinity;
          hlsLevels.forEach((lvl, idx) => {
            const h = lvl.height || 0;
            if (h > 0) {
              const diff = Math.abs(h - targetH);
              if (diff < minDiff) {
                minDiff = diff;
                bestIdx = idx;
              }
            }
          });

          hls.autoLevelEnabled = false;
          hls.currentLevel = bestIdx;
          hls.loadLevel = bestIdx;
          hls.nextLevel = bestIdx;

          if (typeof wrapper.__onSelectQuality === 'function') {
            wrapper.__onSelectQuality(bestIdx);
          }
        }

        // Buffer optimization for lower resolutions
        if (hls && hls.config) {
          if (targetH <= 480) {
            hls.config.maxBufferLength = 25;
            hls.config.maxBufferSize = 20 * 1024 * 1024;
          } else if (targetH <= 720) {
            hls.config.maxBufferLength = 45;
            hls.config.maxBufferSize = 40 * 1024 * 1024;
          } else {
            hls.config.maxBufferLength = 90;
            hls.config.maxBufferSize = 100 * 1024 * 1024;
          }
        }
      }

      qualityPopover.classList.add('hidden');
    }

    wrapper.__setQualityBadge = function (txt) {
      if (qualityChip && txt) qualityChip.textContent = txt;
    };
    video.__setQualityBadge = wrapper.__setQualityBadge;

    wrapper.__updateQualityLevels = function (levels, activeIdx, onSelect) {
      if (levels && levels.length) {
        wrapper.__hlsLevels = levels;
        video.__hlsLevels = levels;
      }
      if (typeof onSelect === 'function') {
        wrapper.__onSelectQuality = onSelect;
      }
      renderQualityMenu();
      if (currentQualityPref !== 'auto') {
        selectQuality(currentQualityPref);
      } else {
        updateQualityBadge();
      }
    };
    video.__updateQualityLevels = wrapper.__updateQualityLevels;

    qualityChip.addEventListener('click', (e) => {
      e.stopPropagation();
      speedPopover.classList.add('hidden');
      renderQualityMenu();
      qualityPopover.classList.toggle('hidden');
    });

    function onVideoDimensionsReady() {
      if (video && video.videoHeight > 0) {
        updateQualityBadge();
        renderQualityMenu();
      }
    }
    video.addEventListener('resize', onVideoDimensionsReady);
    video.addEventListener('loadedmetadata', onVideoDimensionsReady);
    video.addEventListener('canplay', onVideoDimensionsReady);
    video.addEventListener('playing', onVideoDimensionsReady);
    video.addEventListener('timeupdate', () => {
      if (!video.__qualityDetected && video.videoHeight > 0) {
        video.__qualityDetected = true;
        onVideoDimensionsReady();
      }
    });

    renderQualityMenu();
    updateQualityBadge();
    const a16LoadingSpinner = ui.querySelector('#a16LoadingSpinner');
    const a16LoadingText = ui.querySelector('#a16LoadingText');
    const a16Title = ui.querySelector('#a16Title');
    const btnClose = ui.querySelector('#btnA16Close');
    const nextCard = ui.querySelector('#a16NextCard');
    const nextSec = ui.querySelector('#a16NextSec');
    const nextTitle = ui.querySelector('#a16NextTitle');
    const btnCancelNext = ui.querySelector('#btnCancelNext');
    const btnConfirmNext = ui.querySelector('#btnConfirmNext');

    // Set series title formatted like in screenshot: "Boruto - Naruto Next Generations S01E001"
    function updateTitle() {
      try {
        chrome.runtime.sendMessage({ type: 'AW', op: 'ctx' }, (res) => {
          if (!chrome.runtime.lastError && res && res.ok && res.result) {
            const ctx = res.result;
            if (ctx.series) {
              const epStr = ctx.movie ? 'Film ' + ctx.episode : 'S' + String(ctx.season).padStart(2, '0') + 'E' + String(ctx.episode).padStart(3, '0');
              a16Title.textContent = ctx.series + ' ' + epStr + (ctx.title ? ' · ' + ctx.title : '');
              return;
            }
          }
          // Fallback to page title
          const h1 = document.querySelector('.ytw-meta h1')?.textContent?.trim() || 
                     document.querySelector('.watch-under h1')?.textContent?.trim() || 
                     document.querySelector('h1')?.textContent?.trim() || 
                     document.querySelector('.ns-ep-title')?.textContent?.trim() || 
                     document.title || '';
          a16Title.textContent = h1.replace(/AniWorld\.to/i, '').replace(/ani\.pm/i, '').replace(/Watch\s*/i, '').trim() || 'NINA Video';
        });
      } catch (e) {}
    }
    updateTitle();

    // Loading indicator (clean, silent background buffering)
    function showLoading(text) {
      // Only show during initial stream connection if video has not yet started
      if (a16LoadingSpinner && video.currentTime === 0 && (video.paused || video.readyState < 2)) {
        if (text && a16LoadingText) a16LoadingText.textContent = text;
        a16LoadingSpinner.classList.remove('hidden');
        ui.classList.add('is-loading');
      }
    }

    function hideLoading() {
      if (a16LoadingSpinner) {
        a16LoadingSpinner.classList.add('hidden');
        ui.classList.remove('is-loading');
      }
    }

    // small ring in the middle whenever the player waits for data (stall, seek, quality change)
    const bufSpin = ui.querySelector('#a16BufferSpin');
    let bufTimer = null;
    const bufOff = () => { clearTimeout(bufTimer); bufTimer = null; if (bufSpin) bufSpin.classList.remove('on'); };
    const bufOn = () => {
      if (!bufSpin || bufTimer || !(video.currentTime > 0 || video.seeking)) return;
      // only when it really hangs: short hiccups don't flicker
      bufTimer = setTimeout(() => {
        bufTimer = null;
        if (video.seeking || (!video.paused && video.readyState < 3)) bufSpin.classList.add('on');
      }, 350);
    };
    ['waiting', 'stalled', 'seeking'].forEach((ev) => video.addEventListener(ev, bufOn));
    ['playing', 'canplay', 'seeked', 'pause', 'ended', 'emptied'].forEach((ev) => video.addEventListener(ev, () => {
      if (ev === 'canplay' && video.readyState < 3) return;
      bufOff();
    }));
    video.addEventListener('timeupdate', () => { if (bufSpin && bufSpin.classList.contains('on') && !video.seeking && video.readyState >= 3 && !video.paused) bufOff(); });

    wrapper.__a16ShowLoading = showLoading;
    wrapper.__a16HideLoading = hideLoading;

    // Show initial loading prompt only on initial load
    video.addEventListener('loadstart', () => {
      if (video.currentTime === 0) showLoading('Stream wird geladen...');
    });

    // As soon as stream is ready, playing or progressing: HIDE IMMEDIATELY!
    video.addEventListener('loadeddata', hideLoading);
    video.addEventListener('loadedmetadata', hideLoading);
    video.addEventListener('canplay', hideLoading);
    video.addEventListener('canplaythrough', hideLoading);
    video.addEventListener('playing', hideLoading);
    video.addEventListener('seeked', hideLoading);
    video.addEventListener('timeupdate', () => {
      if (!video.paused && video.currentTime > 0) {
        hideLoading();
      }
    });

    // Initial check: if already loaded or playing, ensure it stays hidden
    if (video.readyState >= 2 || video.currentTime > 0) {
      hideLoading();
    }

    // Play / Pause
    function togglePlay() {
      if (video.paused || video.ended) {
        video.play().catch(() => {});
      } else {
        video.pause();
      }
    }

    btnPlay.addEventListener('click', (e) => { e.stopPropagation(); togglePlay(); });
    if (btnCenterPlay) btnCenterPlay.addEventListener('click', (e) => { e.stopPropagation(); togglePlay(); });

    video.addEventListener('play', () => {
      btnPlay.innerHTML = ICONS.pause;
      if (btnCenterPlay) btnCenterPlay.innerHTML = ICONS.pause;
      startIdleTimer();
    });

    video.addEventListener('play', () => wrapper.classList.remove('a16-paused'));
    video.addEventListener('pause', () => { if (!video.ended) wrapper.classList.add('a16-paused'); });

    video.addEventListener('pause', () => {
      btnPlay.innerHTML = ICONS.play;
      if (btnCenterPlay) btnCenterPlay.innerHTML = ICONS.play;
      stopIdleTimer();
      showControls();
    });

    // Timeline Scrubbing & Seeking (Robust & Smooth)
    let isDragging = false;
    let dragPos = 0;

    function setPlayedWidth(pct) {
      const p = Math.max(0, Math.min(100, pct));
      playedBar.style.setProperty('width', `${p}%`, 'important');
    }

    function setBufferWidth(pct) {
      const p = Math.max(0, Math.min(100, pct));
      bufferBar.style.setProperty('width', `${p}%`, 'important');
    }

    function updateTimeline() {
      if (!isDragging && video.duration && isFinite(video.duration)) {
        const pct = (video.currentTime / video.duration) * 100;
        setPlayedWidth(pct);
        setText(timeCur, formatTime(video.currentTime));
        setText(timeDur, formatTime(video.duration));

        if (video.buffered.length > 0) {
          for (let i = video.buffered.length - 1; i >= 0; i--) {
            if (video.buffered.start(i) <= video.currentTime) {
              const bufPct = (video.buffered.end(i) / video.duration) * 100;
              setBufferWidth(bufPct);
              break;
            }
          }
        }
      }
    }

    video.addEventListener('timeupdate', updateTimeline);
    video.addEventListener('loadedmetadata', updateTimeline);
    video.addEventListener('durationchange', updateTimeline);
    video.addEventListener('progress', updateTimeline);
    video.addEventListener('canplay', updateTimeline);
    video.addEventListener('playing', updateTimeline);
    video.addEventListener('seeked', updateTimeline);

    function calcPosFromEvent(e) {
      const rect = timeline.getBoundingClientRect();
      if (!rect.width) return 0;
      const clientX = e.clientX !== undefined ? e.clientX : (e.touches && e.touches[0] ? e.touches[0].clientX : 0);
      return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    }

    function applySeek(pos) {
      if (video.duration && isFinite(video.duration)) {
        const targetTime = pos * video.duration;
        setPlayedWidth(pos * 100);
        setText(timeCur, formatTime(targetTime));
        video.currentTime = targetTime;
      }
    }

    function onTimelinePointerDown(e) {
      if (e.button !== undefined && e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      isDragging = true;
      timeline.classList.add('dragging');
      resetIdleTimer();
      try {
        if (timeline.setPointerCapture && e.pointerId !== undefined) {
          timeline.setPointerCapture(e.pointerId);
        }
      } catch (_) {}
      dragPos = calcPosFromEvent(e);
      applySeek(dragPos);
    }

    // ── Preview picture above the timeline ──
    // A second, muted, lowest-quality copy of the stream that only seeks to the hovered time.
    const prevBox = ui.querySelector('#a16Prev');
    const prevCanvas = prevBox && prevBox.querySelector('canvas');
    const prevCtx = prevCanvas && prevCanvas.getContext('2d');
    let pv = null, pvHls = null, pvSrc = '', pvWant = null, pvBusy = false, pvOk = false;
    function pvDestroy() {
      if (pvHls) { try { pvHls.destroy(); } catch (_) {} pvHls = null; }
      if (pv) { try { pv.removeAttribute('src'); pv.load(); } catch (_) {} pv = null; }
      pvSrc = ''; pvWant = null; pvBusy = false; pvOk = false;
      if (prevBox) prevBox.classList.remove('ready');
    }
    function pvSourceNow() {
      const hls = video.__hlsInstance;
      if (hls && hls.url) return { kind: 'hls', url: hls.url };
      const u = video.currentSrc || video.src || '';
      return u && !u.startsWith('blob:') ? { kind: 'file', url: u } : null;
    }
    function pvPaint() {
      if (!pv || !prevCtx || !pv.videoWidth) return;
      try {
        const w = prevCanvas.width, h = Math.round(w * pv.videoHeight / pv.videoWidth);
        if (prevCanvas.height !== h) prevCanvas.height = h;
        prevCtx.drawImage(pv, 0, 0, w, h);
        pvOk = true;
        prevBox.classList.add('ready');
      } catch (_) { pvOk = false; }   // cross-origin without CORS: no picture, the time stays
    }
    function pvNext() {
      if (!pv || pvBusy || pvWant == null) return;
      const t = pvWant; pvWant = null;
      if (Math.abs(pv.currentTime - t) < 0.4 && pvOk) return;
      pvBusy = true;
      try { pv.currentTime = t; } catch (_) { pvBusy = false; }
    }
    function pvEnsure() {
      const src = pvSourceNow();
      if (!src) return false;
      if (pv && pvSrc === src.url) return true;
      pvDestroy();
      pv = document.createElement('video');
      pv.muted = true; pv.preload = 'auto'; pv.playsInline = true; pv.crossOrigin = 'anonymous';
      pv.addEventListener('seeked', () => { pvBusy = false; pvPaint(); pvNext(); });
      pv.addEventListener('error', () => { pvBusy = false; prevBox.classList.remove('ready'); });
      pvSrc = src.url;
      if (src.kind === 'hls' && window.Hls && window.Hls.isSupported()) {
        pvHls = new window.Hls({ startLevel: 0, autoStartLoad: true, capLevelToPlayerSize: false, maxBufferLength: 3, maxMaxBufferLength: 6, backBufferLength: 0, enableWorker: true, abrEwmaDefaultEstimate: 500000 });
        pvHls.on(window.Hls.Events.MANIFEST_PARSED, () => { try { pvHls.currentLevel = 0; } catch (_) {} });
        pvHls.loadSource(src.url);
        pvHls.attachMedia(pv);
      } else {
        pv.src = src.url;
      }
      return true;
    }
    let pvLast = 0;
    function updatePreview(pos, t) {
      if (!prevBox) return;
      const W = timeline.clientWidth || 1, half = 114;
      prevBox.style.setProperty('left', Math.max(half, Math.min(W - half, pos * W)) + 'px', 'important');
      if (!pvEnsure()) { prevBox.classList.remove('ready'); return; }
      const now = Date.now();
      if (now - pvLast < 90) { pvWant = t; return; }
      pvLast = now; pvWant = t;
      if (pv.readyState >= 1) pvNext(); else pv.addEventListener('loadedmetadata', pvNext, { once: true });
    }
    video.addEventListener('loadstart', pvDestroy);
    video.addEventListener('emptied', pvDestroy);
    window.addEventListener('pagehide', pvDestroy);
    // free the second connection a few seconds after the mouse has left the timeline
    let pvIdle = null;
    timeline.addEventListener('pointerleave', () => { clearTimeout(pvIdle); pvIdle = setTimeout(pvDestroy, 20000); });
    timeline.addEventListener('pointerenter', () => clearTimeout(pvIdle));

    function onTimelinePointerMove(e) {
      const pos = calcPosFromEvent(e);
      if (video.duration && isFinite(video.duration)) {
        const hoverTime = pos * video.duration;
        // the preview loads a second copy of the stream: mouse only, a finger doesn't hover
        if (e.pointerType !== 'touch' && !TOUCH) updatePreview(pos, hoverTime);
        let extra = '';
        const hasDbSkips = !!(aniSkipTimes && (aniSkipTimes.op || aniSkipTimes.ed));

        if (hasDbSkips) {
          // Ausschließlich Datenbank-Skips auf der Timeline markieren
          if (aniSkipTimes.op && hoverTime >= aniSkipTimes.op.startTime && hoverTime <= aniSkipTimes.op.endTime) {
            extra = ' · Intro [Datenbank]';
          } else if (aniSkipTimes.ed && hoverTime >= aniSkipTimes.ed.startTime && hoverTime <= aniSkipTimes.ed.endTime) {
            extra = ' · Outro [Datenbank]';
          }
        } else if (aniSkipQueried || !aniSkipTimes) {
          // Nur wenn es keine Datenbank-Skips gibt: Algorithmus-Bereiche markieren
          if (video.duration > 120 && hoverTime >= 10 && hoverTime <= 145) {
            extra = ' · Intro [Algorithmus]';
          } else if (video.duration > 180 && hoverTime >= video.duration - 115 && hoverTime <= video.duration - 8) {
            extra = ' · Outro [Algorithmus]';
          }
        }

        setText(tooltip, formatTime(hoverTime) + extra);
        tooltip.style.setProperty('left', `${pos * 100}%`, 'important');
      }
      if (!isDragging) return;
      e.preventDefault();
      e.stopPropagation();
      dragPos = pos;
      setPlayedWidth(dragPos * 100);
      if (video.duration && isFinite(video.duration)) {
        setText(timeCur, formatTime(dragPos * video.duration));
      }
    }

    function onTimelinePointerUp(e) {
      if (!isDragging) return;
      e.preventDefault();
      e.stopPropagation();
      isDragging = false;
      timeline.classList.remove('dragging');
      resetIdleTimer();
      try {
        if (timeline.releasePointerCapture && e.pointerId !== undefined) {
          timeline.releasePointerCapture(e.pointerId);
        }
      } catch (_) {}
      dragPos = calcPosFromEvent(e);
      applySeek(dragPos);
    }

    timeline.addEventListener('pointerdown', onTimelinePointerDown);
    timeline.addEventListener('pointermove', onTimelinePointerMove);
    timeline.addEventListener('pointerup', onTimelinePointerUp);
    timeline.addEventListener('pointercancel', onTimelinePointerUp);

    // Global release fallback just in case pointer capture is lost
    window.addEventListener('pointerup', () => { isDragging = false; timeline.classList.remove('dragging'); }, { passive: true });
    window.addEventListener('mouseup', () => { isDragging = false; timeline.classList.remove('dragging'); }, { passive: true });

    timeline.addEventListener('click', (e) => {
      e.stopPropagation();
      applySeek(calcPosFromEvent(e));
    });

    // Skips (Center buttons & Double tap ripples)
    function skip(seconds, isLeft) {
      video.currentTime = Math.max(0, Math.min(video.duration || Infinity, video.currentTime + seconds));
      const rip = isLeft ? rippleLeft : rippleRight;
      const lbl = rip && rip.querySelector('span');
      if (lbl) lbl.textContent = Math.abs(Math.round(seconds)) + 's';
      rip.classList.remove('animate');
      void rip.offsetWidth;
      rip.classList.add('animate');
      setTimeout(() => rip.classList.remove('animate'), 550);
      resetIdleTimer();
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    }

    if (btnCenterBack) btnCenterBack.addEventListener('click', (e) => { e.stopPropagation(); skip(-10, true); });
    if (btnCenterFwd) btnCenterFwd.addEventListener('click', (e) => { e.stopPropagation(); skip(10, false); });

    // Mouse: a click on the video toggles Play / Pause.
    // Finger: a tap shows / hides the controls, a double tap on the left / right third jumps 10 s
    // (keep tapping to jump further), like YouTube / Netflix on the phone.
    const UI_SEL = '.a16-bottom-dock, .a16-top-bar, .a16-next-episode-card, .a16-menu-popover, .a16-skip-btn, .a16-center-fab, .a16-secondary-fab';
    const foreignRoot = FOREIGN && wrapper.classList.contains('a16-foreign-root');
    let lastPointerType = TOUCH ? 'touch' : 'mouse';
    wrapper.addEventListener('pointerdown', (e) => { lastPointerType = e.pointerType || lastPointerType; }, { capture: true, passive: true });
    let tapTimer = null, lastTapAt = 0, lastTapZone = 0;
    function onTouchTap(e) {
      const r = wrapper.getBoundingClientRect();
      const x = r.width ? (e.clientX - r.left) / r.width : 0.5;
      const zone = x < 0.35 ? -1 : x > 0.65 ? 1 : 0;
      const now = Date.now();
      if (zone && zone === lastTapZone && now - lastTapAt < 320) {
        clearTimeout(tapTimer); tapTimer = null;
        lastTapAt = now;
        skip(zone * 10, zone < 0);
        return;
      }
      lastTapAt = now; lastTapZone = zone;
      clearTimeout(tapTimer);
      const toggle = () => {
        tapTimer = null;
        if (ui.classList.contains('idle') || video.paused) resetIdleTimer();
        else { clearTimeout(idleTimer); ui.classList.add('idle'); }
      };
      if (zone) tapTimer = setTimeout(toggle, 260); else toggle();
    }
    function onSurfaceClick(e) {
      if (lastPointerType === 'touch') onTouchTap(e);
      else togglePlay();
    }
    if (foreignRoot) {
      // ani.pm has its own click layer / handlers on the video: take the click first (capture) so the
      // native player can't also toggle and cancel ours out.
      const swallow = (e) => {
        if (e.target.closest && e.target.closest(UI_SEL)) return false;
        e.stopPropagation();
        e.stopImmediatePropagation();
        return true;
      };
      ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'dblclick'].forEach((t) => wrapper.addEventListener(t, swallow, true));
      wrapper.addEventListener('click', (e) => {
        if (!swallow(e)) return;
        e.preventDefault();
        onSurfaceClick(e);
        if (lastPointerType !== 'touch') resetIdleTimer();
      }, true);
    } else {
      wrapper.addEventListener('click', (e) => {
        if (e.target.closest(UI_SEL)) {
          return;
        }
        onSurfaceClick(e);
      });
    }

    // ========================================================
    // VOLUME CONTROL (0% - 600%) & DYNAMIC AUDIO COMPRESSOR
    // (Hebt leise Sprache an, dämpft laute Knalle & Effekte)
    // ========================================================
    let audioCtx = null;
    let gainNode = null;
    let preAmpNode = null;
    let compressorNode = null;
    let mediaSource = null;

    const K_VOL = 'nina_player_volume';
    const K_MUTED = 'nina_player_muted';
    let currentVolume = 1.0;
    try {
      const savedVol = parseFloat(localStorage.getItem(K_VOL));
      if (!isNaN(savedVol) && savedVol >= 0 && savedVol <= 6) {
        currentVolume = savedVol;
      }
    } catch (_) {}

    let isDynamicAudioEnabled = false;
    try {
      isDynamicAudioEnabled = localStorage.getItem('nina_dynamic_audio') === '1';
    } catch (_) {}

    function updateDynAudioBtn() {
      if (!btnDynAudio) return;
      btnDynAudio.classList.toggle('active', isDynamicAudioEnabled);
      btnDynAudio.title = isDynamicAudioEnabled
        ? 'Dynamische Lautstärke: AKTIV (Leise Stimmen lauter, laute Szenen leiser)'
        : 'Dynamische Lautstärke: AUS (Klicken zum Aktivieren)';
    }
    updateDynAudioBtn();

    function initAudioGraph() {
      if (gainNode) return gainNode;
      const Ctor = window.AudioContext || window.webkitAudioContext;
      if (!Ctor) return null;
      try {
        try { video.crossOrigin = 'anonymous'; } catch (_) {}
        audioCtx = new Ctor();
        mediaSource = audioCtx.createMediaElementSource(video);

        preAmpNode = audioCtx.createGain();
        compressorNode = audioCtx.createDynamicsCompressor();
        // Hardware-accelerated speech/loudness leveling
        compressorNode.threshold.value = -28; // dB: Threshold above which sound is compressed
        compressorNode.knee.value = 24;       // dB: Smooth transition curve
        compressorNode.ratio.value = 14;      // Aggressive capping of loud spikes
        compressorNode.attack.value = 0.003;  // 3ms instant response to sudden loud sounds
        compressorNode.release.value = 0.22;  // 220ms smooth recovery for soft dialogue

        gainNode = audioCtx.createGain();     // Master volume (0 - 600%)
        gainNode.gain.value = currentVolume;

        routeAudio();
        return gainNode;
      } catch (err) {
        console.warn('[NINA Player] Audio graph initialization:', err);
        return null;
      }
    }

    function routeAudio() {
      if (!audioCtx || !mediaSource || !gainNode) return;
      try {
        mediaSource.disconnect();
        if (preAmpNode) preAmpNode.disconnect();
        if (compressorNode) compressorNode.disconnect();
        gainNode.disconnect();

        if (isDynamicAudioEnabled && preAmpNode && compressorNode) {
          // Pre-amp boosts soft whispers and dialogue by 2.2x into the compressor
          preAmpNode.gain.value = 2.2;
          mediaSource.connect(preAmpNode);
          preAmpNode.connect(compressorNode);
          compressorNode.connect(gainNode);
        } else {
          // Clean standard pass-through
          mediaSource.connect(gainNode);
        }
        gainNode.connect(audioCtx.destination);
      } catch (err) {
        console.warn('[NINA Player] Audio routing error:', err);
      }
    }

    function toggleDynamicAudio() {
      initAudioGraph();
      if (audioCtx && audioCtx.state === 'suspended') {
        audioCtx.resume().catch(() => {});
      }
      isDynamicAudioEnabled = !isDynamicAudioEnabled;
      try {
        localStorage.setItem('nina_dynamic_audio', isDynamicAudioEnabled ? '1' : '0');
      } catch (_) {}
      updateDynAudioBtn();
      routeAudio();
      showResumeIndicator(0, isDynamicAudioEnabled
        ? 'Dynamische Lautstärke: AN (Sprachausgleich aktiv)'
        : 'Dynamische Lautstärke: AUS');
    }

    if (btnDynAudio) {
      btnDynAudio.addEventListener('click', (e) => {
        e.stopPropagation();
        toggleDynamicAudio();
      });
      btnDynAudio.addEventListener('wheel', handleVolumeWheel, { passive: false });
    }

    function syncVolume() {
      try {
        const isMuted = localStorage.getItem(K_MUTED) === '1';
        video.muted = isMuted;
      } catch (_) {
        video.muted = false;
      }
      applyVolume(currentVolume);
    }

    video.addEventListener('loadedmetadata', syncVolume);
    video.addEventListener('canplay', syncVolume);

    video.addEventListener('play', () => {
      try {
        if (localStorage.getItem(K_MUTED) !== '1' && video.muted) {
          video.muted = false;
        }
      } catch (_) {}
      if (audioCtx && audioCtx.state === 'suspended') {
        audioCtx.resume().catch(() => {});
      }
      syncVolume();
    });

    // Automatically unlock Web Audio API & unmute on any user interaction on the page
    const unlockAudioOnGesture = () => {
      try {
        if (localStorage.getItem(K_MUTED) !== '1' && video.muted) {
          video.muted = false;
        }
      } catch (_) {}
      if (isDynamicAudioEnabled || currentVolume > 1.05) {
        initAudioGraph();
      }
      if (audioCtx && audioCtx.state === 'suspended') {
        audioCtx.resume().catch(() => {});
      }
      syncVolume();
    };
    ['click', 'pointerdown', 'keydown', 'touchstart'].forEach(ev => {
      window.addEventListener(ev, unlockAudioOnGesture, { capture: true, passive: true });
    });
    wrapper.addEventListener('click', unlockAudioOnGesture, { capture: true, passive: true });

    function applyVolume(val) {
      currentVolume = Math.max(0, Math.min(6, parseFloat(val) || 0));
      const pct = Math.round(currentVolume * 100);
      volumeSlider.value = currentVolume;
      volumeSlider.style.setProperty('--v', `${(currentVolume / 6) * 100}%`);
      if (volumeText) {
        setText(volumeText, `${pct}%`);
        volumeText.classList.toggle('boosted', currentVolume > 1.05);
      }

      if (video.muted || currentVolume === 0) {
        btnVolume.innerHTML = ICONS.volumeMute;
        btnVolume.title = 'Stumm (Klick zum Entstummen)';
      } else if (currentVolume < 0.5) {
        btnVolume.innerHTML = ICONS.volumeLow;
        btnVolume.title = `Lautstärke: ${pct}%`;
      } else {
        btnVolume.innerHTML = ICONS.volumeHigh;
        btnVolume.title = `Lautstärke: ${pct}%${currentVolume > 1 ? ' (Boost)' : ''}`;
      }

      if (gainNode) {
        gainNode.gain.value = currentVolume;
        video.volume = 1;
        if (audioCtx && audioCtx.state === 'suspended') {
          audioCtx.resume().catch(() => {});
        }
      } else if (currentVolume <= 1 && !isDynamicAudioEnabled) {
        video.volume = currentVolume;
      } else {
        video.volume = 1;
        const gn = initAudioGraph();
        if (gn) {
          if (audioCtx && audioCtx.state === 'suspended') {
            audioCtx.resume().catch(() => {});
          }
          gn.gain.value = currentVolume;
        }
      }

      try {
        localStorage.setItem(K_VOL, currentVolume);
        localStorage.setItem(K_MUTED, video.muted ? '1' : '0');
      } catch (_) {}

      // Sync with NINA audio-patch if present
      try {
        window.postMessage({ __uscVolume: currentVolume }, '*');
      } catch (_) {}
    }

    // Apply volume immediately on startup so no silence occurs
    syncVolume();

    btnVolume.addEventListener('click', (e) => {
      e.stopPropagation();
      // finger: no hover, so the button opens / closes the slider (0 % = stumm)
      if (lastPointerType === 'touch') {
        volumeWrapper.classList.toggle('open');
        resetIdleTimer();
        return;
      }
      video.muted = !video.muted;
      try {
        localStorage.setItem(K_MUTED, video.muted ? '1' : '0');
      } catch (_) {}
      if (video.muted) {
        btnVolume.innerHTML = ICONS.volumeMute;
        if (volumeText) volumeText.textContent = '0%';
        volumeSlider.style.setProperty('--v', '0%');
      } else {
        applyVolume(currentVolume || 1);
      }
    });

    volumeSlider.addEventListener('input', (e) => {
      e.stopPropagation();
      video.muted = false;
      try {
        localStorage.setItem(K_MUTED, '0');
      } catch (_) {}
      applyVolume(e.target.value);
    });

    // Mouse wheel volume scrolling when hovering over volume control (Button or Slider)
    function handleVolumeWheel(e) {
      e.preventDefault();
      e.stopPropagation();
      video.muted = false;
      const rawDelta = Math.abs(e.deltaY) >= Math.abs(e.deltaX) ? e.deltaY : -e.deltaX;
      const isUp = rawDelta < 0;
      const step = currentVolume >= 1 ? 0.15 : 0.05;
      const nextVol = Math.max(0, Math.min(6, Math.round((currentVolume + (isUp ? step : -step)) * 100) / 100));
      applyVolume(nextVol);
      resetIdleTimer();
    }

    if (volumeWrapper) {
      volumeWrapper.addEventListener('wheel', handleVolumeWheel, { passive: false });
    }
    btnVolume.addEventListener('wheel', handleVolumeWheel, { passive: false });
    volumeSlider.addEventListener('wheel', handleVolumeWheel, { passive: false });

    // Fullscreen Handling
    function isCurrentFullscreen() {
      return !!(document.fullscreenElement || document.webkitFullscreenElement);
    }

    function toggleFullscreen() {
      if (!isCurrentFullscreen()) {
        const target = wrapper || video;
        if (target.requestFullscreen) target.requestFullscreen({ navigationUI: 'hide' }).catch(() => {});
        else if (target.webkitRequestFullscreen) target.webkitRequestFullscreen();
        else if (video.webkitEnterFullscreen) video.webkitEnterFullscreen();   // iPhone: only the video itself
      } else {
        if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
        else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
      }
    }

    btnFullscreen.addEventListener('click', (e) => { e.stopPropagation(); toggleFullscreen(); });
    const btnCast = ui.querySelector('#btnCast');
    if (btnCast) {
      if (typeof window.sinaCast !== 'function') btnCast.hidden = true;
      btnCast.addEventListener('click', (e) => { e.stopPropagation(); if (typeof window.sinaCast === 'function') window.sinaCast(); });
    }
    if (btnClose) {
      btnClose.addEventListener('click', (e) => {
        e.stopPropagation();
        if (isCurrentFullscreen()) toggleFullscreen();
      });
    }

    // phones: fullscreen turns the picture sideways (landscape), leaving it lets the phone decide again
    function lockLandscape(on) {
      if (!TOUCH) return;
      try {
        const o = screen.orientation;
        if (!o) return;
        if (on) { if (o.lock) o.lock('landscape').catch(() => {}); }
        else if (o.unlock) o.unlock();
      } catch (_) {}
    }

    function handleFsChange() {
      const fs = isCurrentFullscreen();
      btnFullscreen.innerHTML = fs ? ICONS.fullscreenExit : ICONS.fullscreen;
      wrapper.classList.toggle('a16-is-fullscreen', fs);
      const fsEl = document.fullscreenElement || document.webkitFullscreenElement;
      lockLandscape(!!fsEl && (fsEl.contains(wrapper) || wrapper.contains(fsEl)));
      try {
        if (fs) {
          sessionStorage.setItem('nina_fullscreen_next', '1');
        }
      } catch (_) {}
    }

    document.addEventListener('fullscreenchange', handleFsChange);
    document.addEventListener('webkitfullscreenchange', handleFsChange);

    function triggerAutoFullscreen() {
      try {
        if (sessionStorage.getItem('nina_fullscreen_next') !== '1') return;
      } catch (_) { return; }

      const enterFs = () => {
        if (isCurrentFullscreen()) {
          try { sessionStorage.removeItem('nina_fullscreen_next'); } catch (_) {}
          return true;
        }
        const target = wrapper || video;
        if (!target) return false;

        const req = target.requestFullscreen ? target.requestFullscreen() : (target.webkitRequestFullscreen ? target.webkitRequestFullscreen() : null);
        if (req && typeof req.then === 'function') {
          req.then(() => {
            try { sessionStorage.removeItem('nina_fullscreen_next'); } catch (_) {}
          }).catch(() => {});
        } else {
          try { sessionStorage.removeItem('nina_fullscreen_next'); } catch (_) {}
        }
        return true;
      };

      // 1. Try immediately
      enterFs();

      // 2. Try on media playback events
      video.addEventListener('canplay', enterFs, { once: true });
      video.addEventListener('playing', enterFs, { once: true });

      // 3. Fallback on user gesture if browser blocked unprompted request
      const onUserGesture = () => {
        enterFs();
        window.removeEventListener('click', onUserGesture, true);
        window.removeEventListener('keydown', onUserGesture, true);
        window.removeEventListener('pointerdown', onUserGesture, true);
      };
      window.addEventListener('click', onUserGesture, true);
      window.addEventListener('keydown', onUserGesture, true);
      window.addEventListener('pointerdown', onUserGesture, true);
    }

    wrapper.__triggerAutoFullscreen = triggerAutoFullscreen;
    video.__triggerAutoFullscreen = triggerAutoFullscreen;
    triggerAutoFullscreen();

    // ========================================================
    // SPEED CONTROL (NINA Sync, 0.25x - 4.0x + Custom Input)
    // ========================================================
    // Keep the user's choice across a new stream/page. Some browsers reset
    // playbackRate to 1 when metadata is loaded.
    const SPEED_PREF_KEY = 'nina_playback_speed';
    let currentSpeed = 1.0;
    let hasSpeedPreference = false;

    function readSavedSpeed() {
      try {
        const carried = parseFloat(sessionStorage.getItem('nina_speed_next'));
        if (Number.isFinite(carried) && carried > 0) return carried;
        const nina = parseFloat(localStorage.getItem(SPEED_PREF_KEY));
        if (Number.isFinite(nina) && nina > 0) return nina;
        const legacy = parseFloat(localStorage.getItem('joyn_speed_value'));
        return Number.isFinite(legacy) && legacy > 0 ? legacy : null;
      } catch (_) { return null; }
    }

    function setSpeed(spd) {
      const s = Math.max(0.1, Math.min(16, parseFloat(spd) || 1));
      const rounded = Math.round(s * 100) / 100;
      currentSpeed = rounded;
      hasSpeedPreference = true;
      video.playbackRate = rounded;
      speedChip.innerHTML = `${ICONS.speed} ${rounded}x`;
      speedPopover.querySelectorAll('.a16-menu-item').forEach(i => {
        i.classList.toggle('active', Math.abs(parseFloat(i.dataset.speed) - rounded) < 0.01);
      });
      try {
        localStorage.setItem('joyn_speed_value', rounded);
        localStorage.setItem(SPEED_PREF_KEY, rounded);
        sessionStorage.setItem('nina_speed_next', rounded);
        window.postMessage({ __joynSpeed: rounded }, '*');
        // NINA keeps a speed per tab and applies it to every new stream: tell it this is now the real one,
        // otherwise the old value (e.g. 1.75) comes back with the next episode
        window.postMessage({ __ninaSpeedFromPlayer: rounded }, '*');
      } catch (_) {}
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    }

    // If something else changed the speed (NINA's tab speed, the page) the chip shows what is really running
    function syncSpeedFromVideo() {
      const real = Math.round((video.playbackRate || 1) * 100) / 100;
      if (Math.abs(real - currentSpeed) < 0.01) return;
      if (hasSpeedPreference) {
        video.playbackRate = currentSpeed;
        return;
      }
      currentSpeed = real;
      speedChip.innerHTML = `${ICONS.speed} ${real}x`;
      speedPopover.querySelectorAll('.a16-menu-item').forEach((i) => {
        i.classList.toggle('active', Math.abs(parseFloat(i.dataset.speed) - real) < 0.01);
      });
    }
    video.addEventListener('ratechange', syncSpeedFromVideo);
    video.addEventListener('loadedmetadata', syncSpeedFromVideo);
    video.addEventListener('playing', syncSpeedFromVideo);

    // Keep speed locked across stream changes / play events
    video.addEventListener('loadedmetadata', () => {
      if (currentSpeed !== 1.0) video.playbackRate = currentSpeed;
    });
    video.addEventListener('play', () => {
      if (currentSpeed !== 1.0) video.playbackRate = currentSpeed;
    });
    video.addEventListener('playing', () => {
      if (currentSpeed !== 1.0) video.playbackRate = currentSpeed;
    });

    speedChip.addEventListener('click', (e) => {
      e.stopPropagation();
      const isOpening = speedPopover.classList.contains('hidden');
      speedPopover.classList.toggle('hidden');
      if (isOpening) {
        stopIdleTimer();
        showControls();
      }
    });

    speedPopover.addEventListener('pointerenter', (e) => {
      if (e.pointerType !== 'mouse') return;
      isHoveringDock = true;
      stopIdleTimer();
      showControls();
    });
    speedPopover.addEventListener('pointerleave', (e) => {
      if (e.pointerType !== 'mouse') return;
      isHoveringDock = false;
      resetIdleTimer();
    });

    speedPopover.addEventListener('pointerdown', (e) => {
      e.stopPropagation();
    });
    speedPopover.addEventListener('mousedown', (e) => {
      e.stopPropagation();
    });

    speedPopover.addEventListener('click', (e) => {
      e.stopPropagation();
      const item = e.target.closest('.a16-menu-item');
      if (item && item.dataset.speed) {
        setSpeed(item.dataset.speed);
        speedPopover.classList.add('hidden');
      }
    });

    document.addEventListener('click', (e) => {
      if (!speedPopover.contains(e.target) && e.target !== speedChip && !speedChip.contains(e.target)) {
        speedPopover.classList.add('hidden');
      }
      if (!qualityPopover.contains(e.target) && e.target !== qualityChip && !qualityChip.contains(e.target)) {
        qualityPopover.classList.add('hidden');
      }
    });

    qualityPopover.addEventListener('pointerenter', (e) => {
      if (e.pointerType !== 'mouse') return;
      isHoveringDock = true;
      stopIdleTimer();
      showControls();
    });
    qualityPopover.addEventListener('pointerleave', (e) => {
      if (e.pointerType !== 'mouse') return;
      isHoveringDock = false;
      resetIdleTimer();
    });
    qualityPopover.addEventListener('pointerdown', (e) => {
      e.stopPropagation();
    });
    qualityPopover.addEventListener('mousedown', (e) => {
      e.stopPropagation();
    });

    // Sync initial speed from the carried next-episode value / localStorage.
    try {
      const savedSpeed = readSavedSpeed();
      if (savedSpeed) setSpeed(savedSpeed);
      sessionStorage.removeItem('nina_speed_next');
    } catch (_) {}

    // Idle Auto-Hide
    let idleTimer = null;
    let isHoveringDock = false;

    function showControls() { ui.classList.remove('idle'); }
    function hideControls() {
      if (!video.paused && !isHoveringDock && speedPopover.classList.contains('hidden') && qualityPopover.classList.contains('hidden')) {
        ui.classList.add('idle');
      }
    }
    function resetIdleTimer() {
      showControls();
      clearTimeout(idleTimer);
      if (!video.paused) {
        idleTimer = setTimeout(hideControls, lastPointerType === 'touch' ? 3500 : 2200);
      }
    }
    function startIdleTimer() { resetIdleTimer(); }
    function stopIdleTimer() { clearTimeout(idleTimer); }

    // Mouse only: phones fire emulated mouseenter / mousemove after every tap, which kept the
    // controls up for good (the "hovered" dock never got a mouseleave).
    const isMouse = (e) => e.pointerType === 'mouse' || e.pointerType === 'pen';
    let lastMove = 0;
    wrapper.addEventListener('pointermove', (e) => {
      if (!isMouse(e)) return;
      const now = Date.now();
      if (now - lastMove < 150 && !ui.classList.contains('idle')) return;   // mousemove fires ~100×/s
      lastMove = now;
      resetIdleTimer();
    }, { passive: true });
    wrapper.addEventListener('pointerenter', (e) => { if (isMouse(e)) resetIdleTimer(); });
    wrapper.addEventListener('pointerleave', (e) => {
      if (!isMouse(e)) return;
      if (!video.paused) {
        clearTimeout(idleTimer);
        idleTimer = setTimeout(hideControls, 600);
      }
    });

    const dock = ui.querySelector('.a16-bottom-dock');
    dock.addEventListener('pointerenter', (e) => { if (!isMouse(e)) return; isHoveringDock = true; stopIdleTimer(); showControls(); });
    dock.addEventListener('pointerleave', (e) => { if (!isMouse(e)) return; isHoveringDock = false; resetIdleTimer(); });
    // a finger on the controls keeps them up a bit longer
    ui.addEventListener('pointerdown', (e) => { if (!isMouse(e) && e.target.closest(UI_SEL)) resetIdleTimer(); }, { passive: true });

    // Keyboard Shortcuts
    window.addEventListener('keydown', (e) => {
      if (!video.isConnected) return;
      if (['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName)) return;
      switch (e.code) {
        case 'Space':
        case 'KeyK':
          e.preventDefault();
          togglePlay();
          break;
        case 'ArrowLeft':
        case 'KeyJ':
          e.preventDefault();
          skip(-10, true);
          break;
        case 'ArrowRight':
        case 'KeyL':
          e.preventDefault();
          skip(10, false);
          break;
        case 'ArrowUp':
          e.preventDefault();
          video.muted = false;
          applyVolume(currentVolume + (currentVolume >= 1 ? 0.25 : 0.1));
          resetIdleTimer();
          break;
        case 'ArrowDown':
          e.preventDefault();
          applyVolume(currentVolume - (currentVolume > 1 ? 0.25 : 0.1));
          resetIdleTimer();
          break;
        case 'KeyM':
          e.preventDefault();
          btnVolume.click();
          resetIdleTimer();
          break;
        case 'KeyF':
          e.preventDefault();
          toggleFullscreen();
          break;
        case 'KeyI':
          if (typeof skipIntroAction === 'function') {
            e.preventDefault();
            skipIntroAction();
          }
          break;
        case 'KeyS':
        case 'KeyN':
          if (typeof skipOutroAction === 'function') {
            e.preventDefault();
            skipOutroAction();
          }
          break;
      }
    });

    // ========================================================
    // NINA INTEGRATION (Progress Tracking & Auto-Next)
    // ========================================================
    let played = 0;
    let lastT = video.currentTime;
    let lastSent = 0;

    function reportProgress(force) {
      if (!video.duration || video.duration < 10) return;
      const now = Date.now();
      if (!force && now - lastSent < 10000) return;
      lastSent = now;
      try {
        chrome.runtime.sendMessage({
          type: 'AW',
          op: 'progress',
          data: {
            position: video.currentTime,
            duration: video.duration,
            played: Math.round(played)
          }
        }, () => void chrome.runtime.lastError);
      } catch (_) {}
    }

    // ========================================================
    // EPISODE PLAYBACK RESUME (Merkt sich Position beim Neuladen)
    // ========================================================
    function getEpisodeStorageKey() {
      try {
        const path = ((window.__sinaPath ? window.__sinaPath() : window.location.pathname) || '').replace(/\/+$/, '');
        return 'nina_ep_pos_' + path;
      } catch (_) {
        return 'nina_ep_pos_default';
      }
    }

    let lastSavedPlaybackTime = 0;
    function saveEpisodePosition() {
      if (!video || !video.duration || !isFinite(video.duration)) return;
      const cur = video.currentTime;
      const key = getEpisodeStorageKey();

      // If finished or in the last 20 seconds, clear saved position so rewatch starts at beginning
      if (cur >= video.duration - 20) {
        try { localStorage.removeItem(key); } catch (_) {}
        return;
      }

      // Do not save in first 3 seconds
      if (cur < 3) return;

      // Throttle localStorage writes (max once every 1.5 seconds)
      if (Math.abs(cur - lastSavedPlaybackTime) < 1.5) return;
      lastSavedPlaybackTime = cur;

      try {
        localStorage.setItem(key, JSON.stringify({
          pos: Math.round(cur * 10) / 10,
          dur: Math.round(video.duration),
          at: Date.now()
        }));
      } catch (_) {}
    }

    let hasRestoredPosition = false;
    function restoreEpisodePosition() {
      if (hasRestoredPosition) return;
      try {
        const raw = localStorage.getItem(getEpisodeStorageKey());
        if (!raw) return;
        const d = JSON.parse(raw);
        if (d && typeof d.pos === 'number' && d.pos > 3) {
          if (video.duration && isFinite(video.duration) && d.pos >= video.duration - 15) return;
          hasRestoredPosition = true;
          video.currentTime = d.pos;
          console.log('[NINA Player] Stelle Position wieder her:', formatTime(d.pos));
          showResumeIndicator(d.pos);
        }
      } catch (_) {}
    }

    function showResumeIndicator(pos, customText) {
      let b = wrapper.querySelector('.a16-resume-badge');
      if (!b) {
        b = document.createElement('div');
        b.className = 'a16-resume-badge';
        wrapper.appendChild(b);
      }
      b.innerHTML = `<svg viewBox="0 0 24 24" style="width:16px;height:16px;fill:currentColor;"><path d="M12 5V1l5 5-5 5V7c-3.31 0-6 2.69-6 6s2.69 6 6 6 6-2.69 6-6h2c0 4.42-3.58 8-8 8s-8-3.58-8-8 3.58-8 8-8z"/></svg> ${customText || ('Fortgesetzt bei ' + formatTime(pos))}`;
      b.classList.remove('fade-out');
      b.classList.add('visible');
      setTimeout(() => {
        b.classList.add('fade-out');
        setTimeout(() => b.classList.remove('visible', 'fade-out'), 500);
      }, 3000);
    }

    // ========================================================
    // ANISKIP & HEURISTIC INTRO / OUTRO DETECTION
    // ========================================================
    let aniSkipTimes = null;
    let aniSkipQueried = false;
    let currentSkipMode = null; // 'aniskip_op' | 'aniskip_ed' | 'intro' | 'outro' | null

    function fetchAniSkipTimes() {
      if (!video.duration || !isFinite(video.duration) || video.duration < 100) return;

      let series = '';
      let season = 1;
      let episode = 1;

      try {
        chrome.runtime.sendMessage({ type: 'AW', op: 'ctx' }, (res) => {
          if (res && res.ok && res.result) {
            const ctx = res.result;
            if (ctx.series) series = ctx.series;
            if (ctx.season) season = ctx.season;
            if (ctx.episode) episode = ctx.episode;
          }
          queryAniSkip();
        });
      } catch (_) {
        queryAniSkip();
      }

      function queryAniSkip() {
        if (!series || !episode) {
          try {
            const parts = (window.__sinaPath ? window.__sinaPath() : window.location.pathname).split('/').filter(Boolean);
            const slugIdx = parts.indexOf('stream');
            if (slugIdx >= 0 && parts[slugIdx + 1]) {
              series = series || parts[slugIdx + 1].replace(/-/g, ' ');
            }
            const sm = (window.__sinaPath ? window.__sinaPath() : window.location.pathname).match(/staffel-(\d+)/i);
            if (sm) season = parseInt(sm[1], 10);
            const em = (window.__sinaPath ? window.__sinaPath() : window.location.pathname).match(/episode-(\d+)/i);
            if (em) episode = parseInt(em[1], 10);

            // ani.pm fallback
            if (!series) {
              const h1 = document.querySelector('.ytw-meta h1, .watch-under h1, h1')?.textContent?.trim();
              if (h1) series = h1;
              else if (document.title) {
                series = document.title.replace(/\s*[—·-]\s*Episode\s*\d+.*$/i, '').replace(/\s*[—·-]\s*ani\.pm.*$/i, '').trim();
              }
            }
            if (!episode) {
              const qp = new URLSearchParams(window.location.search).get('ep');
              const hm = (qp && /^\d+$/.test(qp) ? [null, qp] : null) || window.location.hash.match(/#ep(\d+)/i) || document.title.match(/Episode\s*(\d+)/i);
              if (hm) episode = parseInt(hm[1], 10);
            }
            if (!season) {
              const smTitle = (series || '').match(/(?:Season|Staffel)\s*(\d+)|(\d+)(?:nd|rd|th|st)\s*Season/i);
              if (smTitle) season = parseInt(smTitle[1] || smTitle[2], 10);
            }
          } catch (_) {}
        }

        if (!series || !episode) {
          aniSkipQueried = true;
          updateSkipButton();
          return;
        }

        chrome.runtime.sendMessage({
          type: 'AW',
          op: 'getAniSkipTimes',
          series,
          season,
          episode,
          duration: video.duration
        }, (res) => {
          aniSkipQueried = true;
          if (!chrome.runtime.lastError && res && res.ok && res.result && (res.result.op || res.result.ed)) {
            aniSkipTimes = res.result;
            console.log('[NINA Player] Exakte AniSkip Zeiten aus Datenbank aktiv:', aniSkipTimes);
          } else {
            aniSkipTimes = null;
            console.log('[NINA Player] Keine Datenbank-Skips für diese Folge gefunden. Fallback auf Algorithmus aktiv.');
          }
          updateSkipButton();
        });
      }
    }

    video.addEventListener('loadedmetadata', fetchAniSkipTimes);
    video.addEventListener('canplay', fetchAniSkipTimes);

    const AUTO_NEXT_KEY = 'nina_auto_next';
    function getAutoNext() {
      try { return localStorage.getItem(AUTO_NEXT_KEY) !== '0'; } catch (_) { return true; }
    }
    function setAutoNext(on) {
      try { localStorage.setItem(AUTO_NEXT_KEY, on ? '1' : '0'); } catch (_) {}
      if (!on) closeNextOverlay();
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    }

    function updateSkipButton() {
      const prevMode = currentSkipMode;
      const done = () => {
        if (currentSkipMode !== prevMode && typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
      };
      if (!btnSkipIntro || !txtSkipBtn || !video.duration || !isFinite(video.duration)) {
        if (btnSkipIntro) btnSkipIntro.classList.add('hidden');
        currentSkipMode = null;
        return done();
      }

      const cur = video.currentTime;
      const dur = video.duration;
      const hasDbSkips = !!(aniSkipTimes && (aniSkipTimes.op || aniSkipTimes.ed));

      // 1. Wenn Datenbank-Skips für diese Folge existieren: IMMER NUR DIE AUS DER DATENBANK ANZEIGEN!
      if (hasDbSkips) {
        // Exact AniSkip Database Opening
        if (aniSkipTimes.op && cur >= aniSkipTimes.op.startTime && cur < aniSkipTimes.op.endTime) {
          currentSkipMode = 'aniskip_op';
          txtSkipBtn.innerHTML = `Intro überspringen <span class="a16-skip-badge a16-skip-badge-db" title="Quelle: AniSkip Online-Datenbank (exakt)">Datenbank</span> (${formatTime(aniSkipTimes.op.startTime)} - ${formatTime(aniSkipTimes.op.endTime)}) [I]`;
          btnSkipIntro.classList.remove('hidden');
          return done();
        }

        // Exact AniSkip Database Ending
        if (aniSkipTimes.ed && cur >= aniSkipTimes.ed.startTime && cur < aniSkipTimes.ed.endTime) {
          currentSkipMode = 'aniskip_ed';
          txtSkipBtn.innerHTML = `Outro überspringen <span class="a16-skip-badge a16-skip-badge-db" title="Quelle: AniSkip Online-Datenbank (exakt)">Datenbank</span> (Nächste Folge) [S]`;
          btnSkipIntro.classList.remove('hidden');
          return done();
        }

        // In einer Folge mit Datenbank-Skips KEINE Algorithmus-Skips anzeigen!
        currentSkipMode = null;
        btnSkipIntro.classList.add('hidden');
        return done();
      }

      // 2. Nur wenn es in der Datenbank KEINE Skips für diese Folge gibt (Query beendet ohne Treffer):
      // Fallback: Algorithmus-Skips
      if (aniSkipQueried || !aniSkipTimes) {
        // Fallback Heuristic Opening (zwischen 10s und 145s)
        if (dur > 120 && cur >= 10 && cur <= 145) {
          currentSkipMode = 'intro';
          txtSkipBtn.innerHTML = `Intro überspringen <span class="a16-skip-badge a16-skip-badge-algo" title="Quelle: Intelligenter Erkennungs-Algorithmus (Fallback)">Algorithmus</span> (+85s) [I]`;
          btnSkipIntro.classList.remove('hidden');
          return done();
        }

        // Fallback Heuristic Ending (letzte 115s bis 8s vor Ende)
        if (dur > 180 && cur >= dur - 115 && cur < dur - 8) {
          currentSkipMode = 'outro';
          txtSkipBtn.innerHTML = `Outro überspringen <span class="a16-skip-badge a16-skip-badge-algo" title="Quelle: Intelligenter Erkennungs-Algorithmus (Fallback)">Algorithmus</span> (Nächste Folge) [S]`;
          btnSkipIntro.classList.remove('hidden');
          return done();
        }
      }

      currentSkipMode = null;
      btnSkipIntro.classList.add('hidden');
      done();
    }

    function skipIntroAction() {
      if (!video || !video.duration) return;
      if (aniSkipTimes && aniSkipTimes.op && video.currentTime < aniSkipTimes.op.endTime) {
        const target = Math.min(video.duration - 5, aniSkipTimes.op.endTime + 0.5);
        video.currentTime = target;
        showResumeIndicator(target, 'Intro übersprungen · [Datenbank]');
      } else {
        const target = Math.min(video.duration - 10, video.currentTime + 85);
        video.currentTime = target;
        showResumeIndicator(target, 'Intro übersprungen · (+85s)');
      }
      if (btnSkipIntro) btnSkipIntro.classList.add('hidden');
      currentSkipMode = null;
      resetIdleTimer();
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    }

    function skipOutroAction() {
      if (btnSkipIntro) btnSkipIntro.classList.add('hidden');
      const isDb = currentSkipMode === 'aniskip_ed';
      currentSkipMode = null;
      const nextUrl = getNextEpisodeUrl();
      if (nextUrl) {
        showResumeIndicator(video.currentTime, isDb ? 'Outro übersprungen · [Datenbank]' : 'Outro übersprungen');
        goToNextEpisode(nextUrl);
      } else if (aniSkipTimes && aniSkipTimes.ed && video.duration && video.currentTime < aniSkipTimes.ed.endTime) {
        const target = Math.min(video.duration - 1, aniSkipTimes.ed.endTime + 0.5);
        video.currentTime = target;
        showResumeIndicator(target, 'Outro übersprungen · [Datenbank]');
      } else if (video.duration && isFinite(video.duration)) {
        video.currentTime = Math.max(video.currentTime, video.duration - 5);
      }
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    }

    if (btnSkipIntro) {
      btnSkipIntro.addEventListener('click', (e) => {
        e.stopPropagation();
        if (currentSkipMode === 'aniskip_op' || currentSkipMode === 'intro') {
          skipIntroAction();
        } else if (currentSkipMode === 'aniskip_ed' || currentSkipMode === 'outro') {
          skipOutroAction();
        }
      });
    }

    video.addEventListener('loadedmetadata', restoreEpisodePosition);
    video.addEventListener('canplay', restoreEpisodePosition);
    video.addEventListener('seeking', saveEpisodePosition);
    video.addEventListener('seeked', saveEpisodePosition);
    window.addEventListener('beforeunload', saveEpisodePosition);
    window.addEventListener('pagehide', saveEpisodePosition);

    video.addEventListener('timeupdate', () => {
      const dt = video.currentTime - lastT;
      if (dt > 0 && dt < 3) played += dt;
      lastT = video.currentTime;
      reportProgress(false);
      saveEpisodePosition();
      updateSkipButton();
    });

    video.addEventListener('pause', () => {
      reportProgress(true);
      saveEpisodePosition();
    });

    // ========================================================
    // AUTO-NEXT & EPISODE NAVIGATION (Fix: 'Jetzt' Button)
    // ========================================================
    let nextShown = false;
    let nextTimer = null;
    window.addEventListener('ninaA16EpisodeSwitched', () => {
      nextShown = false;
      nextTimer = null;
    });

    function closeNextOverlay() {
      clearInterval(nextTimer);
      if (nextCard) nextCard.classList.add('hidden');
    }

    function getNextEpisodeUrl() {
      if (window.__sinaCtx) {
        const n = window.__sinaCtx.next;
        return n && n.href ? new URL(n.href, window.location.href).href : null;
      }
      // After an in-place switch the surrounding AniWorld/Skin DOM can still
      // describe the previous episode. The URL is the authoritative source.
      try {
        const m = (window.__sinaPath ? window.__sinaPath() : window.location.pathname).match(/(.*\/staffel-\d+\/episode-)(\d+)(.*)/i);
        if (m) {
          const curNum = parseInt(m[2], 10);
          if (!isNaN(curNum)) return new URL(`${m[1]}${curNum + 1}${m[3] || ''}`, window.location.href).href;
        }
      } catch (_) {}

      // 1. Direct link from NINA Skin action bar
      const skinBtn = document.querySelector('.ns-ep-acts a.ns-btn[href*="/episode-"], .ns-ep-acts a[href*="/episode-"]');
      if (skinBtn && skinBtn.getAttribute('href')) {
        return new URL(skinBtn.getAttribute('href'), window.location.href).href;
      }

      // 2. Direct link from NINA Skin episode list sidebar (next item after current)
      const curRow = document.querySelector('.ns-side .ns-ep.cur');
      if (curRow) {
        const nextRow = curRow.nextElementSibling;
        const nextA = nextRow ? (nextRow.tagName === 'A' ? nextRow : nextRow.querySelector('a[href]')) : null;
        if (nextA && nextA.getAttribute('href')) {
          return new URL(nextA.getAttribute('href'), window.location.href).href;
        }
      }

      // 3. AniWorld native hoster next buttons / lists
      const nativeCur = document.querySelector('#stream ul li.active, table.seasonEpisodesList tr.active');
      if (nativeCur) {
        const nextLi = nativeCur.nextElementSibling;
        const nextA = nextLi ? nextLi.querySelector('a[href*="/episode-"]') : null;
        if (nextA && nextA.getAttribute('href')) {
          return new URL(nextA.getAttribute('href'), window.location.href).href;
        }
      }
      const nativeNextA = document.querySelector('a.nextEpisode, .nav-next a, [data-action="next-episode"]');
      if (nativeNextA && nativeNextA.getAttribute('href')) {
        return new URL(nativeNextA.getAttribute('href'), window.location.href).href;
      }

      // 4. URL calculation: /anime/stream/<slug>/staffel-<S>/episode-<E>
      try {
        const m = (window.__sinaPath ? window.__sinaPath() : window.location.pathname).match(/(.*\/staffel-\d+\/episode-)(\d+)(.*)/i);
        if (m) {
          const prefix = m[1];
          const curNum = parseInt(m[2], 10);
          const suffix = m[3] || '';
          if (!isNaN(curNum)) {
            return new URL(`${prefix}${curNum + 1}${suffix}`, window.location.href).href;
          }
        }
      } catch (_) {}

      return null;
    }

    function goToNextEpisode(preResolvedUrl) {
      closeNextOverlay();
      const nextUrl = preResolvedUrl || getNextEpisodeUrl();

      try {
        const speed = Math.round((video.playbackRate || 1) * 100) / 100;
        sessionStorage.setItem('nina_speed_next', String(speed));
        localStorage.setItem('nina_playback_speed', String(speed));
        sessionStorage.setItem('nina_autoplay_next', '1');
        sessionStorage.setItem('nina_fullscreen_next', '1');
        chrome.runtime.sendMessage({ type: 'AW', op: 'setAutoplay' }, () => void chrome.runtime.lastError);
      } catch (_) {}

      // Keep the existing video element when the next stream was preloaded.
      if (nextUrl && typeof window.__switchToPreloadedEpisode === 'function') {
        try {
          if (window.__switchToPreloadedEpisode(nextUrl)) return;
        } catch (_) {}
      }

      // The background navigation command is only needed for the fallback.
      try { chrome.runtime.sendMessage({ type: 'AW', op: 'next' }, () => void chrome.runtime.lastError); } catch (_) {}

      if (nextUrl) {
        console.log('[NINA Player] Starte nächste Folge:', nextUrl);
        try {
          if (window.top && window.top !== window) {
            window.top.location.href = nextUrl;
            return;
          }
        } catch (_) {}
        window.location.href = nextUrl;
      } else {
        // Asynchronous fallback: ask background script for ctx.next
        try {
          chrome.runtime.sendMessage({ type: 'AW', op: 'ctx' }, (res) => {
            if (res && res.ok && res.result && res.result.next && res.result.next.href) {
              const u = new URL(res.result.next.href, window.location.href).href;
              try {
                if (window.top && window.top !== window) { window.top.location.href = u; return; }
              } catch (_) {}
              window.location.href = u;
            }
          });
        } catch (_) {}
      }
    }

    async function onEpisodeEnded() {
      if (nextShown) return;
      nextShown = true;
      reportProgress(true);

      const nextUrl = getNextEpisodeUrl();
      if (!getAutoNext() || !nextUrl) return;

      // Retrieve title label if possible
      let label = 'Nächste Folge';
      try {
        chrome.runtime.sendMessage({ type: 'AW', op: 'ctx' }, (res) => {
          if (res && res.ok && res.result && res.result.next && res.result.next.label) {
            label = res.result.next.label;
            if (nextTitle) nextTitle.textContent = label;
          }
        });
      } catch (_) {}

      if (nextTitle) nextTitle.textContent = label;
      if (nextCard) nextCard.classList.remove('hidden');

      let remaining = 7;
      setText(nextSec, String(remaining));

      btnCancelNext.onclick = (e) => {
        e.stopPropagation();
        closeNextOverlay();
      };

      btnConfirmNext.onclick = (e) => {
        e.stopPropagation();
        goToNextEpisode(nextUrl);
      };

      clearInterval(nextTimer);
      nextTimer = setInterval(() => {
        remaining--;
        setText(nextSec, String(remaining));
        if (remaining <= 0) {
          clearInterval(nextTimer);
          goToNextEpisode(nextUrl);
        }
      }, 1000);
    }

    video.addEventListener('ended', onEpisodeEnded);
    video.addEventListener('timeupdate', () => {
      if (video.duration && video.duration > 60 && video.duration - video.currentTime < 0.8 && !video.seeking) {
        onEpisodeEnded();
      }
      if (video.duration && video.duration - video.currentTime <= 60 && !video.ended && typeof window.__preloadNextEpisode === 'function') {
        const next = getNextEpisodeUrl();
        if (next) window.__preloadNextEpisode(next);
      }
    });

    // Fernbedienung vom Handy / anderen Geräten
    const notifyRemote = () => {
      if (typeof window.__sinaRemoteChanged === 'function') window.__sinaRemoteChanged();
    };
    ['play', 'pause', 'seeked', 'volumechange', 'loadedmetadata', 'durationchange'].forEach((ev) => {
      video.addEventListener(ev, notifyRemote);
    });
    document.addEventListener('fullscreenchange', notifyRemote);
    document.addEventListener('webkitfullscreenchange', notifyRemote);

    window.__sinaRemote = {
      getState() {
        return {
          paused: !!video.paused,
          pos: isFinite(video.currentTime) ? video.currentTime : 0,
          dur: isFinite(video.duration) ? video.duration : 0,
          speed: currentSpeed,
          volume: currentVolume,
          muted: !!video.muted,
          dynAudio: !!isDynamicAudioEnabled,
          autoNext: getAutoNext(),
          skipMode: currentSkipMode,
          fullscreen: isCurrentFullscreen(),
        };
      },
      exec(action, value) {
        switch (action) {
          case 'toggle': togglePlay(); break;
          case 'play': video.play().catch(() => {}); break;
          case 'pause': video.pause(); break;
          case 'seek':
            if (video.duration && isFinite(video.duration)) {
              const t = Math.max(0, Math.min(video.duration, Number(value) || 0));
              video.currentTime = t;
              showResumeIndicator(t, 'Gesprungen zu ' + formatTime(t));
            }
            break;
          case 'seekBy': {
            const d = Number(value) || 0;
            skip(d, d < 0);
            break;
          }
          case 'speed':
            setSpeed(value);
            showResumeIndicator(0, 'Geschwindigkeit: ' + currentSpeed + 'x');
            break;
          case 'volume':
            video.muted = false;
            applyVolume(value);
            showResumeIndicator(0, 'Lautstärke: ' + Math.round(currentVolume * 100) + '%');
            break;
          case 'mute': {
            const want = typeof value === 'boolean' ? value : !video.muted;
            video.muted = want;
            try { localStorage.setItem(K_MUTED, want ? '1' : '0'); } catch (_) {}
            if (want) {
              btnVolume.innerHTML = ICONS.volumeMute;
              if (volumeText) volumeText.textContent = '0%';
              volumeSlider.style.setProperty('--v', '0%');
            } else {
              applyVolume(currentVolume || 1);
            }
            showResumeIndicator(0, want ? 'Ton aus' : 'Ton an');
            break;
          }
          case 'dynAudio':
            toggleDynamicAudio();
            break;
          case 'autoNext': {
            const want = typeof value === 'boolean' ? value : !getAutoNext();
            setAutoNext(want);
            showResumeIndicator(0, want ? 'Auto-Weiter: AN' : 'Auto-Weiter: AUS');
            break;
          }
          case 'skip':
            if (value === 'outro' || (value !== 'intro' && (currentSkipMode === 'aniskip_ed' || currentSkipMode === 'outro'))) {
              skipOutroAction();
            } else {
              skipIntroAction();
            }
            break;
          case 'skipIntro': skipIntroAction(); break;
          case 'skipOutro': skipOutroAction(); break;
          case 'next': goToNextEpisode(); break;
          case 'prev': {
            const p = window.__sinaCtx && window.__sinaCtx.prev;
            if (p && p.href) {
              try {
                const speed = Math.round((video.playbackRate || 1) * 100) / 100;
                sessionStorage.setItem('nina_speed_next', String(speed));
                localStorage.setItem('nina_playback_speed', String(speed));
                sessionStorage.setItem('nina_autoplay_next', '1');
                if (isCurrentFullscreen()) sessionStorage.setItem('nina_fullscreen_next', '1');
              } catch (_) {}
              location.hash = p.href;
            }
            break;
          }
          case 'fullscreen': toggleFullscreen(); break;
        }
        notifyRemote();
      },
    };
    notifyRemote();

    // Auto-play on open if triggered by auto-next
    try {
      if (sessionStorage.getItem('nina_autoplay_next') === '1') {
        sessionStorage.removeItem('nina_autoplay_next');
        video.play().catch(() => {});
      } else {
        chrome.runtime.sendMessage({ type: 'AW', op: 'autoplay', take: true }, (res) => {
          if (res && res.ok && res.result) {
            video.play().catch(() => {});
          }
        });
      }
    } catch (_) {}

    // Suppress hoster remnants (and on ani.pm the native control bar) whenever the hoster's
    // framework re-renders — an observer instead of polling every 800 ms, and only changes
    // outside our own controls count.
    const isForeign = FOREIGN && wrapper.classList.contains('a16-foreign-root');
    let sweepQueued = false;
    const sweepNow = () => {
      sweepQueued = false;
      if (!video.isConnected) return;   // may be moved between containers when the stream changes
      suppressHosterElements(wrapper);
      if (isForeign) suppressForeignControls(wrapper, video, ui);
    };
    const sweepMo = new MutationObserver((muts) => {
      if (sweepQueued) return;
      for (const m of muts) {
        if (!ui.contains(m.target)) { sweepQueued = true; requestAnimationFrame(sweepNow); return; }
      }
    });
    sweepMo.observe(wrapper, { childList: true, subtree: true });
    // slow safety net for style-only changes the observer doesn't see; nothing while the tab is hidden
    const sweepIv = setInterval(() => { if (!document.hidden) sweepNow(); }, isForeign ? 3000 : 5000);
    sweepNow();

    window.addEventListener('hashchange', () => {
      setTimeout(() => {
        updateTitle();
        fetchAniSkipTimes();
      }, 100);
    });

    window.__mountA16Player = mountAndroid16Player;
  }

  window.__mountA16Player = mountAndroid16Player;

  // Scan & Mount only on AniWorld videos
  function scanAndMount() {
    const videos = document.querySelectorAll('video');
    videos.forEach(mountAndroid16Player);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', scanAndMount);
  } else {
    scanAndMount();
  }

  const observer = new MutationObserver((mutations) => {
    for (let i = 0; i < mutations.length; i++) {
      const mut = mutations[i];
      for (let j = 0; j < mut.addedNodes.length; j++) {
        const node = mut.addedNodes[j];
        if (node.nodeType === 1) {
          if (node.tagName === 'VIDEO') {
            mountAndroid16Player(node);
          } else if (node.querySelectorAll) {
            const vids = node.querySelectorAll('video');
            vids.forEach(mountAndroid16Player);
          }
        }
      }
    }
  });

  observer.observe(document.documentElement || document.body, { childList: true, subtree: true });
})();
