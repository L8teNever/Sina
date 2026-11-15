// Geräte: jedes offene Sina meldet sich mit seinem Profil an, teilt seinen Wiedergabe-Stand und kann Folgen
// an andere Geräte mit demselben Profil schicken oder sie vom Handy aus fernsteuern.
// Fernseher werden erkannt und mit der Fernbedienung (Pfeiltasten, OK, Zurück) gesteuert.
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

  const UA = navigator.userAgent;
  const IPAD = /iPad/.test(UA) || (/Macintosh/.test(UA) && navigator.maxTouchPoints > 1);
  const kind = () => (window.SINA_TV ? 'tv' : IPAD || (/Android/.test(UA) && !/Mobile/.test(UA)) || /Tablet/.test(UA) ? 'tablet' : /iPhone|iPod|Android.*Mobile|Mobile/.test(UA) ? 'phone' : 'computer');
  let deviceId = lget('deviceId', null);
  if (!deviceId) { deviceId = Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4); lset('deviceId', deviceId); }
  const defaultName = () => {
    const k = kind();
    let n;
    if (k === 'tv') n = /Tizen/i.test(UA) ? 'Samsung TV' : /Web0S|WebOS|NetCast/i.test(UA) ? 'LG TV' : /BRAVIA/i.test(UA) ? 'Sony TV' : /AFT[A-Z]|FireTV/i.test(UA) ? 'Fire TV' : /Android ?TV|Chromecast|CrKey/i.test(UA) ? 'Android TV' : /Roku/i.test(UA) ? 'Roku' : 'Fernseher';
    else if (k === 'tablet') n = IPAD ? 'iPad' : 'Tablet';
    else if (k === 'phone') n = /iPhone/.test(UA) ? 'iPhone' : 'Android-Handy';
    else n = /Macintosh|Mac OS X/.test(UA) ? 'Mac' : /Windows/.test(UA) ? 'Windows-PC' : /CrOS/.test(UA) ? 'Chromebook' : 'Computer';
    return n + ' ' + deviceId.slice(0, 3);
  };
  const deviceName = () => (lget('deviceName', '') || '').trim() || defaultName();
  const ICON = {
    tv: 'M21 3H3c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h5v2h8v-2h5c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 14H3V5h18v12z',
    phone: 'M16 1H8C6.34 1 5 2.34 5 4v16c0 1.66 1.34 3 3 3h8c1.66 0 3-1.34 3-3V4c0-1.66-1.34-3-3-3zm-2 20h-4v-1h4v1zm3.25-3H6.75V4h10.5v14z',
    tablet: 'M18.5 0h-14C3.12 0 2 1.12 2 2.5v19C2 22.88 3.12 24 4.5 24h14c1.38 0 2.5-1.12 2.5-2.5v-19C21 1.12 19.88 0 18.5 0zm-7 23c-.83 0-1.5-.67-1.5-1.5s.67-1.5 1.5-1.5 1.5.67 1.5 1.5-.67 1.5-1.5 1.5zm7.5-4H4V3h15v16z',
    computer: 'M20 18c1.1 0 1.99-.9 1.99-2L22 6c0-1.1-.9-2-2-2H4c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2H0v2h24v-2h-4zM4 6h16v10H4V6z',
  };
  const SVG = {
    play: '<svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>',
    pause: '<svg viewBox="0 0 24 24"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>',
    prev: '<svg viewBox="0 0 24 24"><path d="M6 6h2v12H6zm3.5 6 8.5 6V6z"/></svg>',
    next: '<svg viewBox="0 0 24 24"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>',
    skip: '<svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>',
    vol: '<svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z"/></svg>',
    mute: '<svg viewBox="0 0 24 24"><path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"/></svg>',
    close: '<svg viewBox="0 0 24 24"><path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>',
    fs: '<svg viewBox="0 0 24 24"><path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z"/></svg>',
    cast: '<svg viewBox="0 0 24 24"><path d="M21 3H3c-1.1 0-2 .9-2 2v3h2V5h18v14h-7v2h7c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11z"/></svg>',
  };
  const icon = (k) => '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="' + (ICON[k] || ICON.computer) + '"/></svg>';
  const escH = (t) => String(t ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  const fmtT = (sec) => {
    if (!isFinite(sec) || sec < 0) return '0:00';
    const s = Math.floor(sec % 60), m = Math.floor((sec / 60) % 60), h = Math.floor(sec / 3600);
    const pad = (n) => String(n).padStart(2, '0');
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
  };
  const isEpHash = (h) => /^#\/anime\/stream\/.+\/(episode|film)-\d+/.test(h || '');

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

  // ---------- Wiedergabe-Stand dieses Geräts melden ----------
  let lastReportedActive = false;
  let statePushTimer = null;
  let castLock = null;   // { devId, name, hash }: Handy hat weitergeschickt
  function buildLocalState() {
    const ctx = window.__sinaCtx;
    const rem = window.__sinaRemote;
    const h = location.hash || '';
    if (!ctx && !rem) return { active: false };
    if (!isEpHash(h)) return { active: false };
    if (castLock && castLock.hash === h) return { active: false };
    const st = rem && typeof rem.getState === 'function' ? rem.getState() : null;
    const title = ctx ? (ctx.series + (ctx.epLabel ? ' · ' + ctx.epLabel : '')) : document.title.replace(/ – Sina$/, '');
    const epLabel = ctx && ctx.title ? ctx.title : (ctx && ctx.epLabel ? ctx.epLabel : '');
    return {
      active: true,
      hash: h,
      title,
      epLabel,
      paused: st ? !!st.paused : true,
      pos: st ? st.pos : 0,
      dur: st ? st.dur : 0,
      speed: st ? st.speed : 1,
      volume: st ? st.volume : 1,
      muted: st ? !!st.muted : false,
      dynAudio: st ? !!st.dynAudio : false,
      autoNext: st ? !!st.autoNext : true,
      skipMode: st ? st.skipMode : null,
      hasPrev: !!(ctx && ctx.prev),
      hasNext: !!(ctx && ctx.next),
      fullscreen: st ? !!st.fullscreen : false,
      quality: st && st.quality ? String(st.quality) : 'auto',
      qualities: st && Array.isArray(st.qualities) ? st.qualities : [],
    };
  }
  function pushDeviceStateNow(keepalive) {
    clearTimeout(statePushTimer);
    if (typeof P === 'undefined' || !P) return;
    const payload = buildLocalState();
    if (!payload.active && !lastReportedActive) return;
    lastReportedActive = !!payload.active;
    const q = new URLSearchParams({ id: deviceId, profile: P, name: deviceName(), kind: kind() });
    fetch('/api/devices/state?' + q, {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: JSON.stringify(payload),
      keepalive: !!keepalive,
    }).catch(() => {});
  }
  function scheduleDeviceState(immediate) {
    clearTimeout(statePushTimer);
    if (immediate) { pushDeviceStateNow(false); return; }
    statePushTimer = setTimeout(() => pushDeviceStateNow(false), 220);
  }
  window.__sinaRemoteChanged = function () {
    scheduleDeviceState(!window.__sinaRemote && !window.__sinaCtx);
  };
  setInterval(() => {
    if (document.hidden || typeof P === 'undefined' || !P) return;
    if (window.__sinaRemote) {
      const st = window.__sinaRemote.getState();
      if (!st.paused) pushDeviceStateNow(false);
    }
  }, 3000);
  window.addEventListener('pagehide', () => {
    if (typeof P !== 'undefined' && P && lastReportedActive) {
      const q = new URLSearchParams({ id: deviceId, profile: P, name: deviceName(), kind: kind() });
      fetch('/api/devices/state?' + q, { method: 'POST', headers: { 'Content-Type': 'text/plain' }, body: '{"active":false}', keepalive: true }).catch(() => {});
    }
  });

  // ---------- Jedes Gerät: anmelden und auf Befehle warten ----------
  let pollCtl = null;
  async function pollLoop(prof, ctl) {
    let fails = 0;
    while (!ctl.signal.aborted) {
      try {
        const r = await fetch('/api/devices/poll?' + new URLSearchParams({ id: deviceId, profile: prof, name: deviceName(), kind: kind() }), { signal: ctl.signal });
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
    if (!c) return;
    if (c.type === 'open') {
      if (!/^#\//.test(c.hash || '')) return;
      if (c.position > 3 && /\/(episode|film)-\d+$/.test(c.hash)) {
        const path = decodeURIComponent(c.hash.replace(/^#/, '').split('?')[0]).replace(/\/+$/, '');
        try { localStorage.setItem('nina_ep_pos_' + path, JSON.stringify({ pos: c.position, dur: c.duration || 0, at: Date.now() })); } catch {}
      }
      toast((c.from ? 'Von ' + c.from : 'Von einem anderen Gerät') + ': ' + (c.title || 'weiter schauen'));
      if (location.hash === c.hash) { if (typeof route === 'function') route(); } else location.hash = c.hash;
      return;
    }
    if (c.type === 'control') {
      if (window.__sinaRemote && typeof window.__sinaRemote.exec === 'function') {
        window.__sinaRemote.exec(c.action, c.value);
        setTimeout(() => pushDeviceStateNow(false), 120);
      } else if (c.action === 'next' && window.__sinaCtx && window.__sinaCtx.next) {
        location.hash = window.__sinaCtx.next.href;
      } else if (c.action === 'prev' && window.__sinaCtx && window.__sinaCtx.prev) {
        location.hash = window.__sinaCtx.prev.href;
      }
    }
  }

  // ---------- Andere Geräte mit demselben Profil ----------
  let others = [];
  let remoteTargetId = null;
  let lastRemoteId = null;
  let miniDismissed = false;
  let remotePollTimer = null;
  let remoteTickTimer = null;
  let remoteDraggingSeek = false;
  let remoteDraggingVol = false;
  let remoteLocalPos = { id: '', pos: 0, at: 0 };

  async function refreshOthers() {
    if (typeof P === 'undefined' || !P || document.hidden) return;
    try {
      const r = await fetch('/api/devices?' + new URLSearchParams({ profile: P, me: deviceId }));
      others = r.ok ? await r.json() : [];
    } catch { others = []; }
    updateCastUI();
    updateRemoteIfOpen();
    renderMiniRemote();
  }
  function liveOthers() {
    return others.filter((d) => d.state && d.state.active);
  }
  function updateCastUI() {
    const live = liveOthers();
    const b = document.getElementById('castBtn');
    if (b) {
      b.hidden = typeof P === 'undefined' || !P;
      b.classList.toggle('has-live', live.length > 0);
      b.title = live.length === 1
        ? 'Fernbedienung: ' + live[0].name
        : others.length === 1
          ? 'Auf ' + others[0].name + ' weiter schauen / steuern'
          : others.length ? 'Geräte & Fernbedienung' : 'Auf anderem Gerät weiter schauen';
    }
    document.querySelectorAll('.a16-cast-btn').forEach((pb) => {
      pb.classList.toggle('has-devs', others.length > 0);
      pb.title = others.length === 1 ? 'Auf ' + others[0].name + ' weiter schauen' : 'Auf anderem Gerät weiter schauen';
    });
    const e = document.getElementById('castEp');
    if (e) {
      e.hidden = typeof P === 'undefined' || !P;
      const lbl = e.querySelector('span');
      if (lbl) lbl.textContent = others.length === 1 ? 'Auf ' + others[0].name + ' weiter schauen' : 'Auf anderem Gerät weiter schauen';
    }
  }
  async function sendCmd(toId, cmd) {
    const r = await fetch('/api/devices/send?' + new URLSearchParams({ to: toId, profile: P }), {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: JSON.stringify({ ...cmd, from: deviceName() }),
    });
    if (!r.ok) {
      const j = await r.json().catch(() => ({}));
      throw new Error(j.error || 'Gerät nicht erreichbar.');
    }
    return true;
  }
  async function sendControl(toId, action, value) {
    try {
      await sendCmd(toId, { type: 'control', action, value });
      setTimeout(refreshOthers, 180);
      setTimeout(refreshOthers, 600);
    } catch (e) {
      toast(e.message || 'Gerät nicht erreichbar.');
      refreshOthers();
    }
  }
  async function castTo(dev, openRemAfter) {
    const v = document.querySelector('.ns-player video');
    const pos = v && v.currentTime > 3 ? v.currentTime : 0;
    const dur = v && isFinite(v.duration) ? v.duration : 0;
    const cmd = { type: 'open', hash: location.hash || '#/', position: pos, duration: dur, title: document.title.replace(/ – Sina$/, '') };
    try {
      await sendCmd(dev.id, cmd);
      if (v) { try { v.pause(); } catch {} }
      if (!window.SINA_TV) setCastLock(dev);
      lastRemoteId = dev.id;
      miniDismissed = false;
      toast('Läuft jetzt auf ' + dev.name);
      if (openRemAfter !== false && !window.SINA_TV) {
        setTimeout(() => { openRemote(dev.id); refreshOthers(); }, 250);
      }
    } catch (e) {
      toast(e.message || 'Gerät nicht erreichbar.');
      refreshOthers();
    }
  }

  // ---------- Handy als Fernbedienung: lokal nicht weiter abspielen ----------
  function setCastLock(dev) {
    castLock = { devId: dev.id, name: dev.name, hash: location.hash };
    pushDeviceStateNow(false);
    syncCastLock();
  }
  function releaseCastLock() {
    castLock = null;
    document.querySelectorAll('.sina-castlock').forEach((el) => el.remove());
  }
  function syncCastLock() {
    if (castLock && castLock.hash !== location.hash) releaseCastLock();
    if (!castLock) return;
    const pl = document.querySelector('.ns-player');
    if (!pl) return;
    const v = pl.querySelector('video');
    if (v && !v.paused) { try { v.pause(); } catch {} }
    let ov = pl.querySelector('.sina-castlock');
    if (!ov) {
      ov = document.createElement('div');
      ov.className = 'sina-castlock';
      ov.innerHTML = `${SVG.cast}<div class="scl-txt">Läuft auf <b></b></div>
        <div class="scl-acts">
          <button type="button" class="ns-btn" data-scl="remote">Fernbedienung</button>
          <button type="button" class="ns-tonal" data-scl="here">Hier abspielen</button>
        </div>`;
      ov.addEventListener('click', (e) => {
        e.stopPropagation();
        const b = e.target.closest('[data-scl]');
        if (!b || !castLock) return;
        if (b.dataset.scl === 'remote') { openRemote(castLock.devId); return; }
        const dev = others.find((d) => d.id === castLock.devId);
        const st = dev && dev.state && dev.state.active && dev.state.hash === castLock.hash ? dev.state : null;
        const pos = st ? currentRemotePos(dev) : null;
        sendControl(castLock.devId, 'pause');
        releaseCastLock();
        const vv = document.querySelector('.ns-player video');
        if (vv) {
          if (pos != null && pos > 0) { try { vv.currentTime = pos; } catch {} }
          vv.play().catch(() => {});
        }
      });
      pl.appendChild(ov);
    }
    const nameEl = ov.querySelector('b');
    if (nameEl.textContent !== castLock.name) nameEl.textContent = castLock.name;
  }
  // Startet das Video lokal trotzdem (Autoplay, Taste), sofort wieder anhalten.
  document.addEventListener('play', (e) => {
    if (!castLock || castLock.hash !== location.hash) return;
    const t = e.target;
    if (t && t.tagName === 'VIDEO' && t.closest && t.closest('.ns-player')) { try { t.pause(); } catch {} }
  }, true);
  window.addEventListener('hashchange', () => setTimeout(syncCastLock, 0));
  setInterval(syncCastLock, 1000);

  // ---------- Modals: Geräteauswahl & Fernbedienung ----------
  function closeModal() {
    clearInterval(remotePollTimer); remotePollTimer = null;
    clearInterval(remoteTickTimer); remoteTickTimer = null;
    remoteTargetId = null;
    const m = document.querySelector('.sina-modal');
    if (m) m.remove();
    renderMiniRemote();
  }
  function openPicker(fromEp) {
    closeModal();
    const root = document.getElementById('nina-skin');
    const who = (typeof profile === 'function' ? profile().name : '');
    const onEp = fromEp || isEpHash(location.hash);
    const m = document.createElement('div');
    m.className = 'sina-modal';
    m.innerHTML = `<div class="sina-sheet" role="dialog">
      <div class="sina-sheet-head"><h2>${onEp ? 'Auf welchem Gerät?' : 'Verbundene Geräte'}</h2><button type="button" class="sina-x" data-close aria-label="Schließen">${SVG.close}</button></div>
      ${others.length
        ? `<p>Geräte mit Profil „${escH(who)}“. Du kannst Folgen hinschicken oder sie direkt fernsteuern.</p>
          <div class="sina-devlist">${others.map((t, i) => {
            const st = t.state && t.state.active ? t.state : null;
            const sub = st ? `${st.title || 'Wiedergabe'} · ${st.paused ? 'Pausiert' : 'Läuft'} · ${fmtT(st.pos)}${st.dur ? ' / ' + fmtT(st.dur) : ''}` : 'Bereit';
            return `<div class="sina-devrow">
              <button type="button" class="sina-tvbtn" data-action="${onEp ? 'cast' : 'remote'}" data-dev="${i}">
                ${icon(t.kind)}
                <span class="sina-devtxt"><b>${escH(t.name)}</b><small>${escH(sub)}</small></span>
                ${st ? '<span class="sina-live-dot" title="Spielt gerade"></span>' : ''}
              </button>
              ${onEp && st ? `<button type="button" class="sina-rem-mini-btn" data-action="remote" data-dev="${i}" title="Fernbedienung öffnen">Steuern</button>` : ''}
            </div>`;
          }).join('')}</div>`
        : `<p>Gerade ist kein anderes Gerät da. Öffne Sina auf dem Fernseher, Mac, PC oder Tablet und wähle dort ebenfalls „${escH(who)}“. Dann taucht es hier auf.</p>`}
      <button type="button" class="sina-tvbtn" data-close style="background:transparent;justify-content:center">${others.length ? 'Abbrechen' : 'OK'}</button>
    </div>`;
    root.appendChild(m);
    m.addEventListener('click', (e) => {
      const b = e.target.closest('[data-dev]');
      if (b) {
        const dev = others[+b.dataset.dev];
        if (!dev) return;
        const act = b.dataset.action;
        closeModal();
        if (act === 'cast') castTo(dev, true); else openRemote(dev.id);
        return;
      }
      if (e.target === m || e.target.closest('[data-close]')) closeModal();
    });
    const first = m.querySelector('[data-dev], [data-close]'); if (first) first.focus();
  }

  function currentRemotePos(dev) {
    const st = dev && dev.state && dev.state.active ? dev.state : null;
    if (!st) return 0;
    let p = st.pos || 0;
    if (!st.paused && remoteLocalPos.id === dev.id && remoteLocalPos.at) {
      const dt = Math.max(0, (Date.now() - remoteLocalPos.at) / 1000) * (st.speed || 1);
      p = Math.min(st.dur || Infinity, p + dt);
    }
    return p;
  }

  function openRemote(devId) {
    closeModal();
    remoteTargetId = devId;
    lastRemoteId = devId;
    miniDismissed = false;
    const root = document.getElementById('nina-skin');
    const m = document.createElement('div');
    m.className = 'sina-modal';
    m.innerHTML = `<div class="sina-sheet sina-remote" role="dialog" aria-label="Fernbedienung"></div>`;
    root.appendChild(m);

    const sheet = m.querySelector('.sina-sheet');
    const getDev = () => others.find((d) => d.id === remoteTargetId);

    const paint = () => {
      const dev = getDev();
      if (!dev) {
        if (sheet.dataset.mode !== 'gone') {
          sheet.dataset.mode = 'gone';
          sheet.innerHTML = `<div class="sina-sheet-head"><h2>Gerät getrennt</h2><button type="button" class="sina-x" data-close aria-label="Schließen">${SVG.close}</button></div>
            <p>Dieses Gerät ist nicht mehr erreichbar.</p>
            <button type="button" class="sina-tvbtn" data-close style="justify-content:center">OK</button>`;
        }
        return;
      }
      const st = dev.state && dev.state.active ? dev.state : null;
      if (st) remoteLocalPos = { id: dev.id, pos: st.pos || 0, at: Date.now() };
      const pos = st ? currentRemotePos(dev) : 0;
      const dur = st ? (st.dur || 0) : 0;
      const spd = st ? (Math.round((st.speed || 1) * 100) / 100) : 1;
      const vol = st ? (st.muted ? 0 : (st.volume ?? 1)) : 1;
      const volMax = vol > 2 ? 6 : 2;
      const skipOp = !!(st && (st.skipMode === 'aniskip_op' || st.skipMode === 'intro'));
      const skipEd = !!(st && (st.skipMode === 'aniskip_ed' || st.skipMode === 'outro'));
      const onEp = isEpHash(location.hash);
      const diffEp = onEp && (!st || st.hash !== location.hash);
      const canPull = !!(st && st.hash && st.hash !== location.hash);
      const speeds = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3];
      const nextMode = st ? 'live' : 'empty';

      if (sheet.dataset.mode !== nextMode) {
        sheet.dataset.mode = nextMode;
        sheet.innerHTML = `
          <div class="sr-head">
            <div class="sr-dev">
              ${icon(dev.kind)}
              <div>
                <div class="sr-name" id="srName"></div>
                <div class="sr-status" id="srStatus"></div>
              </div>
            </div>
            <div class="sr-head-acts">
              <button type="button" class="sina-x" id="srSwitch" title="Gerät wechseln" hidden>${SVG.cast}</button>
              <button type="button" class="sina-x" data-close aria-label="Schließen">${SVG.close}</button>
            </div>
          </div>

          ${st ? `
            <div class="sr-now"><b id="srTitle"></b><small id="srSub"></small></div>

            <div class="sr-seek-wrap">
              <input type="range" class="sr-seek" id="srSeek" min="0" max="100" step="1" value="0">
              <div class="sr-times"><span id="srPos">0:00</span><span id="srDur">--:--</span></div>
            </div>

            <div class="sr-transport">
              <button type="button" class="sr-btn" id="srPrev" data-act="prev" title="Vorherige Folge">${SVG.prev}</button>
              <button type="button" class="sr-btn" data-act="seekBy" data-val="-30" title="30 s zurück">-30s</button>
              <button type="button" class="sr-btn" data-act="seekBy" data-val="-10" title="10 s zurück">-10s</button>
              <button type="button" class="sr-play" id="srToggle" data-act="toggle" title="Play / Pause">${SVG.play}</button>
              <button type="button" class="sr-btn" data-act="seekBy" data-val="10" title="10 s vor">+10s</button>
              <button type="button" class="sr-btn" data-act="seekBy" data-val="30" title="30 s vor">+30s</button>
              <button type="button" class="sr-btn" id="srNext" data-act="next" title="Nächste Folge">${SVG.next}</button>
            </div>

            <div class="sr-skips">
              <button type="button" class="sr-skip" id="srSkipIntro" data-act="skipIntro">
                ${SVG.skip}
                <span id="srSkipIntroTxt">Intro skippen</span>
              </button>
              <button type="button" class="sr-skip" id="srSkipOutro" data-act="skipOutro">
                ${SVG.next}
                <span id="srSkipOutroTxt">Outro skippen</span>
              </button>
            </div>

            <div class="sr-block">
              <div class="sr-lbl">Geschwindigkeit</div>
              <div class="sr-speeds">
                ${speeds.map((s) => `<button type="button" class="sr-pill" data-act="speed" data-val="${s}">${s}x</button>`).join('')}
              </div>
            </div>

            <div class="sr-block">
              <div class="sr-lbl">Qualität</div>
              <div class="sr-quals" id="srQuals"></div>
            </div>

            <div class="sr-block">
              <div class="sr-lbl">Optionen &amp; Ton</div>
              <div class="sr-toggles">
                <button type="button" class="sr-pill" id="srAutoNext" data-act="autoNext">Auto-Weiter: AN</button>
                <button type="button" class="sr-pill" id="srDynAudio" data-act="dynAudio">Sprache +</button>
                <button type="button" class="sr-pill" id="srFs" data-act="fullscreen" title="Vollbild umschalten">${SVG.fs} Vollbild</button>
              </div>
              <div class="sr-vol-row">
                <button type="button" class="sr-btn" id="srMute" data-act="mute">${SVG.vol}</button>
                <input type="range" class="sr-vol" id="srVol" min="0" max="2" step="0.05" value="1">
                <span class="sr-vol-pct" id="srVolPct">100%</span>
              </div>
            </div>
          ` : `
            <div class="sr-empty" id="srEmpty"></div>
          `}

          <div class="sr-foot">
            <button type="button" class="ns-btn" id="srSendHere" hidden></button>
            <button type="button" class="ns-tonal" id="srPullHere" hidden>Auf diesem Gerät weiter schauen</button>
          </div>
        `;
      }

      const q = (id) => sheet.querySelector('#' + id);
      q('srName').textContent = dev.name;
      const statusHtml = st
        ? `<span class="sr-dot ${st.paused ? 'p' : ''}"></span>${st.paused ? 'Pausiert' : 'Läuft'}${spd !== 1 ? ` · ${spd}x` : ''}`
        : 'Bereit (keine Folge offen)';
      const statusEl = q('srStatus');
      if (statusEl.dataset.v !== statusHtml) { statusEl.dataset.v = statusHtml; statusEl.innerHTML = statusHtml; }
      q('srSwitch').hidden = others.length <= 1;

      const sendBtn = q('srSendHere');
      sendBtn.hidden = !diffEp;
      if (diffEp) sendBtn.textContent = 'Aktuelle Folge auf ' + dev.name + ' senden';
      q('srPullHere').hidden = !canPull;

      if (!st) {
        const emptyEl = q('srEmpty');
        if (emptyEl.dataset.n !== dev.name) {
          emptyEl.dataset.n = dev.name;
          emptyEl.innerHTML = `Auf <b>${escH(dev.name)}</b> läuft gerade kein Video. Öffne dort eine Folge oder schicke eine vom Handy aus hin.`;
        }
        return;
      }

      q('srTitle').textContent = st.title || 'Wiedergabe';
      const subEl = q('srSub');
      subEl.textContent = st.epLabel || '';
      subEl.hidden = !st.epLabel;

      const seekEl = q('srSeek');
      seekEl.max = String(dur > 0 ? dur : 100);
      seekEl.disabled = dur <= 0;
      if (!remoteDraggingSeek) {
        seekEl.value = String(Math.min(dur || 100, pos));
        q('srPos').textContent = fmtT(pos);
      }
      q('srDur').textContent = dur > 0 ? fmtT(dur) : '--:--';

      q('srPrev').disabled = !st.hasPrev;
      q('srNext').disabled = !st.hasNext;
      const toggleBtn = q('srToggle');
      const pKey = st.paused ? '1' : '0';
      if (toggleBtn.dataset.p !== pKey) { toggleBtn.dataset.p = pKey; toggleBtn.innerHTML = st.paused ? SVG.play : SVG.pause; }

      q('srSkipIntro').classList.toggle('hot', skipOp);
      q('srSkipIntroTxt').textContent = 'Intro skippen' + (st.skipMode === 'aniskip_op' ? ' · DB' : '');
      q('srSkipOutro').classList.toggle('hot', skipEd);
      q('srSkipOutroTxt').textContent = st.hasNext ? 'Outro / Nächste' : 'Outro skippen';

      sheet.querySelectorAll('[data-act="speed"]').forEach((b) => {
        b.classList.toggle('on', Math.abs(spd - Number(b.dataset.val)) < 0.02);
      });

      const quals = ['auto'].concat((Array.isArray(st.qualities) && st.qualities.length ? st.qualities : [1080, 720, 480, 360]).map(String));
      const qWrap = q('srQuals');
      const qKey = quals.join(',');
      if (qWrap.dataset.k !== qKey) {
        qWrap.dataset.k = qKey;
        qWrap.innerHTML = quals.map((k) => `<button type="button" class="sr-pill" data-act="quality" data-val="${k}">${k === 'auto' ? 'Auto' : k + 'p'}</button>`).join('');
      }
      const curQ = String(st.quality || 'auto');
      qWrap.querySelectorAll('[data-act="quality"]').forEach((b) => b.classList.toggle('on', b.dataset.val === curQ));

      const anBtn = q('srAutoNext');
      anBtn.classList.toggle('on', !!st.autoNext);
      anBtn.dataset.val = String(!st.autoNext);
      anBtn.textContent = 'Auto-Weiter: ' + (st.autoNext ? 'AN' : 'AUS');

      q('srDynAudio').classList.toggle('on', !!st.dynAudio);
      q('srFs').classList.toggle('on', !!st.fullscreen);

      const muteBtn = q('srMute');
      muteBtn.title = st.muted ? 'Ton an' : 'Stumm';
      const mKey = (st.muted || vol === 0) ? '1' : '0';
      if (muteBtn.dataset.m !== mKey) { muteBtn.dataset.m = mKey; muteBtn.innerHTML = mKey === '1' ? SVG.mute : SVG.vol; }

      const volEl = q('srVol');
      volEl.max = String(volMax);
      if (!remoteDraggingVol) {
        volEl.value = String(vol);
        q('srVolPct').textContent = Math.round(vol * 100) + '%';
      }
    };

    paint();
    sheet._paint = paint;

    // Interaktionen im Fernbedienungs-Fenster
    sheet.addEventListener('click', (e) => {
      if (e.target.closest('[data-close]')) { closeModal(); return; }
      if (e.target.closest('#srSwitch')) { openPicker(false); return; }
      const dev = getDev();
      if (!dev) return;
      if (e.target.closest('#srSendHere')) { castTo(dev, false); return; }
      if (e.target.closest('#srPullHere')) {
        const st = dev.state;
        if (!st || !st.hash) return;
        const pos = currentRemotePos(dev);
        if (pos > 3 && /\/(episode|film)-\d+$/.test(st.hash)) {
          const path = decodeURIComponent(st.hash.replace(/^#/, '').split('?')[0]).replace(/\/+$/, '');
          try { localStorage.setItem('nina_ep_pos_' + path, JSON.stringify({ pos, dur: st.dur || 0, at: Date.now() })); } catch {}
        }
        sendControl(dev.id, 'pause');
        closeModal();
        releaseCastLock();
        if (location.hash === st.hash) { if (typeof route === 'function') route(); } else location.hash = st.hash;
        return;
      }
      const btn = e.target.closest('[data-act]');
      if (!btn || btn.disabled) return;
      const act = btn.dataset.act;
      let val = btn.dataset.val;
      if (val === 'true') val = true;
      else if (val === 'false') val = false;
      else if (val === 'auto') val = 'auto';
      else if (val !== undefined) val = Number(val);

      // Sofortiges visuelles Feedback vor der Server-Antwort
      if (dev.state) {
        if (act === 'toggle') dev.state.paused = !dev.state.paused;
        else if (act === 'speed') dev.state.speed = val;
        else if (act === 'quality') dev.state.quality = String(val);
        else if (act === 'autoNext') dev.state.autoNext = val;
        else if (act === 'dynAudio') dev.state.dynAudio = !dev.state.dynAudio;
        else if (act === 'mute') dev.state.muted = !dev.state.muted;
        else if (act === 'seekBy') {
          const cur = currentRemotePos(dev);
          dev.state.pos = Math.max(0, Math.min(dev.state.dur || Infinity, cur + val));
          remoteLocalPos = { id: dev.id, pos: dev.state.pos, at: Date.now() };
        }
        paint();
      }
      sendControl(dev.id, act, val);
    });

    sheet.addEventListener('input', (e) => {
      if (e.target.id === 'srSeek') {
        remoteDraggingSeek = true;
        const p = document.getElementById('srPos');
        if (p) p.textContent = fmtT(Number(e.target.value));
      } else if (e.target.id === 'srVol') {
        remoteDraggingVol = true;
        const p = document.getElementById('srVolPct');
        if (p) p.textContent = Math.round(Number(e.target.value) * 100) + '%';
      }
    });
    sheet.addEventListener('change', (e) => {
      const dev = getDev();
      if (!dev) return;
      if (e.target.id === 'srSeek') {
        remoteDraggingSeek = false;
        const val = Number(e.target.value);
        if (dev.state) { dev.state.pos = val; remoteLocalPos = { id: dev.id, pos: val, at: Date.now() }; }
        sendControl(dev.id, 'seek', val);
      } else if (e.target.id === 'srVol') {
        remoteDraggingVol = false;
        const val = Number(e.target.value);
        if (dev.state) { dev.state.volume = val; dev.state.muted = val === 0; }
        sendControl(dev.id, 'volume', val);
      }
    });

    m.addEventListener('click', (e) => { if (e.target === m) closeModal(); });

    // Schnelle Aktualisierung solange die Fernbedienung offen ist
    remotePollTimer = setInterval(refreshOthers, 1500);
    remoteTickTimer = setInterval(() => {
      if (remoteDraggingSeek || remoteDraggingVol) return;
      const dev = getDev();
      if (!dev || !dev.state || !dev.state.active || dev.state.paused) return;
      const p = currentRemotePos(dev);
      const posEl = document.getElementById('srPos');
      const seekEl = document.getElementById('srSeek');
      if (posEl) posEl.textContent = fmtT(p);
      if (seekEl) seekEl.value = String(Math.min(Number(seekEl.max) || p, p));
    }, 500);
  }

  function updateRemoteIfOpen() {
    if (!remoteTargetId || remoteDraggingSeek || remoteDraggingVol) return;
    const sheet = document.querySelector('.sina-modal .sina-remote');
    if (sheet && typeof sheet._paint === 'function') sheet._paint();
  }

  // ---------- Schwebende Mini-Fernbedienung ----------
  function renderMiniRemote() {
    const existing = document.getElementById('sinaMiniRemote');
    if (window.SINA_TV || remoteTargetId || miniDismissed) {
      if (existing) existing.remove();
      return;
    }
    const dev = (lastRemoteId && others.find((d) => d.id === lastRemoteId && d.state && d.state.active)) || null;
    if (!dev) {
      if (existing) existing.remove();
      return;
    }
    const root = document.getElementById('nina-skin');
    if (!root) return;
    let bar = existing;
    if (!bar) {
      bar = document.createElement('div');
      bar.id = 'sinaMiniRemote';
      bar.className = 'sina-mini-remote';
      bar.innerHTML = `
        <div class="smr-info">
          <span class="smr-ico"></span>
          <div class="smr-txt"><b class="smr-title"></b><small class="smr-sub"></small></div>
        </div>
        <button type="button" class="smr-skip" title="Überspringen" hidden>${SVG.skip}</button>
        <button type="button" class="smr-play" title="Play / Pause"></button>
        <button type="button" class="smr-close" aria-label="Ausblenden">${SVG.close}</button>
      `;
      root.appendChild(bar);
      bar.addEventListener('click', (e) => {
        if (e.target.closest('.smr-close')) { miniDismissed = true; bar.remove(); return; }
        const curDev = others.find((d) => d.id === lastRemoteId);
        if (!curDev) return;
        if (e.target.closest('.smr-play')) { sendControl(curDev.id, 'toggle'); return; }
        if (e.target.closest('.smr-skip')) { sendControl(curDev.id, 'skip'); return; }
        openRemote(curDev.id);
      });
    }
    const st = dev.state;
    const hasSkip = st.skipMode === 'aniskip_op' || st.skipMode === 'intro' || st.skipMode === 'aniskip_ed' || st.skipMode === 'outro';
    const icoEl = bar.querySelector('.smr-ico');
    if (icoEl.dataset.k !== dev.kind) { icoEl.dataset.k = dev.kind; icoEl.innerHTML = icon(dev.kind); }
    bar.querySelector('.smr-title').textContent = st.title || dev.name;
    bar.querySelector('.smr-sub').textContent = `${dev.name} · ${st.paused ? 'Pausiert' : 'Läuft'}`;
    bar.querySelector('.smr-skip').hidden = !hasSkip;
    const playBtn = bar.querySelector('.smr-play');
    const pKey = st.paused ? '1' : '0';
    if (playBtn.dataset.p !== pKey) { playBtn.dataset.p = pKey; playBtn.innerHTML = st.paused ? SVG.play : SVG.pause; }
  }

  async function onCastBtn() {
    await refreshOthers();
    if (others.length === 1) {
      const dev = others[0];
      if (dev.state && dev.state.active) openRemote(dev.id);
      else if (isEpHash(location.hash)) castTo(dev, true);
      else openRemote(dev.id);
    } else {
      openPicker(false);
    }
  }
  async function onCastEp() {
    await refreshOthers();
    if (others.length === 1) castTo(others[0], true);
    else openPicker(true);
  }
  // aus der Player-Leiste: Vollbild verlassen, sonst wäre die Auswahl unsichtbar
  window.sinaCast = async function () {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else if (document.webkitFullscreenElement && document.webkitExitFullscreen) document.webkitExitFullscreen();
    } catch {}
    onCastEp();
  };
  document.addEventListener('click', (e) => {
    if (e.target.closest('#castBtn')) { e.preventDefault(); onCastBtn(); return; }
    if (e.target.closest('#castEp')) { e.preventDefault(); onCastEp(); }
  });

  // ---------- Von der App aufgerufen ----------
  window.sinaOnProfile = function () {
    if (pollCtl) { pollCtl.abort(); pollCtl = null; }
    others = []; lastReportedActive = false; closeModal(); updateCastUI(); renderMiniRemote();
    if (typeof P === 'undefined' || !P) return;
    pollCtl = new AbortController();
    pollLoop(P, pollCtl);
    setTimeout(refreshOthers, 400);
    setTimeout(() => pushDeviceStateNow(false), 300);
  };
  window.sinaEpisodeRendered = function () { updateCastUI(); refreshOthers(); scheduleDeviceState(false); };
  setInterval(refreshOthers, 8000);
  document.addEventListener('visibilitychange', () => { if (!document.hidden) refreshOthers(); });

  // Einstellungen: Name und Modus dieses Geräts
  window.sinaDeviceCard = function () {
    const m = mode();
    const kname = { tv: 'Fernseher', phone: 'Handy', tablet: 'Tablet', computer: 'Computer' }[kind()];
    return `<section class="sina-card">
      <h2>Dieses Gerät</h2>
      <p>Jedes Gerät mit Sina und demselben Profil kann eine Folge an ein anderes weitergeben („Auf anderem Gerät weiter schauen“) und es per Fernbedienung steuern. Im <b>Fernseher</b>-Modus gibt es große Kacheln und Steuerung mit der Fernbedienung (Pfeiltasten, OK, Zurück).</p>
      <div class="sina-kv"><span>Erkannt als</span><span>${kname}</span><span>Name</span><span><input id="devName" maxlength="30" placeholder="${escH(defaultName())}" value="${escH(lget('deviceName', '') || '')}" style="width:100%;max-width:260px"></span>
      <span>Fernseher-Modus</span><span><select id="devMode"><option value="auto" ${m === 'auto' ? 'selected' : ''}>Automatisch erkennen</option><option value="tv" ${m === 'tv' ? 'selected' : ''}>Immer an</option><option value="normal" ${m === 'normal' ? 'selected' : ''}>Immer aus</option></select></span></div>
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

  // ---------- Fernbedienung (TV-Steuerkreuz) ----------
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
      if (zoneRoot()) { closeModal(); e.preventDefault(); return; }
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
