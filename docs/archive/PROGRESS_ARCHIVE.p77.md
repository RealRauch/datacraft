# PROGRESS_ARCHIVE.p77.md — Shard (Phase 77, append-only)

> Created 2026-11-14T15:05 — home for the detail block of Phase 77 on completion.
> Contains: Phase 77 — Access form/report migration: feasibility and decision. Header is write-once — never edited.

---

### Phase 77 — Access form/report migration: feasibility and decision

**Scope (Steps):**

- **77.1 Feasibility: jetdb against the committed fixture and the SaveAsText exports**
- **77.2 Route decision: own decoder with jetdb as oracle, or a jetdb-linked migration tool**
- **77.3 Freeze the supported control/property subset and the fixture pair**
- **77.4 Documentation and phase gate**

**Goal:** Answer M88's feasibility question with evidence and a binding route
decision, so the implementation phase can be packaged without speculation.
Native form/report definitions have no official Microsoft specification; the
2026-11-12 M89 study stopped at the property-id table. The 2026-11-14
research found `jetdb` (pure Rust, MIT OR Apache-2.0; CLI/library), which
reads Jet4/ACE form/report designs without Access and publishes the
id/control tables it derived against Access's SaveAsText output. This phase
does not ship the importer: it verifies the claim on our own fixture, chooses
the clean-room route, and freezes the subset and the fixture pair.

- **77.1 — jetdb feasibility.** Pin a `jetdb-cli` version and run it
  read-only against `fixtures/programmability.accdb`; compare the form/report
  properties (`Left`/`Top`/`Width`/`Height`, `Caption`, `ControlSource`, the
  control types) with the committed SaveAsText exports under
  `tools/fixture-build/objects/{forms,reports}`. Record coverage and every
  mismatch or unnamed id. "Works", "partial" and "stop" are all legitimate
  outcomes and are documented like M89. No product code is touched; the tool
  is a test-time oracle only (the libmdb pattern, L31).
- **77.2 — route decision (High gate).** Choose between (a) our own decoder
  from own black-box observation, with jetdb as the test-time oracle, our own
  generated property-id fixtures and the SaveAsText exports as ground truth,
  and (b) a separate migration tool that links `jetdb` (pure Rust, no
  FFI/COM, license-compatible) and maps its `FormProperties` onto
  `FormDef`/`ReportDef`. Route (a) keeps the clean-room claim untouched;
  route (b) is faster and needs the explicit clean-room release plus a
  `THIRD-PARTY-NOTICES.md` entry. The decision lands as a dated decision
  record with the effort estimate.
- **77.3 — subset and fixture pair.** Freeze the supported control kinds
  (label, text box, check box, combo, command button, subform) and the
  property set (rects, caption, control source; events/VBA stay unbound per
  ADR-2), define the unsupported-list format, and pin a fixture pair (source
  export or database + expected sidecar JSON) for the chosen route. New
  fixtures are Medium-class.
- **77.4 — documentation and gate.** Update M88 (steps, verdict), the
  FORMAT-JET M89 addendum, README/ROADMAP where the migration route becomes
  user-facing, and CHANGELOG only if user-visible behaviour lands (it should
  not in this phase). Gate: canonical verification set where code or fixtures
  changed, file-size gate, `docs_validate` clean, CI green.

Content classes: the feasibility run is read-only research (no repo change);
decision records and documentation are Low; the frozen fixture pair is Medium
(new fixtures). The phase deliberately ends before implementation — the
decoder or adapter is packaged as the next phase from the decision record.

**Acceptance:** 77.1 produces a written, reproducible comparison of jetdb's
form/report output against the committed SaveAsText exports for the fixture,
naming every covered and missing property/control; 77.2 records a binding
route decision with the clean-room gate and an effort estimate (or a
documented stop); 77.3 freezes the subset and the fixture pair; the canonical
set is green where code or fixtures changed, `docs_validate` is clean and the
phase's commits are CI-green. No Access installation is required for any of
it.

**Verification:** the feasibility run is reproducible from the pinned
`jetdb-cli` version and the committed fixture; `docs_validate` clean; the
canonical set (fmt, clippy `-D warnings`, workspace tests, lcov ≥ 95 %),
file-size gate and CI green — for a docs-only outcome the canonical set is
still run and reported, with "no code changed" as the evidence.
**Verification:** Phase gate: docs_validate clean, fmt/clippy -D warnings/workspace tests green with no code change (the phase-76 lcov export 95.575% still describes the identical tree), file-size gate unchanged. ADR-9, ROADMAP §20 (Route A fallback), FORMAT-JET feasibility report, M88-IMPORT contract and the M88 backlog update are in place. Phase closes as a decision phase; Route B implementation is Phase 78.
