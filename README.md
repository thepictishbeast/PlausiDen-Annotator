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

A small zero-backend tool for the operator-led UI/UX audit loop. Three
deployable shapes share one captured-session JSON format:

1. **Bookmarklet** — paste a `javascript:…` URL into any browser; the
   annotator panel injects into the page being inspected. Works in
   Firefox / Chrome / Safari / Tauri webviews / file:// pages alike.
2. **PlausiDen-Crawler step** — a new `annotate` step kind for
   Crawler journeys. Crawler drives Playwright through the journey,
   pauses at the step, records an annotation session.
3. **Tauri shell** — wraps a webview with the bookmarklet pre-injected
   so the operator gets an OS-native annotator app.

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

## Quick use (bookmarklet)

```
1. Open the page you want to audit.
2. Paste the contents of `dist/bookmarklet.js` into a browser
   bookmark URL prefixed with `javascript:`.
3. Click the bookmark on the target page.
4. The annotator panel appears bottom-right.
5. Click "Pick" → click any element on the page → add a comment.
6. Click "Save" → a `.json` file downloads with the captured session.
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
- **No build step for the bookmarklet shape.** `src/annotator.js` is
  a single self-contained file. `dist/bookmarklet.js` is the
  minified one-liner produced by a tiny shell script — no
  rollup / webpack / parcel dependency tree.
- **Same captured-session shape across all three deployable shapes.**
  See `examples/sample-session.json`.

## Sibling integrations

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
