# PlausiDen-Annotator — Architecture

## Goal

> Let the operator browse software UI, click elements, comment, and
> walk away with a JSON session that captures what they saw, what
> they said about it, and what the page was doing in the background
> (console + errors + network).

## Three shapes, one session format

```
┌─────────────────────────────┐     ┌─────────────────────────────┐
│  Bookmarklet                │     │  PlausiDen-Crawler step     │
│                             │     │                             │
│  Drop into any browser.     │     │  `annotate` step kind in    │
│  Inject src/annotator.js    │     │  a journey JSON. Crawler    │
│  into the target page.      │     │  drives Playwright; pauses  │
│  All capture happens in     │     │  at the step; same payload  │
│  page context.              │     │  format emitted.            │
│                             │     │                             │
│  → JSON download            │     │  → findings/<journey>.json  │
└──────────┬──────────────────┘     └──────────┬──────────────────┘
           │                                   │
           └─────────────┬─────────────────────┘
                         ▼
            ┌─────────────────────────────┐
            │  Captured-session JSON      │
            │  (examples/sample-session.json)
            │                             │
            │  - meta (url, ua, ts)       │
            │  - annotations[]            │
            │  - console_log[]            │
            │  - errors[]                 │
            │  - failed_requests[]        │
            │  - csp_violations[]         │
            └──────────┬──────────────────┘
                       ▼
              ┌────────────────────┐
              │  Consumers         │
              │                    │
              │  - human read      │
              │  - Claude analysis │
              │  - LFI ingest      │
              │  - CRM ticket      │
              │  - Audits finding  │
              └────────────────────┘
```

## Captured-session JSON shape

The single source of truth across all three shapes. See
`examples/sample-session.json` for a fully-populated example.

```jsonc
{
  "schema_version": 1,
  "meta": {
    "url": "https://example.com/admin",
    "title": "Admin",
    "user_agent": "...",
    "viewport": { "w": 1440, "h": 900 },
    "started_ms": 1714534580000,
    "ended_ms":   1714534825000,
    "tool":       "annotator-bookmarklet/0.1"
  },
  "annotations": [
    {
      "id":           "a1",
      "ts_offset_ms": 12500,
      "tag":          "contrast",         // see TAGS below
      "comment":      "button label fails WCAG AA against the gradient",
      "element": {
        "outerHTML":     "<button class=\"cta\">Get Started</button>",
        "selector":      "main > section.hero > div > button.cta",
        "bbox":          { "x": 612, "y": 340, "w": 180, "h": 48 },
        "computed_style_diff": {        // only props differing from body
          "color":            "rgb(255,255,255)",
          "background-image": "linear-gradient(45deg, …)",
          "font-weight":      "600"
        },
        "aria": { "role": "button", "aria-label": null }
      }
    }
  ],
  "console_log": [
    { "ts_offset_ms":  3100, "level": "warn",
      "message": "Deprecated: foo() will be removed in v2.0" }
  ],
  "errors": [
    { "ts_offset_ms": 18900, "kind": "onerror",
      "message": "Cannot read property 'length' of undefined",
      "stack": "..." }
  ],
  "failed_requests": [
    { "ts_offset_ms": 4500, "url": "/api/recommendations",
      "status": 500, "duration_ms": 1240 }
  ],
  "csp_violations": [
    { "ts_offset_ms": 12000, "directive": "script-src",
      "blocked_uri": "https://untrusted.example.com/x.js" }
  ]
}
```

## Tags

A small fixed set so consumers can group:

| Tag | Use |
|---|---|
| `a11y` | accessibility — keyboard, screen-reader, ARIA |
| `contrast` | color contrast ratios (WCAG) |
| `alignment` | spacing, alignment, off-grid, overflow |
| `copy` | wording, tone, punctuation, clarity |
| `perf` | render performance, jank, slow load |
| `bug` | functional — button doesn't work, state corrupt |
| `suggestion` | net-new idea, not a defect |
| `other` | escape hatch |

## Capture-time mechanics

Bookmarklet shape:

  1. **DOM selection** — operator clicks "Pick" → mouse-move
     overlays a translucent highlight on the hovered element →
     click captures the element.
     - selector via a small heuristic (`tag.class:nth-child(n)`)
     - outerHTML truncated to 4 KB
     - computed style: read every prop, diff against `document.body`
       to keep only the ones that actually differ
     - bbox via `getBoundingClientRect()`
     - aria fields via `el.getAttributeNames().filter(starts-with-aria-)`

  2. **Console capture** — at panel-load time, monkey-patch
     `console.log/info/warn/error/debug`. Original methods invoked
     for normal console behavior; payloads also recorded with a
     timestamp delta from session start.

  3. **Error capture** — install `window.onerror` and
     `unhandledrejection` listeners. Both record `kind`, `message`,
     `stack`, and ts.

  4. **Network capture** — monkey-patch `window.fetch` to record
     non-2xx responses. PerformanceObserver subscribed to
     `resource` entries to catch failed XHRs and slow requests.

  5. **CSP capture** — `document.addEventListener('securitypolicyviolation', …)`.

  6. **Save** — operator clicks "Save" in the panel; the in-memory
     session payload is `JSON.stringify`d and downloaded as
     `annotator-{ts}.json`.

## Crawler integration sketch

PlausiDen-Crawler's existing journey JSON gains a step kind:

```jsonc
{
  "kind": "annotate",
  "label": "review-admin-flow",
  "panel_position": "bottom-right",     // optional
  "auto_capture_first_console_error": true  // optional — pause flag
}
```

Crawler at this step:
  - Loads `src/annotator.js` into the page via
    `page.add_script_tag({ content: file_string })`.
  - Pauses the journey loop (no auto-advance to next step).
  - Operator interacts with the page in the existing Playwright
    browser window.
  - When operator clicks "Save" in the panel, the panel calls
    `window.__crawler_emit_session(payload)` (Crawler exposes
    this binding). Crawler writes the payload into
    `findings/<journey>-<step-label>.json`.
  - Crawler resumes the journey.

## Tauri shell sketch

Wraps a webview. On window.create:
  - Load operator-supplied target URL.
  - Inject `src/annotator.js` after `DOMContentLoaded`.
  - Provide a native menu item "Save session..." that hits the
    panel's save action via `window.__annotator_save()`.

Same payload shape; different transport.

## Privacy + safety

- **No auto-upload.** A bookmarklet should never POST anywhere.
  The operator initiates upload (download first, transmit
  second). Tauri shell + Crawler can have explicit operator-
  configured destinations.
- **outerHTML truncation.** Capped at 4 KB per element to avoid
  pulling in giant SVGs / encoded images / pasted content.
- **No credentials.** The annotator reads the page; it doesn't
  read storage, cookies, or window.localStorage. (Future
  operator opt-in could change this; default off.)
- **Session lives in memory until Save.** Closing the tab loses
  the session — the operator MUST hit Save. Future improvement:
  IndexedDB persistence with operator-set retention.

## Versioning

`schema_version: 1` is committed. Additive changes (new optional
fields) keep version 1. Breaking changes bump to 2 and ship a
compat shim in the consumers.
