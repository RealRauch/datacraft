# BACKLOG.md — Open items (As of: 2026-11-01T02:08

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

### [ ] H8 — CLI client as fourth headless command-bus client — 🟠
- **Location:** new crate `crates/datacraft-cli` (bin `datacraft-cli`), step 9.4
- **Scope:** Fourth client of the command bus (besides GUI, MCP/JSON-RPC server, test harness): args → `Command` variants → `dispatch()` → human-readable or `--json` output of events and `inspect()` snapshots. No logic — transport only; shares the transport layer with the Phase 9.1 MCP server. Exit codes meaningful and deterministic output for headless tests.
- **Acceptance:** Every command-bus operation reachable from the shell; round-trip tests assert JSON snapshots; crate rules respected (ui/cli never touch storage directly).

### [ ] H9 — Jet4 writer workstream (create tables, write rows) — 🟠
- **Location:** `datacraft-storage/src/jet/` (writer), post-Phase-4 workstream
- **Scope:** Jet4 write milestone from ARCHITECTURE §6 (create tables, insert/update rows, usage-map allocation, index maintenance deferred). UI write commands route to `InMemoryEngine` until this lands; then the `Jet4Backend` gains write capability. Bit-for-bit goal: writes must open without repair prompts in MS Access — proven by golden round-trips + oracle.
- **Acceptance:** Access opens written files without repair; decode(encode(decode(x))) == decode(x) over fixtures; write-path coverage >= gate.

### [ ] H10 — Access data types and field properties — core domain model — 🟠
- **Location:** `datacraft-core/src/schema/` (new module)
- **Scope:** Logical Access domain model in core (ARCHITECTURE §7): `FieldKind` (Text{max_len}, Memo, Number(NumericKind), DateTime, Currency, AutoNumber(AutoKind), Boolean{tri_state}, Binary(BinaryKind)), numeric/auto/binary sub-kinds, `IndexMode`, typed `FieldProperties` (caption, input mask, format, default value, validation rule/text, required, allow zero length, indexed) with per-kind validation semantics, and a domain `FieldValue` the later command bus validates against. Storage keeps its physical types and maps them onto this model in Phase 4 (storage -> core is the allowed dependency direction).
- **Acceptance:** Unit + proptest coverage per kind (boundaries, property validity rules, JSON round-trips); no core dependency on storage; canonical verification set green.

### [ ] H11 — Versioned schema sidecar document (serde persistence) — 🟠
- **Location:** `datacraft-core/src/schema/document.rs` (versioned serde document), ADR-1 in docs/ARCHITECTURE.md §7
- **Scope:** Versioned schema document (the `.datacraft` sidecar format foundation, ADR-1): serde JSON with a `schema_version` field, save/load + strict round-trip, version gate (own version parses, higher version -> precise `Unsupported` error, lower versions migrate forward — identity migration for v1), deterministic serialization for byte-stable files. The document carries tables + field defs incl. properties; forms/reports design layers join it in Phases 5-8.
- **Acceptance:** Round-trip proptests over generated schemas; version-gate unit tests (v_next rejected with message, v_current ok, v_prev migrates); deterministic bytes (two serializations of equal documents are identical); canonical verification set green.

### [ ] H12 — GUID field kind — plain Replication ID columns unmodelable — 🟠
- **Location:** `datacraft-core/src/schema/field.rs` (FieldKind), `datacraft-storage` (Phase 4 JetType mapping)
- **Problem:** No `FieldKind` variant accepts `FieldValue::Guid` — Jet/ACE GUID columns that are NOT AutoNumbers (Access UI: Number with FieldSize "Replication ID") cannot be modeled. Found while closing step 3.1/3.2 coverage: every kind rejects Guid values, so the Phase 4 storage->core mapping has no target for plain Guid columns.
- **Scope:** Decide the modeling (Number(NumericKind::ReplicationId) — Access-faithful — vs a dedicated FieldKind::Guid), implement validate_value to accept FieldValue::Guid, extend the family/mismatch tables, document the decision in docs/FORMAT-JET.md or ARCHITECTURE §7. Do it before the Phase 4 storage mapping step.
- **Acceptance:** A Guid column of a real fixture maps to a FieldKind that accepts FieldValue::Guid; family/mismatch coverage stays >= gate.

---

## 🟡 MEDIUM

> No open items.

---

## 🟢 LOW

> No open items.

### [ ] L1 — Jet 3 (Access 97) read support — 🟢
- **Location:** `datacraft-storage/src/jet/`
- **Scope:** Jet 3 (Access 97) read support: 2048-byte pages, codepage-based text (no UTF-16), Jet3 TDEF layout. Deferred per default decision 2026-10-31 — Jet4 and ACE read paths come first; Jet3 files are detected via the version dword and rejected with a precise `Unsupported` error until this item lands.
- **Acceptance:** A Jet3 fixture decodes to schema + rows through the same `StorageEngine` boundary.

### [ ] L2 — Jet4 column type 0x11 (MSysAccessObjects.Data) — study and decide mapping — 🟢
- **Location:** `crates/datacraft-storage/src/jet/tdef.rs` (JetType::from_byte), `tests/oracle.rs` (skip path)
- **Problem:** Jet4 column type `0x11` is not in HACKING's (incomplete) type table and not in `docs/FORMAT-JET.md`; both fixtures use it in `MSysAccessObjects.Data` (long-value column). mdb-tools itself warns "unsupported data type 17" on the same column. Our reader answers with a precise `Unsupported`, and the oracle skips such tables.
- **Scope:** Focused study on the fixture's MSysAccessObjects rows (likely long-binary/LVAL like OLE; jackcess reads the table fine). Then either map the type or keep the precise `Unsupported` with a documented reason.
- **Acceptance:** Decision documented in `docs/FORMAT-JET.md`; oracle no longer needs the skip, or the skip carries the documented reason.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T3 — Fixture pin integrity — restore pristine testV2000, keep Access-touched variant, manifest + CI hash gate — 🔵
- **Location:** `fixtures/` (`testV2000.mdb`, manifest), CI workflow
- **Problem:** `fixtures/testV2000.mdb` no longer matches the jackcess pin (`be0a2139`): the file was opened in real Access, which rewrote system-table pages and appended 22 pages on open (full evidence + hashes in `docs/FIXTURE-SOURCES.md` §4). A silently drifted fixture poisons the oracle evidence base — the recorded pin hash no longer describes the file on disk.
- **Scope:** restore pristine `testV2000.mdb` from the verified pin source (sha256 `126bad49…`); keep the Access-rewritten file as `fixtures/testV2000-access-touched.mdb` (second writer variant — T2 material, do not discard); replace `jackcess-pin.txt` with a per-file manifest (`repo@commit` + sha256, sources and recommended Phase-2 pin set per `docs/FIXTURE-SOURCES.md` §5); add a CI job that verifies committed fixture hashes; document the rule: fixtures are never opened in writer-capable applications (Access) — byte experiments run on copies only.
- **Acceptance:** committed `fixtures/manifest` hashes match the committed files and CI fails on drift; pristine + Access-touched variants exist as separate fixtures; pin set per docs/FIXTURE-SOURCES.md §5 pinned; rule documented in docs/TESTING.md.

---

## ✅ DONE INDEX

> (One line per completed item, with commit hash; details in archive.)
- K1 — Initialize git repository with GitHub Actions quality gates — done (done in `db6e6d5` — workflow with fmt/clippy/test-matrix/coverage/file-size jobs; cargo jobs guarded until workspace; amended in 6e2a3df (foreign .tmp-brand files unstaged))
- K2 — Author architecture blueprint (docs/ARCHITECTURE.md) — done (done in `f28165d` — docs/ARCHITECTURE.md: principles→mechanisms, crate graph, command API, storage decision record, phase mapping)
- H1 — Create Cargo workspace and dependency manifest — done (done in `68aab5f` + `930a289` — workspace with core/storage/ui crates, edition 2024, deps via cargo add (eframe 0.37.0), committed Cargo.lock)
- H2 — Coverage tooling and exception policy (docs/TESTING.md) — done (done in `16e6c4d` — docs/TESTING.md with exception registry #1 (datacraft-ui crate exclusion; coverage attribute unstable on stable), ci.yml coverage job updated)
- H3 — Toolchain pinning and lint configuration — done (done in `6e2a3df` — rust-toolchain.toml pinned to 1.98.1, MSRV 1.98 in workspace.package, shared lints (unsafe_code forbid, clippy all))
- K3 — Repair invalid CI workflow (hashFiles in job-level if) — done (done in the `fix(ci)` commit — preflight job pattern replaces invalid hashFiles-in-job-if; first valid run 41242950916 in progress)
- T1 — Boundary coverage below gate — error display arms and mock write-path validations untested — done (done in `2084be0` — contract+colocated tests close gap; coverage 81.92% -> 98.17%; found by CI gate (run 41242950916), lesson: canonical verification set runs per step)
- M1 — Decide persistence format of the form/report design layer — done (decided 2026-10-31 via ADR-1 in docs/ARCHITECTURE.md §7 (human offline, default ratified by blanket phase 3-9 release): versioned .datacraft sidecar (serde JSON) for phases 5-8; native accdb embedding only after ACE writer)
- T2 — Real-row tail semantics — mask bytes and var_table entries disagree with synthetic model — done (commit: null-mask bit index = column id (row.rs); oracle green on both fixture writers; CI oracle job added)
- H4 — Define StorageEngine trait boundary — done (steps 2.1 (test 4a507ec, feat 6b7dbc8))
- H5 — Pure-Rust Jet page layer and file header decoder — done (steps 2.2 (test 442f1c9, feat 984006f))
- H6 — Pure-Rust table definition and data decoder — done (steps 2.3 (test b2c8f63, feat 367a07b))
- H7 — libmdb test oracle integration and golden-file suite — done (steps 2.4 + T2 fixes (a200dee, 92bce57, 683a88f); CI oracle job added; T3 keeps fixture-pin follow-up open)
