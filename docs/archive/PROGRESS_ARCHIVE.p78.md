# PROGRESS_ARCHIVE.p78.md — Shard (Phase 78, append-only)

> Created 2026-11-14T15:29 — home for the detail block of Phase 78 on completion.
> Contains: Phase 78 — datacraft-import: the Route B migration tool. Header is write-once — never edited.

---

### Phase 78 — datacraft-import: the Route B migration tool

**Scope (Steps):**

- **78.1 Import library: mapping, defaults, unsupported report and the contract test**
- **78.2 CLI and sidecar merge: arguments, atomic save, exit codes, subprocess E2E**
- **78.3 Six-kind and defaults fixture via the Access COM generator**
- **78.4 Documentation, packaging and phase gate**

**Goal:** Ship Route B (ADR-9): a pure-Rust `datacraft-import` library and
binary that reads native Jet4/ACE forms/reports through pinned `jetdb` 0.4
and merges the supported subset into the `.datacraft` sidecar, with a
precise unsupported report. The engine stays clean-room; the tool is the
declared adapter. The binding mapping contract is `docs/M88-IMPORT.md` —
the steps implement it unchanged.

- **78.1 — import library and contract test.** New workspace crate
  `datacraft-import` (library + binary target) on `datacraft-core` and
  `jetdb = "0.4.0"`: open the file read-only, list forms/reports, map the
  six control kinds and their properties, convert twips → points, apply the
  confirmed default table, read 2-byte values unsigned, and collect the
  per-object unsupported/ignored report. The RED test is the frozen
  expected sidecar for `fixtures/programmability.accdb` (four objects,
  exact JSON), written first; GREEN completes the mapping. New workspace
  member + dependency = Medium (released 2026-11-14 with Route B).
- **78.2 — CLI and sidecar merge.** `datacraft-import <database>
  [--sidecar <path>] [--strict] [--json]`: merge semantics (replace
  imported forms/reports by case-insensitive name, keep everything else),
  atomic sidecar save, exit codes 0/1/2, human and JSON reports. Real-
  subprocess E2E on temp paths; the database is never written.
- **78.3 — six-kind/defaults fixture.** Extend `tools/fixture-build` with
  an Access COM generator (the step-37.3 recipe) that creates a fresh
  database with one form/report per supported control kind, distinctive
  property values, and a defaults object with untouched controls; export
  SaveAsText, build a **new** fixture (never the committed programmability
  file), pin its manifest hash and provenance in `docs/FIXTURE-SOURCES.md`,
  and extend the contract tests to CheckBox, ComboBox, SubForm and the
  remaining default values. New fixture = Medium.
- **78.4 — documentation, packaging and gate.** README migration guide,
  `THIRD-PARTY-NOTICES.md` (jetdb 0.4, MIT/Apache), AGENTS §1 crate graph,
  PUBLISHING order/packaging (published crate after core), CHANGELOG,
  FORMAT-JET/M88 cross-links. Gate: canonical set with lcov ≥ 95 %, file-
  size gate, `docs_validate` clean, CI green.

Content classes: the new crate, the jetdb dependency and the generated
fixture are Medium (released with Route B on 2026-11-14); no High-class
file is touched — the product keeps its no-COM/no-FFI rule, and the Access
COM step runs only in the fixture tooling at build time.

**Acceptance:** the tool imports the committed fixture's four objects into
the exact expected sidecar; the unsupported report names every skipped
control and property; the CLI merge/atomic-save/exit-code contract is
pinned by a subprocess suite; the generated fixture proves the six control
kinds and the default table; the database is never written and no
COM/unsafe path ships; the canonical set is green (lcov ≥ 95 %), the
file-size gate holds, `docs_validate` is clean and the phase's commits are
CI-green.

**Verification:** `cargo test -p datacraft-import` (library + CLI), the
committed-fixture contract test, the generated-fixture suite, the
canonical set (fmt, clippy `-D warnings`, workspace tests, lcov ≥ 95 %),
file-size gate, `cargo package --list -p datacraft-import`, `docs_validate`
clean and CI green.
**Verification:** Phase gate: fmt, clippy -D warnings and workspace tests green; lcov 19610/20519 = 95.570% (import crate 95.0% cli / 97.3% map / 98.9% lib); file-size gate clean; rustdoc -D warnings green; boundary guard 12 crates acyclic; cargo package --list clean for datacraft-import (licenses/NOTICE/README/sources); fixture manifest verifies; docs_validate clean. README migration guide, PUBLISHING order, THIRD-PARTY-NOTICES (jetdb), CHANGELOG, TESTING and ROADMAP §20 are current; M95 filed for the stale phase tables. CI remains the binding coverage gate after the push.
