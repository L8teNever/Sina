// Ersetzt die Erweiterungs-Schnittstelle (chrome.runtime / chrome.storage), die der Android-16-Player erwartet,
// durch die Sina-App: Kontext der Folge, Fortschritt, Intro/Outro-Zeiten.
(function () {
  'use strict';
  // Profile: Schlüssel des Players (nina_…) landen im Speicher des gewählten Profils, und jede Änderung wird gemeldet.
  window.__sinaProfile = null;
  window.__sinaDirty = null;
  const SP = Storage.prototype;
  const rawGet = SP.getItem, rawSet = SP.setItem, rawRemove = SP.removeItem;
  const mapKey = (st, k) => (st === window.localStorage && window.__sinaProfile && typeof k === 'string' && k.startsWith('nina_') ? 'sina:' + window.__sinaProfile + ':' + k : k);
  const dirty = (st, k) => { if (st === window.localStorage && window.__sinaDirty && typeof k === 'string' && window.__sinaProfile && k.startsWith('sina:' + window.__sinaProfile + ':')) window.__sinaDirty(); };
  SP.getItem = function (k) { return rawGet.call(this, mapKey(this, k)); };
  SP.setItem = function (k, v) { const m = mapKey(this, k); rawSet.call(this, m, v); dirty(this, m); };
  SP.removeItem = function (k) { const m = mapKey(this, k); rawRemove.call(this, m); dirty(this, m); };
  window.__sinaPath = () => decodeURIComponent((location.hash || '').replace(/^#/, '').split('?')[0]).replace(/\/+$/, '');
  window.__sinaCtx = null;     // { series, season, episode, movie, title, next:{href,label} } – setzt die App
  window.__sinaSkip = null;    // Intro/Outro-Zeiten, falls die Quelle sie mitliefert (ani.pm)
  window.__sinaOnProgress = null;
  let autoplay = false;

  const answer = (cb, value) => { if (typeof cb === 'function') setTimeout(() => cb(value), 0); };

  const runtime = {
    lastError: undefined,
    sendMessage(msg, cb) {
      const op = msg && msg.op;
      if (op === 'ctx') return answer(cb, { ok: true, result: window.__sinaCtx || {} });
      if (op === 'progress') {
        try { if (window.__sinaOnProgress) window.__sinaOnProgress(msg.data); } catch (_) {}
        return answer(cb, { ok: true });
      }
      if (op === 'setAutoplay') { autoplay = true; return answer(cb, { ok: true }); }
      if (op === 'autoplay') { const was = autoplay; if (msg.take) autoplay = false; return answer(cb, { ok: true, result: was }); }
      if (op === 'next') return answer(cb, { ok: true });
      if (op === 'getAniSkipTimes') {
        if (window.__sinaSkip) return answer(cb, { ok: true, result: window.__sinaSkip });
        const q = new URLSearchParams({ series: msg.series || '', season: msg.season || 1, episode: msg.episode || 1, duration: msg.duration || 0 });
        fetch('/api/aniskip?' + q).then((r) => r.json()).then((j) => cb && cb({ ok: true, result: j })).catch(() => cb && cb({ ok: false }));
        return;
      }
      answer(cb, { ok: false });
    },
  };

  const storage = {
    local: {
      get(keys, cb) {
        const out = {};
        for (const k of [].concat(keys || [])) {
          try { const v = localStorage.getItem('sina:' + (window.__sinaProfile || 'x') + ':ext:' + k); if (v !== null) out[k] = JSON.parse(v); } catch (_) {}
        }
        answer(cb, out);
      },
      set(obj, cb) {
        for (const [k, v] of Object.entries(obj || {})) { try { localStorage.setItem('sina:' + (window.__sinaProfile || 'x') + ':ext:' + k, JSON.stringify(v)); } catch (_) {} }
        answer(cb, undefined);
      },
    },
    onChanged: { addListener() {} },
  };

  window.chrome = window.chrome || {};
  window.chrome.runtime = runtime;
  window.chrome.storage = storage;
})();
