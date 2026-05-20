# TOOLS.md — PlausiDen-Annotator

Canonical command index. Annotator layers typed annotations on Crawler runtime output.

> Cross-repo TOOLS reference: see [../PlausiDen-Forge/TOOLS.md](../PlausiDen-Forge/TOOLS.md).

---

## Annotation lifecycle (from this repo + Forge consumer)

```
forge audit annotation_review        Surface operator-flagged annotations as findings
                                     (per task #13: Annotator↔Forge integration)

cargo build --workspace              Build the annotator workspace
cargo test --workspace               Run annotator tests
```

---

## See also

- `AGENTS.md` — repo orientation
- `../PlausiDen-Crawler/AGENTS.md` — upstream of annotation targets
- `../PlausiDen-Forge/crates/forge-phases/src/annotation_review.rs` — downstream consumer
