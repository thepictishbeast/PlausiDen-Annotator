> # ⚠️ DO NOT USE — UNVERIFIED — UNSAFE ⚠️
>
> This software is **unverified and unsafe for any production use**.
> It is published publicly only for transparency, third-party audit,
> and reproducibility. Treat every commit as guilty until proven
> innocent.
>
> By using this code you accept:
> - **No warranty** of any kind, express or implied.
> - **No fitness** for any particular purpose.
> - **No guarantee** of correctness, safety, or freedom from defects.
> - **Zero liability** on the maintainer for any damages — data loss,
>   security compromise, financial loss, or any consequential damages.
>
> The code is under active engineering development per the
> [Adversarial Validation Protocol v2](https://github.com/thepictishbeast/PlausiDen-AVP-Doctrine/blob/main/AVP2_PROTOCOL.md).
> Every commit's default verdict is **STILL BROKEN**. AVP-2 requires
> a minimum of 36 verification passes before a `SHIP-DECISION:`
> annotation may be considered. **No commit in this repository has
> reached `SHIP-DECISION:` status.**

<!-- repo-label: tooling -->
<!-- repo-class: ux-inspection-and-capture -->
<!-- repo-status: experimental -->

# PlausiDen-Annotator

> Browse software UI. Click elements. Comment. Walk away with a JSON
> session that captures what you saw, what you said about it, and
> what the page was doing in the background.

A small backend-light tool for the operator-led UI/UX audit loop.
Four deployable shapes share one captured-session JSON format:

1. **Bookmarklet** — paste a `javascript:…` URL into any browser; the
   annotator panel injects into the page being inspected. Works in
   Firefox / Chrome / Safari / Tauri webviews / file:// pages alike.
   Zero backend; sessions land as a JSON download.
2. **PlausiDen-Crawler step** — a new `annotate` step kind for
   Crawler journeys. Crawler drives Playwright through the journey,
   pauses at the step, records an annotation session.
3. **Tauri shell** — wraps a webview with the bookmarklet pre-injected
   so the operator gets an OS-native annotator app.
4. **annotator-relay** — optional local axum HTTP daemon
   (`crates/annotator-relay/`) that the bookmarklet can POST sessions
   to instead of downloading a JSON file. Stores sessions on disk +
   exposes `GET /sessions` (list) + `GET /sessions/<id>` (fetch)
   endpoints so downstream tools (Forge `phase_annotation_review`,
   PlausiDen-Audits, CRM, AI agents) can stream-consume sessions
   without manual upload. Designed to be Tauri-shelled eventually;
   runs as a standalone process today.

Captured sessions are JSON files the operator can re-open in the
viewer, hand to Claude Code for analysis, post to a
[PlausiDen-CRM](https://github.com/thepictishbeast/PlausiDen-CRM)
ticket, or feed into [PlausiDen-Audits](https://github.com/thepictishbeast/PlausiDen-Audits)
findings folders.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the design.
See [`examples/sample-session.json`](examples/sample-session.json) for
the captured-session shape every renderer reads.

## Status

`experimental` — bookmarklet shape works end-to-end (DOM selection +
console/error/network capture + comment UI + JSON export). Crawler
integration + Tauri shell are TODO.

## Quick use (bookmarklet, zero-backend)

```
1. Open the page you want to audit.
2. Copy the entire contents of `dist/bookmarklet.js` (the file
   already starts with `javascript:` — paste verbatim into a
   browser bookmark URL field).
3. Click the bookmark on the target page.
4. The annotator panel appears bottom-right.
5. Click "Pick" → click any element on the page → add a comment.
6. Click "Save" → a `.json` file downloads with the captured session.
```

### Building the bookmarklet

`dist/bookmarklet.js` is generated from `src/annotator.js` (the
single self-contained IIFE) via a small stdlib-only Python
script. **No build dependencies** — no rollup, no webpack, no
parcel, no Node. Just:

```sh
python3 tools/build-bookmarklet.py
# → built dist/bookmarklet.js — N bytes (X% of 32 KB budget)
```

The script:
- Strips standalone `//` line comments + collapses blank lines
- URL-encodes per RFC 3986 unreserved
- Prefixes `javascript:`
- Warns + exits non-zero if size exceeds 32 KB (modern-browser
  URL-bar budget floor)
- Idempotent: same input → byte-identical output

The committed `dist/bookmarklet.js` is the canonical build —
re-run the script after any `src/annotator.js` change + commit
both together.

## Quick use (annotator-relay, networked)

```sh
# Start the relay (default port 9234, data dir ./annotator-data/)
cargo run --release -p annotator-relay -- \
    --bind 127.0.0.1:9234 \
    --data-dir ./annotator-data
```

Then in another terminal or your browser:

```sh
# POST a session (the bookmarklet does this when configured with a relay URL)
curl -X POST http://127.0.0.1:9234/sessions \
    -H 'content-type: application/json' \
    --data @examples/sample-session.json

# List stored sessions
curl http://127.0.0.1:9234/sessions

# Fetch one by id
curl http://127.0.0.1:9234/sessions/<id>
```

The relay validates every POST against the captured-session schema
(matches `examples/sample-session.json`). Malformed sessions return
`400 Bad Request` with a `serde_json` parse error in the body. Each
stored session is persisted as `<data-dir>/<id>.json` (mode 600).

**Bookmarklet → relay path**: configure the bookmarklet build to
include `--relay-url http://127.0.0.1:9234` so the "Save" action
POSTs the session instead of triggering a file download. Falls back
to download on `fetch` failure (offline operator, relay not running,
CORS error).

### End-to-end test

`crates/annotator-relay/tests/e2e_round_trip.rs` exercises the full
POST → list → fetch round-trip + 404/400 negative paths against an
in-process axum instance. Run via:

```sh
cargo test -p annotator-relay --test e2e_round_trip
```

## What gets captured per session

- Each annotation:
  - operator-typed comment + tag (a11y / contrast / alignment / copy /
    perf / bug / suggestion / other)
  - selected element's outerHTML (truncated to 4 KB)
  - computed-style diff vs the document body (only the props that differ)
  - bounding box (x / y / w / h)
  - accessibility tree node (aria-* attributes + role)
  - timestamp (ms since session start)
- Whole-session log (auto):
  - every console.{log, info, warn, error, debug}
  - every window.onerror + unhandledrejection
  - every failed fetch (non-2xx, aborted, refused) via fetch monkey-patch
  - every CSP violation (SecurityPolicyViolationEvent)

## Design constraints

- **Zero backend by default.** A bookmarklet should not need a server.
  Sessions land as a JSON download; persistence is the operator's
  filesystem.
- **Privacy-respectful.** Don't auto-post anywhere. Operator
  initiates the upload step explicitly.
- **No build-dependency-tree for the bookmarklet.** `src/annotator.js`
  is a single self-contained IIFE. `dist/bookmarklet.js` is the
  URL-encoded `javascript:` form produced by
  `tools/build-bookmarklet.py` — stdlib Python only, no rollup
  / webpack / parcel / Node dependency tree. The committed
  `dist/bookmarklet.js` is the canonical build; re-run the script
  after any `src/annotator.js` change.
- **Same captured-session shape across all three deployable shapes.**
  See `examples/sample-session.json`.

## Sibling integrations

- **PlausiDen-Forge** *(planned: `phase_annotation_review`)*: a
  Forge phase can poll the annotator-relay for sessions flagged
  against the current build, surface findings as `Severity::Warn`
  or `Strict` rows in the BuildReport. Closes the human-in-the-
  loop gap between Forge audit (machine) and Annotator capture
  (human).
- **PlausiDen-Crawler**: the captured-session JSON is consumed by
  Crawler's `findings/` output format with one transform step.
- **PlausiDen-Audits**: a session that flagged a real issue can be
  attached to an `audits/<slug>/findings/` entry with the
  `annotated-by-operator` tag.
- **PlausiDen-AI (LFI)**: the captured-session JSON can be ingested
  via the `lfi_api` `/v1/meta-learning/measurements` endpoint as a
  measurement record set (one per annotation; concept_label =
  `annotation:{tag}:{element_summary}`).
- **PlausiDen-CRM**: a session can be turned into a ticket with one
  curl call (TODO when CRM ingest endpoint exists).

## License

Proprietary, all rights reserved (`LICENSE` to be added; pending
operator decision on whether to open-source the bookmarklet).
