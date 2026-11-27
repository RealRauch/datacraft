# BACKLOG_ARCHIVE.p103.md — Shard (Phase 103, append-only)

> Created 2026-11-27T16:18 — home for item blocks archived in the context of Phase 103 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 103. Header is write-once — never edited.

---

### [x] F19 — Inline fix: the T12 allowlist kept the pre-split report_design.rs keys — 🟢
- **Location:** `crates/datacraft-ui/src/i18n/bypass.rs` (`ALLOWED`).
- **Problem:** The phase-102 L30 split moved the report preview's page navigation into `report_design/preview.rs`; the two painted-glyph allowlist entries kept the pre-split `report_design.rs` keys, so the paint-site scan failed on main (preview.rs:38/49 flagged).
- **Fix:** the two `ALLOWED` entries now name `report_design/preview.rs` (commit `f88edf5`; the label corrected to F19 in `789bdff` — f88edf5's message mislabeled the id as F13, taken by a shard).
- **Found:** while running the canonical verification set for step 103.1 (2026-11-27).
- **Done:** fixed inline in `f88edf5` (label correction `789bdff`); found during the step-103.1 verification run 2026-11-27

---

### [x] M134 — Migration readiness report and benchmark instrument (phase 103, roadmap §14/§18) — 🟡
- **Location:** `crates/datacraft-core/src/vba_analysis.rs` (precedent), a new readiness module, UI report surface, `tools/dao-oracle/`, `docs/BENCHMARKS.md`.
- **Problem:** Roadmap §14 freezes an app-level migration readiness report that aggregates per-object evidence, and §18 freezes a benchmark methodology/harness — neither exists, so migration effort and performance claims have no instrument.
- **Approach:** One read-only `AnalyzeApp` command aggregating ring coverage per form/report/query, VBA module/transpilation estimate, the phase-100 link inventory, unsupported features and an effort summary; a UI/wire surface; a benchmark harness in `tools/dao-oracle` plus `docs/BENCHMARKS.md` (generator recipe, method controls, measured results, honest gaps).
- **Acceptance:** The report lists every object with its ring status and unsupported reasons and is read-only by construction; all four clients can request it; the benchmark harness produces family-A numbers against DAO and the documented method, with results and the remaining gaps recorded in `docs/BENCHMARKS.md`; the phase-103 method retrospective (PLAYBOOK §8) rides the gate.
- **Done:** completed in phase 103 (RED `a767f8a`/`658d41e`, feat `04c820d`/`98d3510`/`4897b05`, bench `3621c82`, docs `aeb4b51`); the retrospective rode the gate (AGENTS §11, next checkpoint phase 108)

## ✅ Done Index

> One-liner per completed item (details above).

- F19 — Inline fix: the T12 allowlist kept the pre-split report_design.rs keys — done (fixed inline in `f88edf5` (label correction `789bdff`); found during the step-103.1 verification run 2026-11-27)
- M134 — Migration readiness report and benchmark instrument (phase 103, roadmap §14/§18) — done (completed in phase 103 (RED `a767f8a`/`658d41e`, feat `04c820d`/`98d3510`/`4897b05`, bench `3621c82`, docs `aeb4b51`); the retrospective rode the gate (AGENTS §11, next checkpoint phase 108))
