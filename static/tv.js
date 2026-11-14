// Fernseher: erkennt Fernseher, steuert mit der Fernbedienung (Pfeiltasten, OK, Zurück) und nimmt vom Handy
// „weiter schauen“ entgegen. Am Handy zeigt es die Fernseher mit demselben Profil und schickt ihnen die Folge.
// Wird vor dem Hauptskript geladen; die Hilfen der App (P, route, api …) werden erst bei Gebrauch angesprochen.
(function () {
  'use strict';
  const TV_RE = /(SMART-?TV|SmartTV|Tizen|Web0S|WebOS|NetCast|HbbTV|BRAVIA|Android ?TV|AFT[A-Z]|CrKey|Chromecast|VIERA|Roku|AppleTV|PlayStation|Xbox|Opera TV|NETTV|FireTV|MiTV|Large Screen|Hisense|VIDAA|TitanOS|Foxxum|Philips)/i;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const lget = (k, d) => { try { const v = localStorage.getItem('sina:g:' + k); return v === null ? d : JSON.parse(v); } catch { return d; } };
  const lset = (k, v) => { try { localStorage.setItem('sina:g:' + k, JSON.stringify(v)); } catch {} };

  // ---------- Gerät ----------
  const mode = () => lget('deviceMode', 'auto');
  const detect = () => { const m = mode(); if (m === 'tv') return true; if (m === 'normal') return false; return TV_RE.test(navigator.userAgent) || /[?&]tv=1/.test(location.search); };
  window.SINA_TV = detect();
  document.documentElement.classList.toggle('sina-tv', window.SINA_TV);

  let deviceId = lget('deviceId', null);
  if (!deviceId) { deviceId = Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4); lset('deviceId', deviceId); }
  const defaultName = () => {
    const u = navigator.userAgent;
    const brand = /Tizen/i.test(u) ? 'Samsung TV' : /Web0S|WebOS|NetCast/i.test(u) ? 'LG TV' : /BRAVIA/i.test(u) ? 'Sony TV' : /AFT[A-Z]|FireTV/i.test(u) ? 'Fire TV' : /Android ?TV|Chromecast|CrKey/i.test(u) ? 'Android TV' : /Roku/i.test(u) ? 'Roku' : 'Fernseher';
    return brand + ' ' + deviceId.slice(0, 3);
  };
  const deviceName = () => (lget('deviceName', '') || '').trim() || defaultName();

  // ---------- Hinweis ----------
  function toast(text) {
    const root = document.getElementById('nina-skin') || document.body;
    const t = document.createElement('div');
    t.className = 'sina-toast';
    t.textContent = text;
    root.appendChild(t);
    setTimeout(() => t.remove(), 4000);
  }
  window.sinaToast = toast;

  // ---------- Fernseher: auf Befehle vom Handy warten ----------
  let pollCtl = null;
  async function pollLoop(profile, ctl) {
    let fails = 0;
    while (!ctl.signal.aborted) {
      try {
        const r = await fetch('/api/devices/poll?' + new URLSearchParams({ id: deviceId, profile, name: deviceName() }), { signal: ctl.signal });
        if (!r.ok) throw new Error('HTTP ' + r.status);
        const j = await r.json();
        fails = 0;
        for (const c of j.cmds || []) handleCmd(c);
      } catch (e) {
        if (ctl.signal.aborted) return;
        fails++;
        await sleep(Math.min(15000, 1500 * fails));
      }
    }
  }
  function handleCmd(c) {
    if (!c || c.type !== 'open' || !/^#\//.test(c.hash || '')) return;
    if (c.position > 3 && /\/(episode|film)-\d+$/.test(c.hash)) {
      // Position für den Player ablegen: er stellt sie beim Start wieder her (nina_… landet im Speicher des Profils)
      const path = decodeURIComponent(c.hash.replace(/^#/, '').split('?')[0]).replace(/\/+$/, '');
      try { localStorage.setItem('nina_ep_pos_' + path, JSON.stringify({ pos: c.position, dur: c.duration || 0, at: Date.now() })); } catch {}
    }
    toast('Vom Handy: ' + (c.title || 'weiter schauen'));
    if (location.hash === c.hash) { if (typeof route === 'function') route(); } else location.hash = c.hash;
  }

  // ---------- Handy: Fernseher mit demselben Profil finden ----------
  let tvs = [];
  async function refreshTvs() {
    if (window.SINA_TV || typeof P === 'undefined' || !P || document.hidden) return;
    try {
      const r = await fetch('/api/devices?' + new URLSearchParams({ profile: P, me: deviceId }));
      tvs = r.ok ? await r.json() : [];
    } catch { tvs = []; }
    updateCastUI();
  }
  const castSvg = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M21 3H3c-1.1 0-2 .9-2 2v3h2V5h18v14h-7v2h7c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11z"/></svg>';
  function updateCastUI() {
    const show = tvs.length > 0 && !window.SINA_TV;
    const b = document.getElementById('castBtn');
    if (b) { b.hidden = !show; b.title = tvs.length === 1 ? 'Auf ' + tvs[0].name + ' weiter schauen' : 'Auf einem Fernseher weiter schauen'; }
    const e = document.getElementById('castEp');
    if (e) e.hidden = !show;
  }
  async function castTo(tv) {
    const v = document.querySelector('video');
    const pos = v && v.currentTime > 3 ? v.currentTime : 0;
    const dur = v && isFinite(v.duration) ? v.duration : 0;
    const cmd = { type: 'open', hash: location.hash || '#/', position: pos, duration: dur, title: document.title.replace(/ – Sina$/, '') };
    try {
      const r = await fetch('/api/devices/send?' + new URLSearchParams({ to: tv.id, profile: P }), { method: 'POST', headers: { 'Content-Type': 'text/plain' }, body: JSON.stringify(cmd) });
      if (!r.ok) { const j = await r.json().catch(() => ({})); toast(j.error || 'Fernseher nicht erreichbar.'); refreshTvs(); return; }
      if (v) { try { v.pause(); } catch {} }
      toast('Läuft jetzt auf ' + tv.name);
    } catch { toast('Fernseher nicht erreichbar.'); }
  }
  function closePicker() { const m = document.querySelector('.sina-modal'); if (m) m.remove(); }
  function openPicker() {
    closePicker();
    const root = document.getElementById('nina-skin');
    const m = document.createElement('div');
    m.className = 'sina-modal';
    m.innerHTML = `<div class="sina-sheet" role="dialog"><h2>Auf welchem Fernseher?</h2><p>Diese Fernseher haben Sina offen und dasselbe Profil gewählt.</p>
      ${tvs.map((t, i) => `<button type="button" class="sina-tvbtn" data-tv="${i}">${castSvg}<span>${String(t.name).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))}</span></button>`).join('')}
      <button type="button" class="sina-tvbtn" data-close style="background:transparent">Abbrechen</button></div>`;
    root.appendChild(m);
    m.addEventListener('click', (e) => {
      const b = e.target.closest('[data-tv]');
      if (b) { closePicker(); castTo(tvs[+b.dataset.tv]); return; }
      if (e.target === m || e.target.closest('[data-close]')) closePicker();
    });
    const first = m.querySelector('[data-tv]'); if (first) first.focus();
  }
  function onCast() { if (tvs.length === 1) castTo(tvs[0]); else if (tvs.length > 1) openPicker(); else toast('Kein Fernseher gefunden.'); }
  document.addEventListener('click', (e) => { if (e.target.closest('#castBtn, #castEp')) { e.preventDefault(); onCast(); } });

  // ---------- Von der App aufgerufen ----------
  window.sinaOnProfile = function () {
    if (pollCtl) { pollCtl.abort(); pollCtl = null; }
    tvs = []; updateCastUI();
    if (typeof P === 'undefined' || !P) return;
    if (window.SINA_TV) { pollCtl = new AbortController(); pollLoop(P, pollCtl); }
    else refreshTvs();
  };
  window.sinaEpisodeRendered = function () { updateCastUI(); refreshTvs(); };
  setInterval(refreshTvs, 8000);
  document.addEventListener('visibilitychange', () => { if (!document.hidden) refreshTvs(); });

  // Einstellungen: Name und Modus dieses Geräts
  window.sinaDeviceCard = function () {
    const m = mode();
    return `<section class="sina-card">
      <h2>Dieses Gerät</h2>
      <p>Ein <b>Fernseher</b> zeigt große Kacheln, lässt sich mit der Fernbedienung (Pfeiltasten, OK, Zurück) steuern und meldet sich mit seinem Profil an. Am Handy erscheint dann „Auf Fernseher weiter schauen“.</p>
      <div class="sina-kv"><span>Erkannt als</span><span>${window.SINA_TV ? 'Fernseher' : 'Handy / PC'}</span><span>Name</span><span><input id="devName" maxlength="30" placeholder="${defaultName()}" value="${(lget('deviceName', '') || '').replace(/"/g, '&quot;')}" style="width:100%;max-width:260px"></span>
      <span>Modus</span><span><select id="devMode"><option value="auto" ${m === 'auto' ? 'selected' : ''}>Automatisch erkennen</option><option value="tv" ${m === 'tv' ? 'selected' : ''}>Immer als Fernseher</option><option value="normal" ${m === 'normal' ? 'selected' : ''}>Nie als Fernseher</option></select></span></div>
      <div class="sina-row"><button class="ns-btn" id="devSave">Speichern</button></div></section>`;
  };
  window.sinaDeviceBind = function () {
    const b = document.getElementById('devSave');
    if (!b) return;
    b.addEventListener('click', () => {
      lset('deviceName', document.getElementById('devName').value.trim());
      lset('deviceMode', document.getElementById('devMode').value);
      location.reload();
    });
  };

  // ---------- Fernbedienung ----------
  if (!window.SINA_TV) return;

  const SEL = '#nina-skin a[href], #nina-skin button, #nina-skin input, #nina-skin select, #nina-skin textarea, #nina-skin [tabindex="0"]';
  const visible = (el) => {
    if (el.hidden || el.disabled || el.closest('[hidden]')) return false;
    const r = el.getBoundingClientRect();
    return r.width > 2 && r.height > 2;
  };
  function zoneRoot() {
    const prof = document.getElementById('profiles');
    if (prof && !prof.hidden) return prof;
    return document.querySelector('.sina-modal');
  }
  function candidates() {
    const z = zoneRoot();
    return [...document.querySelectorAll(SEL)].filter((el) => {
      if (z && !z.contains(el)) return false;
      if (!z && document.getElementById('profiles') && !document.getElementById('profiles').hidden) return false;
      if (el.closest('.ns-player') && !el.classList.contains('ns-player')) return false;   // Player-Knöpfe: nur über Tasten
      if (el.classList.contains('ns-tick')) return false;
      return visible(el);
    });
  }
  function focusEl(el) {
    el.focus({ preventScroll: true });
    el.scrollIntoView({ block: 'center', inline: 'center', behavior: 'smooth' });
  }
  function move(dir) {
    const els = candidates();
    if (!els.length) return;
    let cur = document.activeElement;
    if (!els.includes(cur)) cur = null;
    if (!cur) {
      const inView = els.find((el) => { const r = el.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; });
      focusEl(inView || els[0]);
      return;
    }
    const r = cur.getBoundingClientRect();
    const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of els) {
      if (el === cur) continue;
      const b = el.getBoundingClientRect();
      const dx = b.left + b.width / 2 - cx, dy = b.top + b.height / 2 - cy;
      const horiz = dir === 'left' || dir === 'right';
      const along = horiz ? (dir === 'right' ? dx : -dx) : (dir === 'down' ? dy : -dy);
      if (along <= 6) continue;
      const across = Math.abs(horiz ? dy : dx);
      const overlap = horiz ? !(b.bottom < r.top || b.top > r.bottom) : !(b.right < r.left || b.left > r.right);
      const score = along + across * (overlap ? 0.3 : 3);
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (best) focusEl(best);
  }
  const DIRS = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
  const KC_DIRS = { 37: 'left', 38: 'up', 39: 'right', 40: 'down' };
  const isBack = (e) => ['BrowserBack', 'GoBack', 'XF86Back'].includes(e.key) || [10009, 461, 166].includes(e.keyCode);
  const video = () => document.querySelector('.ns-player video');
  function toggle() { const v = video(); if (v) { if (v.paused) v.play().catch(() => {}); else v.pause(); } }

  window.addEventListener('keydown', (e) => {
    const ae = document.activeElement;
    const typing = ae && /^(INPUT|TEXTAREA|SELECT)$/.test(ae.tagName);
    const dir = DIRS[e.key] || KC_DIRS[e.keyCode];
    const inPlayer = !!(ae && ae.classList && ae.classList.contains('ns-player'));

    if (isBack(e) || (e.key === 'Escape' && !typing)) {
      const prof = document.getElementById('profiles');
      if (prof && !prof.hidden) return;                       // Auswahl „Wer schaut?“: erst wählen
      if (zoneRoot()) { closePicker(); e.preventDefault(); return; }
      if (document.fullscreenElement) return;                 // Vollbild beendet der Player selbst
      e.preventDefault(); e.stopPropagation();
      if (history.length > 1) history.back();
      return;
    }
    if (typing && dir !== 'up' && dir !== 'down') return;     // im Textfeld: Pfeile links/rechts bearbeiten den Text
    if (e.key === 'MediaPlayPause' || e.key === 'MediaPlay' || e.key === 'MediaPause') { e.preventDefault(); toggle(); return; }
    if (e.key === 'MediaFastForward' || e.key === 'MediaRewind') {
      const v = video(); if (v) { v.currentTime = Math.max(0, v.currentTime + (e.key === 'MediaFastForward' ? 30 : -30)); e.preventDefault(); }
      return;
    }
    if (inPlayer && (e.key === 'Enter' || e.keyCode === 13)) { e.preventDefault(); e.stopPropagation(); toggle(); return; }
    if (dir) {
      if (inPlayer && (dir === 'left' || dir === 'right')) return;   // der Player spult selbst
      e.preventDefault(); e.stopPropagation();
      move(dir);
    }
  }, true);

  // Fokus setzen, sobald eine Seite fertig ist (auf der Folgenseite: der Player)
  let focusTimer = null;
  const mo = new MutationObserver(() => {
    clearTimeout(focusTimer);
    focusTimer = setTimeout(() => {
      const ae = document.activeElement;
      if (ae && ae !== document.body && ae.isConnected && visible(ae)) return;
      if (typeof P === 'undefined' || !P || zoneRoot()) return;
      const pl = document.querySelector('.ns-player');
      if (pl) { pl.setAttribute('tabindex', '0'); focusEl(pl); return; }
      const first = candidates().find((el) => el.closest('#main')) || candidates()[0];
      if (first) focusEl(first);
    }, 150);
  });
  const mainEl = document.getElementById('main');
  if (mainEl) mo.observe(mainEl, { childList: true });
  const prof = document.getElementById('profiles');
  if (prof) mo.observe(prof, { attributes: true, attributeFilter: ['hidden'] });
})();
