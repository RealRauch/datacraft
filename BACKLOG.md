# BACKLOG.md — Open items (As of: 2026-10-31T22:26

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

### [ ] H4 — Define StorageEngine trait boundary — 🟠
- **Location:** `datacraft-storage/src/engine.rs`
- **Scope:** Backend-agnostic traits (open, schema introspection, row reads, writes, transactions), error taxonomy, feature-gated backends: `jet-rs` (product) and `libmdb-oracle` (test-only, behind cfg(test)/feature flag).
- **Acceptance:** Trait and mock backend fully unit-tested; no Jet/ACE-specific types leak across the boundary.

### [ ] H5 — Pure-Rust Jet page layer and file header decoder — 🟠
- **Location:** `datacraft-storage/src/jet/` (page layer, file header)
- **Scope:** Clean-room decoder for the Jet page layer based on public format documentation only (MDB Tools community docs, Microsoft Open Specifications): page headers, page allocation (usage maps), file header, version detection.
- **Acceptance:** Table-driven and proptest unit tests; byte-level parsing verified against fixture files; no code copied from MDB Tools.

### [ ] H6 — Pure-Rust table definition and data decoder — 🟠
- **Location:** `datacraft-storage/src/jet/` (TDEF/TDAT decoders)
- **Scope:** Table definition pages (TDEF), column metadata (V2/V3 descriptors), data pages (TDAT), data types (text/memo/number/date/currency/boolean/OLE), system table (`MSysObjects`) introspection.
- **Acceptance:** Fixtures readable table-by-table with deterministic ordering; unit tests colocated; coverage gate green.

### [ ] H7 — libmdb test oracle integration and golden-file suite — 🟠
- **Location:** `datacraft-storage/tests/`, `fixtures/`, CI oracle job
- **Scope (revised 2026-10-31, human decision):** libmdb oracle runs **in CI only** — ubuntu (apt `mdbtools`) and macOS (brew), no local Windows oracle; local verification relies on committed golden-file fixtures (created with MDB Tools tooling only). Strict schema + row comparison between our decoder and libmdb output; golden-file round-trip harness for the later write path. The CI system-deps step is High-class (within released H7 scope, flagged at the step).
- **Acceptance:** Oracle comparison green in CI on ubuntu + macOS; mismatches fail with byte-level diagnostics; local suite passes with golden files only.

### [ ] H8 — CLI client as fourth headless command-bus client — 🟠
- **Location:** new crate `crates/datacraft-cli` (bin `datacraft-cli`), step 9.4
- **Scope:** Fourth client of the command bus (besides GUI, MCP/JSON-RPC server, test harness): args → `Command` variants → `dispatch()` → human-readable or `--json` output of events and `inspect()` snapshots. No logic — transport only; shares the transport layer with the Phase 9.1 MCP server. Exit codes meaningful and deterministic output for headless tests.
- **Acceptance:** Every command-bus operation reachable from the shell; round-trip tests assert JSON snapshots; crate rules respected (ui/cli never touch storage directly).

---

## 🟡 MEDIUM

> No open items.

### [ ] M1 — Decide persistence format of the form/report design layer — 🟡
- **Location:** `docs/ARCHITECTURE.md` §7, `datacraft-core` (Phases 5–8)
- **Scope:** Real `.accdb` files store forms/reports in binary system tables. DataCraft persists its design layer (forms, reports, control layouts) for Phases 5–8 in an own versioned serde document next to the database file; the long-term option (native embedding into accdb-compatible storage once the ACE write path exists) must be decided before Phase 8 data binding is finalized.
- **Acceptance:** Written decision record (ADR in ARCHITECTURE.md) covering: file format + versioning of the design document, migration path to native embedding, impact on bit-for-bit write compatibility.

---

## 🟢 LOW

> No open items.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T1 — Boundary coverage below gate — error display arms and mock write-path validations untested — 🔵
- **Location:** `datacraft-storage/src/error.rs` (6/25 lines), `datacraft-storage/src/mock.rs` (91/104), found by the CI coverage job (run 41242950916) after step 2.1 skipped the llvm-cov re-run locally.
- **Goal:** Close the coverage gap to >= 95%: Display arms `Io`/`Unsupported`/`OutOfRange`/`SchemaViolation`, `Error::source()`/`From<io::Error>`, and the mock write-path validations (insert/update arity violation, update/delete out-of-range, empty-table window).
- **Scope:** Contract tests only — no production change. Lesson recorded: the canonical verification set (incl. llvm-cov) runs per step, not per phase.
- **Acceptance:** `cargo llvm-cov --workspace --exclude datacraft-ui --fail-under-lines 95` exits 0 locally and in CI.

---

## ✅ DONE INDEX

> (One line per completed item, with commit hash; details in archive.)
- K1 — Initialize git repository with GitHub Actions quality gates — done (done in `db6e6d5` — workflow with fmt/clippy/test-matrix/coverage/file-size jobs; cargo jobs guarded until workspace; amended in 6e2a3df (foreign .tmp-brand files unstaged))
- K2 — Author architecture blueprint (docs/ARCHITECTURE.md) — done (done in `f28165d` — docs/ARCHITECTURE.md: principles→mechanisms, crate graph, command API, storage decision record, phase mapping)
- H1 — Create Cargo workspace and dependency manifest — done (done in `68aab5f` + `930a289` — workspace with core/storage/ui crates, edition 2024, deps via cargo add (eframe 0.37.0), committed Cargo.lock)
- H2 — Coverage tooling and exception policy (docs/TESTING.md) — done (done in `16e6c4d` — docs/TESTING.md with exception registry #1 (datacraft-ui crate exclusion; coverage attribute unstable on stable), ci.yml coverage job updated)
- H3 — Toolchain pinning and lint configuration — done (done in `6e2a3df` — rust-toolchain.toml pinned to 1.98.1, MSRV 1.98 in workspace.package, shared lints (unsafe_code forbid, clippy all))
- K3 — Repair invalid CI workflow (hashFiles in job-level if) — done (done in the `fix(ci)` commit — preflight job pattern replaces invalid hashFiles-in-job-if; first valid run 41242950916 in progress)
