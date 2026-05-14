# PlausiDen-Annotator — vision document

> "If PlausiDen-Annotator was already built and did everything we
> wanted, what would this doc say?"

This is that doc. **[shipped]** works today. **[in-flight]** is
mid-build. **[queued]** has a task ID. **[concept]** has been
implied or requested and a developer should design it.

---

## 1. What PlausiDen-Annotator IS

**Operator-led UX audit + capture tool.** Browse any UI, click
elements, drop comments. Walk away with a typed JSON session that
records what you saw, what you said about it, and what the page
was doing in the background (DOM, console, network, errors).

Three deployable shapes share one captured-session JSON format:

1. **Bookmarklet** — paste a `javascript:…` URL into any browser;
   panel injects into the page being inspected. Firefox / Chrome
   / Safari / Tauri webviews / `file://` pages — all the same
   surface [shipped].
2. **PlausiDen-Crawler step** — a new `annotate` Journey step
   kind; Crawler drives Playwright through the journey, pauses
   at the annotate step, records the operator's session
   [in-flight].
3. **Tauri shell** — OS-native app that wraps a webview with the
   bookmarklet pre-injected [queued].

Captured sessions are JSON files the operator can re-open in the
viewer, hand to a Claude Code agent for analysis, post to a
PlausiDen-CRM ticket, or feed into PlausiDen-Audits finding
folders. Same wire format across all three shapes.

PlausiDen-Annotator is **not**:

- A bug tracker (PlausiDen-CRM owns ticketing)
- A vulnerability scanner (Vulnerability-Scanner-* crates own that)
- An automated tester (Crawler owns that — Annotator is operator-
  led, not script-led)
- A screen recorder (Loom-the-product is unrelated; PlausiDen-
  Annotator captures a structured session, not a video)
- A persistent backend service (zero-state by design)

PlausiDen-Annotator's contract: render the operator's annotation
panel into ANY browser context, let them pick elements + comment,
emit a typed Session JSON every consumer can read.

## The meta-mission: making AI-built UI reliable

Every PlausiDen tool — Loom, CMS, Forge, Crawler, Annotator —
exists for one common reason: **AI agents building GUI / frontend /
UX work need a reliability substrate that humans don't.**

Annotator's specific contribution: **closing the human-in-the-
loop gap.** An AI agent can render a UI, audit it via Forge,
verify it via Crawler — but the agent CANNOT see "this looks
weird," "this hierarchy is confusing," "Mom won't understand
this label." Only a human reviewer can. Annotator captures that
human review as STRUCTURED JSON the agent can parse:

- **Operator clicks an element**, drops a comment.
- **Captured session** carries: the CSS selector, a reduced DOM
  snapshot, the comment text, the console log, the recent
  network log, the screenshot.
- **Agent reads the session JSON**, knows EXACTLY which element
  the human flagged + WHY + WHAT the runtime state was.
- **Agent fixes**, re-renders, asks for re-review, iterates.

Without Annotator: humans say "the layout's off" in Slack, the
agent guesses what they meant, the iteration loop is slow and
lossy.

With Annotator: every human concern is a typed Finding the agent
can act on programmatically.

## 2. The supersociety stack PlausiDen-Annotator uses

- **Zero-backend by design** — bookmarklet + JSON file. No
  server, no database, no SaaS dependency. Operator's data never
  leaves their machine unless they explicitly attach the JSON
  somewhere.
- **Stable typed `Session` JSON schema** — versioned. Every
  consumer (CRM ticket, Audits folder, Crawler journey, Claude
  agent) reads the same shape.
- **Works in any browser** — bookmarklet has no
  framework dependency, no module loader, no build chain.
  Vanilla JS.
- **Captures real runtime state** — console, network, DOM
  snapshot at the time of annotation. The agent reading the
  session sees what the operator saw.
- **CSP-friendly** — the bookmarklet runs in the page's own
  origin; no third-party requests. If the page has a strict CSP,
  the bookmarklet still works because it doesn't fetch external
  resources.
- **Privacy-respecting** — captured DOM snapshot is reduced /
  truncated; no full-page screenshots leak via third parties.
  Operator decides where the JSON goes.
- **No telemetry** — the bookmarklet does not phone home.

## 3. Personas

### 3.1 Mom — non-technical client

Mom uses Annotator without knowing she's using it.

- Mom clicks "Get help with this page" in her Loom editor.
- Loom injects the Annotator bookmarklet under the hood.
- Mom clicks the wonky element on her published site, types
  "This button looks broken."
- The session JSON gets attached to her support request OR
  posted directly to the agent operating on her site.
- The agent reads the session, knows exactly which element +
  what the runtime state was, fixes it, asks Mom to re-review.

What Mom never has to think about: CSS selectors, network
panels, console logs, DOM snapshots — the panel hides all that
behind one "drop a comment" interaction.

### 3.2 The technical client — wants control

What they get today:

- **Bookmarklet works on any browser** — no install, no
  permissions request.
- **Session JSON is portable** — they can hand it to any review
  tool, Slack channel, ticket system.
- **Captures DOM snapshot** at the time of annotation so the
  comment is anchored even if the page changes later.

What they get next:

- **Crawler journey integration** — record an annotation pass
  during a scripted journey, surfacing exactly when in the flow
  the issue manifested [in-flight].
- **Tauri shell** for OS-native usage with persistent windows
  + tabs [queued].
- **Multi-comment threading** per element — collaborative review
  on one selector [concept].
- **Session diff** — compare two annotation sessions of the same
  page to see how feedback evolved [concept].

### 3.3 The developer — contributor or forker

What they get today:

- **Bookmarklet source under `src/`** — vanilla JS, easy to
  fork.
- **Stable Session JSON schema in `examples/sample-session.json`**
  — every consumer reads it.
- **`docs/ARCHITECTURE.md`** explains the design.
- **Examples folder** with a sample session.

What developers want next:

- **Typed Rust crate (`annotator-session`)** with the Session
  schema as serde structs so Rust consumers can deserialize
  type-safely [queued].
- **MCP server exposing Annotator capabilities** (open-session,
  read-session, list-flagged-elements) [concept].
- **Headless annotator** — automated synthesis of an annotation
  session from a Crawler finding (i.e., "the placeholder-text
  detector fired here, with this DOM context") [concept].
- **Diff renderer** — given two session JSONs of the same page,
  render the comment delta [concept].
- **Browser-extension shape** in addition to bookmarklet for
  always-on annotation mode [concept].
- **WebExtension manifest v3 port** of the bookmarklet
  [concept].
- **VSCode extension** that lets a developer open a session JSON
  from their editor [concept].

### 3.4 Claude Code (and other autonomous agents)

What an agent gets today:

- **Stable Session JSON schema** — read/write/parse with a typed
  contract.
- **Per-session typed array of comments**, each anchored to a
  CSS selector.
- **Reduced DOM snapshot** so the agent can resolve the selector
  to a specific element even if the live page has changed.
- **Console log + network log** captured at annotation time,
  same JSON structure as a Crawler `Report`.

What agents want next:

- **MCP server** exposing every Annotator capability [concept].
- **`annotator-replay`** — feed a session JSON to the agent;
  the agent walks every flagged element, proposes fixes,
  generates a new annotation session with proposed changes
  [concept].
- **Session-driven Crawler test generation** — given an operator
  flagged "this contact form doesn't submit," auto-generate a
  Crawler journey that reproduces the flow [concept].
- **CRM ticket auto-creation** — when a session lands with N
  strict-severity comments, file a CRM ticket per comment
  [concept].
- **Multi-agent review consensus** — N agents independently
  annotate a target; consensus comments are highest-priority
  [concept].
- **Annotation budget** per agent session [concept].

## 4. Capability map

### 4.1 Capture surface

| Capability | Status |
|---|---|
| Bookmarklet panel injection (vanilla JS) | shipped |
| Element pick (click to select) | shipped |
| Comment text per element | shipped |
| Console log capture | shipped |
| Error capture (`window.onerror` + unhandledrejection) | shipped |
| Network log capture | shipped |
| DOM snapshot per annotation | shipped (reduced) |
| Full DOM snapshot (opt-in, for forensic review) | concept |
| Per-step screenshot (Tauri shell or extension) | queued |
| Audio note attachment per comment | concept |
| Voice-to-comment dictation (on-device Whisper) | concept |

### 4.2 Output

| Capability | Status |
|---|---|
| Typed `Session` JSON file download | shipped |
| Typed `Session` JSON schema documented + versioned | shipped |
| Sample session (`examples/sample-session.json`) | shipped |
| Rust `annotator-session` crate (serde-typed schema) | queued |
| TypeScript `@plausiden/annotator-session` package | concept |
| Markdown report from a session | concept |
| HTML report (re-renders the annotated page with overlay) | concept |
| Diff renderer (two sessions of same page) | concept |

### 4.3 Deployment shapes

| Capability | Status |
|---|---|
| Bookmarklet (any browser) | shipped |
| Crawler journey `annotate` step kind | in-flight |
| Tauri shell (OS-native app) | queued |
| Browser-extension (Chromium / Firefox MV3) | concept |
| VSCode extension (open / view session) | concept |
| Self-hostable web viewer for sessions | concept |

### 4.4 Sister-tool integration

| Capability | Status |
|---|---|
| Loom — "get help with this page" launches Annotator | concept |
| CMS — annotation session lands in audit log | concept |
| Forge — `phase_annotation_review` consumes sessions as findings | concept |
| Crawler — `annotate` step kind + auto-record session per finding | in-flight |
| CRM — file a ticket per strict comment | concept |
| Audits — drop session into findings folder for AVP-2 review | concept |
| Salesman — capture a competitor-site annotation for sales context | concept |

### 4.5 Privacy + opsec

| Capability | Status |
|---|---|
| Zero-backend (no server / no SaaS) | shipped (by design) |
| No telemetry | shipped |
| CSP-friendly (no third-party fetches) | shipped |
| Reduced DOM snapshot (truncate sensitive content) | shipped |
| Operator-controlled session destination | shipped |
| Per-session encryption (operator's key) | concept |
| Tor-friendly bookmarklet (works on `.onion` sites) | concept |
| Hardware-key signing of every comment (forensic attribution) | concept |

### 4.6 Documentation

| Capability | Status |
|---|---|
| README with quick-use steps | shipped |
| `docs/ARCHITECTURE.md` | shipped |
| `docs/ANNOTATOR_VISION.md` (this doc) | shipped (T72) |
| Sample session for every consumer | shipped |
| Per-shape integration guide (bookmarklet / crawler / tauri) | partial |
| Per-consumer integration guide (CMS / Forge / Crawler / CRM / Audits) | concept |

## 5. Architecture (when fully built)

```
┌──────────────────── PlausiDen-Annotator ────────────────────┐
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  src/ (vanilla JS — annotator panel + capture)       │  │
│  │  - element pick                                      │  │
│  │  - comment UI                                        │  │
│  │  - console / error / network capture                 │  │
│  │  - reduced DOM snapshot                              │  │
│  │  - Session JSON serialiser                           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────┐  ┌────────────────┐ ┌────────────────┐ │
│  │  Bookmarklet   │  │  Crawler step  │ │  Tauri shell   │ │
│  │  shape         │  │  shape         │ │  shape         │ │
│  │  (any browser, │  │  (scripted     │ │  (OS-native    │ │
│  │   javascript:) │  │   journey      │ │   app + tabs)  │ │
│  │                │  │   integration) │ │                │ │
│  └────────────────┘  └────────────────┘ └────────────────┘ │
│                          ▼                                   │
│                  Session JSON                                │
└──────────────────────────────────────────────────────────────┘
       │                   │              │             │
       ▼                   ▼              ▼             ▼
   Loom editor         Forge phase   Crawler test   CRM ticket
   ┌──────────┐       ┌──────────┐  ┌──────────┐   ┌──────────┐
   │ "get     │       │ phase_   │  │ annotate │   │ file per │
   │ help"    │       │ annot…   │  │ + record │   │ strict   │
   │ launches │       │ consumes │  │ session  │   │ comment  │
   │ panel    │       │ as       │  │          │   │          │
   │          │       │ findings │  │          │   │          │
   └──────────┘       └──────────┘  └──────────┘   └──────────┘
```

Multi-tool consumer mesh:

```
┌────── Annotator session (the single source of truth) ──────┐
│                                                              │
│  Captured by ───▶  Operator (bookmarklet/Tauri/Crawler)     │
│                                                              │
│  Consumed by ───▶  - Loom editor (round-trip review)        │
│                    - Forge phase_annotation_review           │
│                    - Crawler auto-test-generation            │
│                    - CRM ticket per strict comment           │
│                    - Audits folder for AVP-2 trail           │
│                    - Salesman dossier (competitor context)   │
│                    - Claude agent (replay, fix, iterate)     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 6. Roadmap from now to "done"

### Sprint 1 — close the bookmarklet → MVP gap

- Crawler journey `annotate` step kind (the in-flight piece)
- Rust `annotator-session` crate (typed schema for Rust consumers)
- TypeScript `@plausiden/annotator-session` package for TS consumers
- Markdown report renderer from a session
- HTML report renderer (re-renders the annotated page with overlay)
- Per-session encryption (operator's key)

### Sprint 2 — sister-tool integrations

- Loom "get help" button → launches Annotator panel
- CMS audit-log entry per saved session
- Forge `phase_annotation_review` consumes sessions as findings
- CRM auto-ticket per strict comment
- Audits folder integration

### Sprint 3 — agent-facing surface

- MCP server exposing Annotator capabilities
- `annotator-replay` — feed a session JSON, agent walks flagged
  elements, proposes fixes
- Session-driven Crawler test generation (auto-journey from
  flagged form)
- Multi-agent review consensus (N agents annotate independently)
- Annotation budget per agent session

### Sprint 4 — alternative shapes

- Tauri shell (OS-native app)
- Browser-extension (Chromium / Firefox MV3)
- VSCode extension (open / view session)
- Self-hostable web viewer

### Sprint 5+ — the supersociety horizon

**For Mom (non-technical client):**
- Voice-to-comment dictation (on-device Whisper)
- Audio-note attachment per comment
- One-button "send this to the developer" (encrypted DM)
- Auto-translation of comments across languages

**For the technical client:**
- Multi-comment threading per element
- Session diff renderer (compare two passes)
- Heatmap overlay (frequently-flagged elements)
- A/B annotation comparison

**For the developer:**
- Differential session replay (run agent fix → re-annotate →
  diff against original)
- Property-based fuzz on the DOM-reducer (no DOM input panics)
- Type-state Session schema (compile-time guarantees on
  required fields)
- TLA+ specification of the annotation state machine

**For Claude Code (and other autonomous agents):**
- MCP server with full toolset
- Agent annotation as a first-class shape (agent generates a
  session, human reviews it)
- Federated annotation consensus (N agents → consensus comments)
- Annotator-as-a-feedback-loop (agent edits → annotator
  re-reviews → agent edits again, until convergence)

**Cross-cutting supersociety:**
- Hardware-key signing of every comment (forensic attribution)
- Tor-friendly bookmarklet (works on `.onion` sites)
- C2PA-style provenance for every captured DOM snapshot
- Zero-knowledge proof that a comment was authored without
  revealing the author (for adversarial-tester roles)
- Federation across PlausiDen-Annotator instances (one
  operator's session can be replayed in another's installation
  with cryptographic continuity)

## 7. Future shape — three years out

PlausiDen-Annotator becomes the universal human-review surface
for AI-built UI work. Every PlausiDen-served site has a "review
this" affordance that opens the Annotator panel. Every captured
session is a typed JSON the agent can act on, the CMS can store
in its audit log, the Forge can audit, the Crawler can replay.

The bookmarklet stays the lowest-friction entry point. The
Tauri shell becomes the OS-native operator workstation. The
browser-extension becomes the always-on review mode. All three
emit the SAME Session JSON.

Agents don't replace human review — they speed up the iteration
loop. A human flags 5 issues in 30 seconds via Annotator. The
agent fixes all 5 in 2 minutes. The human re-reviews in 30
seconds. Convergence in <5 minutes for what would be a
half-hour Slack conversation today.

Mom-class clients get the simplest possible review affordance:
"click on what looks wrong, type why." The Sacred.Vote-class
technical client gets the full multi-shape multi-consumer mesh
— annotation flowing from Tauri shell to CMS audit log to Forge
phase to Crawler test generation. Both get the same supersociety
guarantees: zero telemetry, zero backend, operator-owned data,
forensic attribution where signed.

## 8. Acceptance criteria for "done"

PlausiDen-Annotator is **done** when:

1. Mom can flag a wonky element with one click + one comment,
   without ever knowing the words "CSS selector" or "DOM
   snapshot."
2. Every captured Session JSON is consumable by every PlausiDen
   tool in the mesh — the schema is the universal contract.
3. The bookmarklet works in Firefox, Chrome, Safari, Tauri
   webviews, and `file://` pages with zero modification.
4. An agent can read a Session JSON and propose a fix for every
   flagged element, with confidence the selector still resolves
   (because of the captured DOM snapshot).
5. Every agent fix can trigger a re-annotation pass; the loop
   converges in <5 minutes for typical UI iteration.
6. Sessions are operator-owned end-to-end — never sent to a
   third party without explicit operator action.
7. Hardware-key-signed comments are the default for forensic
   roles; unsigned comments stay valid for casual review.
8. The Crawler `annotate` step kind is first-class; scripted
   journeys can pause for operator review at any step.
9. The threat model from `~/.claude/CLAUDE.md` (state-actor
   adversary, full breach, unlimited time) holds against the
   captured-session surface — no captured DOM / network / console
   leaks beyond the operator-chosen destination.

The verdict is always **STILL BROKEN** — shipping is risk
acceptance, not a declaration of correctness. The loop resumes
on the next commit.
