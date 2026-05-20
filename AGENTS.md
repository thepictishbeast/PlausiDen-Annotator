# AGENTS.md — PlausiDen-Annotator

Orientation for any AI agent working in this repository. Read **before** writing code.

> Cross-repo orientation: see [../PlausiDen-Forge/PLAUSIDEN_ECOSYSTEM.md](../PlausiDen-Forge/PLAUSIDEN_ECOSYSTEM.md).

---

## RULE 0 — Annotations are typed projections, not free-form notes

PlausiDen-Annotator layers structured annotations on Crawler runtime output. Annotations are typed: they reference specific elements, journey steps, or finding ids; they carry severity + category + cross-references.

**Forbidden:** free-form annotation strings without typed metadata, annotations that don't reference a concrete artifact (a journey step / a Finding id / a primitive variant), annotations that bypass the Annotator's typed surface.

**Canonical defaults:**
- Typed `Annotation` shape: target_kind / target_id / severity / category / author_kind (human / lfi-augmented / llm-augmented) / created_at (RFC 3339).
- Cross-references resolve to known artifacts (Crawler journey steps, Forge Findings, Loom primitives).
- Annotations that fail typed validation are rejected at parse, not silently coerced.

---

## RULE 1 — Look before you build

1. Check existing annotation primitives in the crate before adding a new one.
2. Cross-reference Crawler journey output shapes; Annotator consumes those.
3. Annotator's downstream consumers (operator UI, audit reports) read typed JSON — verify any new shape lands cleanly.

---

## Cross-references

- `PlausiDen-Forge/AGENTS.md` — Forge-side companion (`annotation_review` phase)
- `PlausiDen-Crawler/AGENTS.md` — Crawler-side companion (consumed for journey-step targets)
- `PlausiDen-AVP-Doctrine/N_ORIENTATION_SUBSTRATE.md` — Audience orientation drives annotation visibility per consumer

---

## First steps

1. Read the upstream Crawler shape that the annotation targets.
2. Read the downstream consumer (operator UI / audit phase) that reads the annotation.
3. State the goal in one sentence; match the typed pattern.
