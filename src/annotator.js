// PlausiDen-Annotator — bookmarklet core (schema_version 1).
//
// Drop-in. Self-contained. No build step. Paste into a bookmark
// `javascript:(function(){ /* this file */ })()`. Idempotent on
// repeat-load — re-injecting the same script reopens the panel
// instead of doubling state.
//
// See docs/ARCHITECTURE.md for the captured-session JSON shape
// and design constraints.

(function () {
  'use strict';

  // Idempotency: if we already injected, just reopen the panel.
  if (window.__plausidenAnnotator__) {
    window.__plausidenAnnotator__.openPanel();
    return;
  }

  // -----------------------------------------------------------------
  // State (in-memory until operator hits Save)
  // -----------------------------------------------------------------
  const startedMs = Date.now();
  const state = {
    schema_version: 1,
    meta: {
      url: window.location.href,
      title: document.title || '',
      user_agent: navigator.userAgent || '',
      viewport: { w: window.innerWidth, h: window.innerHeight },
      started_ms: startedMs,
      ended_ms: 0,
      tool: 'annotator-bookmarklet/0.1',
    },
    annotations: [],
    console_log: [],
    errors: [],
    failed_requests: [],
    csp_violations: [],
  };

  let nextAnnotationId = 1;
  let pickMode = false;
  let highlightEl = null;

  function tsOffset() { return Date.now() - startedMs; }

  // -----------------------------------------------------------------
  // Capture: console
  // -----------------------------------------------------------------
  ['log', 'info', 'warn', 'error', 'debug'].forEach(function (level) {
    const orig = console[level];
    console[level] = function () {
      try {
        const args = Array.prototype.slice.call(arguments).map(function (a) {
          if (a == null) return String(a);
          if (typeof a === 'string') return a;
          try { return JSON.stringify(a); } catch (e) { return String(a); }
        });
        state.console_log.push({
          ts_offset_ms: tsOffset(),
          level: level,
          message: args.join(' '),
        });
      } catch (e) { /* never let our wrapper crash the page */ }
      return orig.apply(console, arguments);
    };
  });

  // -----------------------------------------------------------------
  // Capture: errors
  // -----------------------------------------------------------------
  window.addEventListener('error', function (ev) {
    state.errors.push({
      ts_offset_ms: tsOffset(),
      kind: 'onerror',
      message: ev.message || String(ev.error || ''),
      stack: (ev.error && ev.error.stack) || '',
    });
  });
  window.addEventListener('unhandledrejection', function (ev) {
    state.errors.push({
      ts_offset_ms: tsOffset(),
      kind: 'unhandledrejection',
      message: (ev.reason && ev.reason.message) || String(ev.reason || ''),
      stack: (ev.reason && ev.reason.stack) || '',
    });
  });

  // -----------------------------------------------------------------
  // Capture: network (fetch monkey-patch + PerformanceObserver)
  // -----------------------------------------------------------------
  const origFetch = window.fetch;
  if (origFetch) {
    window.fetch = function () {
      const t0 = performance.now();
      const args = Array.prototype.slice.call(arguments);
      const url = (typeof args[0] === 'string') ? args[0] : (args[0] && args[0].url) || '';
      const method = (args[1] && args[1].method) || 'GET';
      return origFetch.apply(window, args).then(function (resp) {
        if (!resp.ok) {
          state.failed_requests.push({
            ts_offset_ms: tsOffset(),
            url: url,
            method: method,
            status: resp.status,
            duration_ms: Math.round(performance.now() - t0),
          });
        }
        return resp;
      }, function (err) {
        state.failed_requests.push({
          ts_offset_ms: tsOffset(),
          url: url,
          method: method,
          status: 0,
          duration_ms: Math.round(performance.now() - t0),
          error: String(err),
        });
        throw err;
      });
    };
  }
  try {
    new PerformanceObserver(function (list) {
      list.getEntries().forEach(function (e) {
        // Only flag XHRs with non-2xx (PerformanceResourceTiming
        // doesn't always carry status; skip until reliable).
        if (e.entryType === 'resource' && e.transferSize === 0 && e.duration > 5000) {
          state.failed_requests.push({
            ts_offset_ms: Math.round(e.startTime),
            url: e.name,
            method: 'unknown',
            status: 0,
            duration_ms: Math.round(e.duration),
            error: 'slow / zero-byte resource',
          });
        }
      });
    }).observe({ type: 'resource', buffered: true });
  } catch (e) { /* PerformanceObserver may be missing on some webviews */ }

  // -----------------------------------------------------------------
  // Capture: CSP violations
  // -----------------------------------------------------------------
  document.addEventListener('securitypolicyviolation', function (ev) {
    state.csp_violations.push({
      ts_offset_ms: tsOffset(),
      directive: ev.effectiveDirective || ev.violatedDirective || '',
      blocked_uri: ev.blockedURI || '',
      violated_directive: ev.violatedDirective || '',
    });
  });

  // -----------------------------------------------------------------
  // DOM utilities
  // -----------------------------------------------------------------
  function makeSelector(el) {
    // Heuristic: tag.cls#id chain up to body, with :nth-child for
    // ambiguous siblings. Keeps selectors readable.
    const parts = [];
    let cur = el;
    while (cur && cur.nodeType === 1 && cur !== document.body) {
      let part = cur.tagName.toLowerCase();
      if (cur.id) { part += '#' + cur.id; parts.unshift(part); break; }
      if (cur.className && typeof cur.className === 'string') {
        const cls = cur.className.trim().split(/\s+/).slice(0, 2).join('.');
        if (cls) part += '.' + cls;
      }
      const parent = cur.parentNode;
      if (parent) {
        const sib = Array.prototype.filter.call(parent.children, function (s) {
          return s.tagName === cur.tagName;
        });
        if (sib.length > 1) {
          const idx = Array.prototype.indexOf.call(parent.children, cur) + 1;
          part += ':nth-child(' + idx + ')';
        }
      }
      parts.unshift(part);
      cur = cur.parentNode;
    }
    return parts.join(' > ');
  }

  function styleDiff(el) {
    const elStyle = window.getComputedStyle(el);
    const baseStyle = window.getComputedStyle(document.body);
    const diff = {};
    for (let i = 0; i < elStyle.length; i++) {
      const prop = elStyle[i];
      const v = elStyle.getPropertyValue(prop);
      if (v !== baseStyle.getPropertyValue(prop)) diff[prop] = v;
    }
    return diff;
  }

  function ariaOf(el) {
    const out = { role: el.getAttribute('role') || null };
    el.getAttributeNames().forEach(function (n) {
      if (n.startsWith('aria-')) out[n] = el.getAttribute(n);
    });
    return out;
  }

  function captureElement(el) {
    const r = el.getBoundingClientRect();
    let outer = el.outerHTML || '';
    if (outer.length > 4096) outer = outer.slice(0, 4096) + '... [truncated]';
    return {
      outerHTML: outer,
      selector: makeSelector(el),
      bbox: { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) },
      computed_style_diff: styleDiff(el),
      aria: ariaOf(el),
    };
  }

  // -----------------------------------------------------------------
  // Pick mode (Chrome-DevTools-Inspect-Element clone)
  // -----------------------------------------------------------------
  function ensureHighlight() {
    if (highlightEl) return highlightEl;
    highlightEl = document.createElement('div');
    highlightEl.style.cssText =
      'position:fixed;z-index:2147483646;pointer-events:none;' +
      'border:2px solid hsl(220 90% 60%);background:hsla(220 90% 60% / 0.18);' +
      'transition:all 60ms ease-out;display:none';
    document.documentElement.appendChild(highlightEl);
    return highlightEl;
  }

  function moveHighlight(el) {
    const h = ensureHighlight();
    if (!el) { h.style.display = 'none'; return; }
    const r = el.getBoundingClientRect();
    h.style.display = 'block';
    h.style.left   = r.left   + 'px';
    h.style.top    = r.top    + 'px';
    h.style.width  = r.width  + 'px';
    h.style.height = r.height + 'px';
  }

  function onPickMove(ev) {
    if (!pickMode) return;
    const el = document.elementFromPoint(ev.clientX, ev.clientY);
    if (el && !panel.contains(el)) moveHighlight(el);
  }

  function onPickClick(ev) {
    if (!pickMode) return;
    const el = document.elementFromPoint(ev.clientX, ev.clientY);
    if (!el || panel.contains(el)) return;
    ev.preventDefault();
    ev.stopPropagation();
    pickMode = false;
    moveHighlight(null);
    promptForAnnotation(el);
  }

  document.addEventListener('mousemove', onPickMove, true);
  document.addEventListener('click', onPickClick, true);

  // -----------------------------------------------------------------
  // Panel UI (Loom-token-aligned)
  // -----------------------------------------------------------------
  const panel = document.createElement('div');
  panel.id = 'plausiden-annotator-panel';
  panel.style.cssText = [
    'position:fixed', 'right:16px', 'bottom:16px', 'z-index:2147483647',
    'width:320px', 'max-height:480px', 'overflow:auto',
    'background:hsl(222 47% 6%)', 'color:hsl(210 20% 92%)',
    'border:1px solid hsl(217 19% 22%)', 'border-radius:8px',
    'font:13px/1.4 ui-monospace,Menlo,monospace', 'padding:12px',
    'box-shadow:0 8px 24px rgba(0,0,0,0.35)',
  ].join(';');
  panel.innerHTML = ''
    + '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">'
    +   '<strong style="font-size:14px">Annotator</strong>'
    +   '<button id="pa-close" style="background:transparent;border:0;color:hsl(215 14% 60%);cursor:pointer;font-size:18px">×</button>'
    + '</div>'
    + '<div style="display:flex;gap:6px;margin-bottom:8px">'
    +   '<button id="pa-pick" style="flex:1;background:hsl(220 90% 60%);color:white;border:0;padding:6px 10px;border-radius:4px;cursor:pointer">Pick element</button>'
    +   '<button id="pa-save" style="flex:1;background:hsl(146 60% 55%);color:white;border:0;padding:6px 10px;border-radius:4px;cursor:pointer">Save</button>'
    + '</div>'
    + '<div id="pa-stats" style="color:hsl(215 14% 60%);font-size:11px;margin-bottom:8px"></div>'
    + '<div id="pa-list"></div>';

  document.documentElement.appendChild(panel);

  function refreshStats() {
    document.getElementById('pa-stats').textContent =
      state.annotations.length + ' annotation(s) · '
      + state.console_log.length + ' log · '
      + state.errors.length + ' err · '
      + state.failed_requests.length + ' net · '
      + state.csp_violations.length + ' csp';
  }

  function refreshList() {
    const list = document.getElementById('pa-list');
    list.innerHTML = state.annotations.map(function (a) {
      return ''
        + '<div style="border-left:3px solid hsl(220 90% 60%);padding:4px 8px;margin-bottom:4px;background:hsl(222 36% 12%);border-radius:0 4px 4px 0">'
        +   '<div style="font-size:10px;text-transform:uppercase;letter-spacing:0.05em;color:hsl(215 14% 60%)">'
        +     a.tag + ' · ' + a.id
        +   '</div>'
        +   '<div style="font-size:12px">' + escapeHtml(a.comment.slice(0, 100)) + (a.comment.length > 100 ? '…' : '') + '</div>'
        + '</div>';
    }).join('');
    refreshStats();
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }

  document.getElementById('pa-close').onclick = function () {
    panel.style.display = 'none';
  };
  document.getElementById('pa-pick').onclick = function () {
    pickMode = !pickMode;
    document.getElementById('pa-pick').textContent = pickMode ? 'Cancel pick' : 'Pick element';
  };
  document.getElementById('pa-save').onclick = function () {
    state.meta.ended_ms = Date.now();
    const blob = new Blob([JSON.stringify(state, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'annotator-' + new Date().toISOString().replace(/[:.]/g, '-') + '.json';
    a.click();
    setTimeout(function () { URL.revokeObjectURL(url); }, 1000);
  };

  function promptForAnnotation(el) {
    // Tiny inline prompt — not a modal because the operator is
    // standing in someone else's app and we don't want to capture
    // their focus aggressively.
    const tags = ['a11y', 'contrast', 'alignment', 'copy', 'perf', 'bug', 'suggestion', 'other'];
    const tag = window.prompt('Tag (one of: ' + tags.join(', ') + '):', 'bug');
    if (!tag) return;
    const comment = window.prompt('Comment:', '');
    if (!comment) return;
    const annot = {
      id: 'a' + (nextAnnotationId++),
      ts_offset_ms: tsOffset(),
      tag: tags.indexOf(tag.trim()) >= 0 ? tag.trim() : 'other',
      comment: comment,
      element: captureElement(el),
    };
    state.annotations.push(annot);
    refreshList();
  }

  refreshStats();

  // Public hooks for the future Crawler step + Tauri shell.
  window.__plausidenAnnotator__ = {
    state: state,
    openPanel: function () { panel.style.display = 'block'; },
    save: document.getElementById('pa-save').onclick,
    snapshot: function () { return JSON.parse(JSON.stringify(state)); },
  };
})();
