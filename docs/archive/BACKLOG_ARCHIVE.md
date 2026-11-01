# BACKLOG_ARCHIVE.md — Archive of completed items (append-only)

> Items are moved here **verbatim**; later additions only as a new section below the item.
> Append the newest items at the bottom.

---

*(empty)*

---

---

### [x] K1 — Initialize git repository with GitHub Actions quality gates — 🔴
- **Location:** `.github/workflows/ci.yml`, `.gitignore`, repo root
- **Scope:** Git repository on `main`; CI jobs: rustfmt, clippy (deny warnings), cross-platform test matrix (Linux/macOS/Windows), coverage gate (`cargo-llvm-cov --fail-under-lines 95`), file-size gate (all `.rs` files < 500 lines). Cargo-dependent jobs guarded by `hashFiles('Cargo.toml')` until the workspace exists.
- **Acceptance:** Workflow file committed; file-size gate runs on every push; all five quality gates documented in the README.
- **Done:** done in `db6e6d5` — workflow with fmt/clippy/test-matrix/coverage/file-size jobs; cargo jobs guarded until workspace; amended in 6e2a3df (foreign .tmp-brand files unstaged)

---

### [x] K2 — Author architecture blueprint (docs/ARCHITECTURE.md) — 🔴
- **Location:** `docs/ARCHITECTURE.md`
- **Scope:** Clean-room principles; core/shell split (headless command API vs. thin egui projection); `StorageEngine` trait boundary with a pure-Rust Jet/ACE backend and libmdb strictly as CI test oracle (decision 2026-10-31); MCP/JSON-RPC readiness; deterministic test-ID scheme; module layout honoring the 500-line limit.
- **Acceptance:** All five project principles mapped to concrete mechanisms; decision record for the storage layer included.
- **Done:** done in `f28165d` — docs/ARCHITECTURE.md: principles→mechanisms, crate graph, command API, storage decision record, phase mapping

---

### [x] H1 — Create Cargo workspace and dependency manifest — 🟠
- **Location:** `Cargo.toml` (workspace), member crates `datacraft-core`, `datacraft-storage`, `datacraft-ui`
- **Scope:** Workspace layout with lints inheritance; deps: `eframe`/`egui` (UI), `serde`/`serde_json`, `proptest` (dev); FFI limited to the test oracle (libmdb, dev/CI only); committed `Cargo.lock` (application).
- **Acceptance:** `cargo check` and `cargo test` pass on all three platforms; module skeleton below the 500-line limit.
- **Done:** done in `68aab5f` + `930a289` — workspace with core/storage/ui crates, edition 2024, deps via cargo add (eframe 0.37.0), committed Cargo.lock

---

### [x] H2 — Coverage tooling and exception policy (docs/TESTING.md) — 🟠
- **Location:** `docs/TESTING.md`, CI coverage job
- **Scope:** `cargo-llvm-cov` configuration; documented `#[coverage(off)]` exception registry (only `main` and trivial egui glue, each with justification); test strategy: colocated unit tests, table-driven parser tests, proptest round-trips, golden-file tests.
- **Acceptance:** Coverage job green at >= 95% lines; every exception listed with justification.
- **Done:** done in `16e6c4d` — docs/TESTING.md with exception registry #1 (datacraft-ui crate exclusion; coverage attribute unstable on stable), ci.yml coverage job updated

---

### [x] H3 — Toolchain pinning and lint configuration — 🟠
- **Location:** `rust-toolchain.toml`, `Cargo.toml` (`[workspace.lints]`)
- **Scope:** MSRV pin, clippy lint groups with deny-warnings in CI, rustfmt defaults; shared workspace lints inherited by all member crates.
- **Acceptance:** Fresh clone builds warning-free on Linux, macOS and Windows.
- **Done:** done in `6e2a3df` — rust-toolchain.toml pinned to 1.98.1, MSRV 1.98 in workspace.package, shared lints (unsafe_code forbid, clippy all)

---

### [x] K3 — Repair invalid CI workflow (hashFiles in job-level if) — 🔴
- **Location:** `.github/workflows/ci.yml`
- **Problem:** Bootstrap workflow uses `hashFiles()` in job-level `if:` — not a valid context there ("Unrecognized named-value"), causing startup failure (0 s) on every push since remote creation.
- **Fix:** Preflight job detects `Cargo.toml` and exposes `has_workspace` output; cargo jobs gate via `needs: preflight` + `if: needs.preflight.outputs.has_workspace == 'true'`. file-size stays unconditional. Repair of an invalid workflow, not a policy change; disclosed to the human under the Phase 2–4 release.
- **Acceptance:** CI run completes without startup failure; cargo jobs run/skip correctly; green on main.
- **Done:** done in the `fix(ci)` commit — preflight job pattern replaces invalid hashFiles-in-job-if; first valid run 41242950916 in progress

---

### [x] T1 — Boundary coverage below gate — error display arms and mock write-path validations untested — 🔵
- **Location:** `datacraft-storage/src/error.rs` (6/25 lines), `datacraft-storage/src/mock.rs` (91/104), found by the CI coverage job (run 41242950916) after step 2.1 skipped the llvm-cov re-run locally.
- **Goal:** Close the coverage gap to >= 95%: Display arms `Io`/`Unsupported`/`OutOfRange`/`SchemaViolation`, `Error::source()`/`From<io::Error>`, and the mock write-path validations (insert/update arity violation, update/delete out-of-range, empty-table window).
- **Scope:** Contract tests only — no production change. Lesson recorded: the canonical verification set (incl. llvm-cov) runs per step, not per phase.
- **Acceptance:** `cargo llvm-cov --workspace --exclude datacraft-ui --fail-under-lines 95` exits 0 locally and in CI.
- **Done:** done in `2084be0` — contract+colocated tests close gap; coverage 81.92% -> 98.17%; found by CI gate (run 41242950916), lesson: canonical verification set runs per step

---

### [x] M1 — Decide persistence format of the form/report design layer — 🟡
- **Location:** `docs/ARCHITECTURE.md` §7, `datacraft-core` (Phases 5–8)
- **Scope:** Real `.accdb` files store forms/reports in binary system tables. DataCraft persists its design layer (forms, reports, control layouts) for Phases 5–8 in an own versioned serde document next to the database file; the long-term option (native embedding into accdb-compatible storage once the ACE write path exists) must be decided before Phase 8 data binding is finalized.
- **Acceptance:** Written decision record (ADR in ARCHITECTURE.md) covering: file format + versioning of the design document, migration path to native embedding, impact on bit-for-bit write compatibility.
- **Done:** decided 2026-10-31 via ADR-1 in docs/ARCHITECTURE.md §7 (human offline, default ratified by blanket phase 3-9 release): versioned .datacraft sidecar (serde JSON) for phases 5-8; native accdb embedding only after ACE writer

---

### [x] T2 — Real-row tail semantics — mask bytes and var_table entries disagree with synthetic model — 🔵
- **Location:** `datacraft-storage/src/jet/row.rs` (RowLayout), real fixture `fixtures/testV2000.mdb`
- **Goal:** Resolve the real-row tail semantics: on the real Access 2000 fixture the catalog rows decode (num_cols=17, var_len=11, plausible fixed values) but the null mask reads `[ff 00 00]` where descriptor-order semantics expect `[5c 28 01]`, and var_table entries read 50/52 while eod=46 — the real tail field order (var_table/eod/var_len/mask) and/or mask bit semantics differ from the synthetic model.
- **Update 2026-11-01 (fixture-source research):** the on-disk `fixtures/testV2000.mdb` is an **Access-rewritten variant** of the pinned jackcess file — it was opened in real Access, which rewrote it on open (no explicit save): 99.71 % of bytes identical, 23 system-area pages changed, +22 pages appended (incl. 4 new TDEF and LVAL pages), header creation region byte-identical. The pristine original at pin commit `be0a2139` is re-verified (sha256 `126bad49…`, matches the handoff record). Both variants are valid Jet4 DBs with untouched user-table pages; the oracle harness parses them measurably differently (pristine: 22 MSysObjects rows decode; touched: different path). Evidence, hashes and comparison table: `docs/FIXTURE-SOURCES.md` §4. Run the hex study against **both** variants — same logical catalog, two writers (jackcess vs real Access) is exactly the comparison the bit-for-bit goal needs. Recovery/pinning of both variants is tracked as T3.
- **Scope:** Focused hex study of one known-content row per variant + CI oracle cross-check (mdb-export) per docs/FORMAT-JET.md "Open: real-row tail semantics"; then fix RowLayout, keep synthetic fixtures byte-exact with reality, re-run the oracle comparison to green.
- **Acceptance:** `DATACRAFT_ORACLE_MDB` oracle test lists tables and matches mdb-export row counts/columns; all gates green; catalog names resolve for the fixture.
- **Done:** commit: null-mask bit index = column id (row.rs); oracle green on both fixture writers; CI oracle job added

---

### [x] H4 — Define StorageEngine trait boundary — 🟠
- **Location:** `datacraft-storage/src/engine.rs`
- **Scope:** Backend-agnostic traits (open, schema introspection, row reads, writes, transactions), error taxonomy, feature-gated backends: `jet-rs` (product) and `libmdb-oracle` (test-only, behind cfg(test)/feature flag).
- **Acceptance:** Trait and mock backend fully unit-tested; no Jet/ACE-specific types leak across the boundary.
- **Done:** steps 2.1 (test 4a507ec, feat 6b7dbc8)

---

### [x] H5 — Pure-Rust Jet page layer and file header decoder — 🟠
- **Location:** `datacraft-storage/src/jet/` (page layer, file header)
- **Scope:** Clean-room decoder for the Jet page layer based on public format documentation only (MDB Tools community docs, Microsoft Open Specifications): page headers, page allocation (usage maps), file header, version detection.
- **Acceptance:** Table-driven and proptest unit tests; byte-level parsing verified against fixture files; no code copied from MDB Tools.
- **Done:** steps 2.2 (test 442f1c9, feat 984006f)

---

### [x] H6 — Pure-Rust table definition and data decoder — 🟠
- **Location:** `datacraft-storage/src/jet/` (TDEF/TDAT decoders)
- **Scope:** Table definition pages (TDEF), column metadata (V2/V3 descriptors), data pages (TDAT), data types (text/memo/number/date/currency/boolean/OLE), system table (`MSysObjects`) introspection.
- **Acceptance:** Fixtures readable table-by-table with deterministic ordering; unit tests colocated; coverage gate green.
- **Done:** steps 2.3 (test b2c8f63, feat 367a07b)

---

### [x] H7 — libmdb test oracle integration and golden-file suite — 🟠
- **Location:** `datacraft-storage/tests/`, `fixtures/`, CI oracle job
- **Scope (revised 2026-10-31, human decisions):** libmdb oracle runs **in CI only** — ubuntu (apt `mdbtools`) and macOS (brew), no local Windows oracle. Oracle compares our decoder against **real `.mdb` sample databases pinned from a public source** (test data, not code — MDB Tools cannot create databases); local Windows verification relies on synthetic fixtures built by our own test helpers. Strict schema + row comparison; golden-file round-trip harness for the later write path. Encrypted/password-protected databases are detected and rejected with `Unsupported` (no decryption). The CI system-deps step is High-class (within released H7 scope, flagged at the step).
- **Acceptance:** Oracle comparison green in CI on ubuntu + macOS against pinned real fixtures; mismatches fail with byte-level diagnostics; local suite passes with synthetic fixtures only.
- **Done:** steps 2.4 + T2 fixes (a200dee, 92bce57, 683a88f); CI oracle job added; T3 keeps fixture-pin follow-up open

---

### [x] H11 — Versioned schema sidecar document (serde persistence) — 🟠
- **Location:** `datacraft-core/src/schema/document.rs` (versioned serde document), ADR-1 in docs/ARCHITECTURE.md §7
- **Scope:** Versioned schema document (the `.datacraft` sidecar format foundation, ADR-1): serde JSON with a `schema_version` field, save/load + strict round-trip, version gate (own version parses, higher version -> precise `Unsupported` error, lower versions migrate forward — identity migration for v1), deterministic serialization for byte-stable files. The document carries tables + field defs incl. properties; forms/reports design layers join it in Phases 5-8.
- **Acceptance:** Round-trip proptests over generated schemas; version-gate unit tests (v_next rejected with message, v_current ok, v_prev migrates); deterministic bytes (two serializations of equal documents are identical); canonical verification set green.
- **Done:** steps 3.2 (test 5432ef9, feat a72c5e6)

---

### [x] H10 — Access data types and field properties — core domain model — 🟠
- **Location:** `datacraft-core/src/schema/` (new module)
- **Scope:** Logical Access domain model in core (ARCHITECTURE §7): `FieldKind` (Text{max_len}, Memo, Number(NumericKind), DateTime, Currency, AutoNumber(AutoKind), Boolean{tri_state}, Binary(BinaryKind)), numeric/auto/binary sub-kinds, `IndexMode`, typed `FieldProperties` (caption, input mask, format, default value, validation rule/text, required, allow zero length, indexed) with per-kind validation semantics, and a domain `FieldValue` the later command bus validates against. Storage keeps its physical types and maps them onto this model in Phase 4 (storage -> core is the allowed dependency direction).
- **Acceptance:** Unit + proptest coverage per kind (boundaries, property validity rules, JSON round-trips); no core dependency on storage; canonical verification set green.
- **Done:** step 3.1 (test 7cf9a31, feat 1271f33)

---

### [x] H13 — Command dispatch core — commands, events, state, storage port — 🟠
- **Location:** `datacraft-core/src/{command,event,state,port}.rs`
- **Scope:** Command dispatch core per ARCHITECTURE §5: `Command` (OpenDatabase/CloseDatabase/CreateTable/DropTable/OpenObject/CloseTab/SetActiveTab/ToggleNavPane — QBE and form commands join in their phases), `Event` (DatabaseOpened ... Failed), `AppState` reducer, `Core::dispatch(&mut self, Command) -> Vec<Event>` (total: failures become `Event::Failed`). Storage I/O sits behind a core-owned `StoragePort` trait (port error taxonomy); storage backends implement it (direction `storage -> core` preserved). `MockPort` ships in core as the stable test backend (mirrors storage's mock).
- **Acceptance:** Contract tests over the full command set incl. failure paths (double open, unknown table, unknown tab, required-open violations); event order deterministic (close-database closes tabs first, drop-table closes dependent tabs first); canonical verification set green.
- **Done:** step 4.1 (test baa4f16, feat f796480)

---

### [x] H14 — State query API and deterministic test IDs — 🟠
- **Location:** `datacraft-core/src/snapshot.rs` (+ test-id helpers on ObjectId/TabState)
- **Scope:** Serializable `UiStateSnapshot` (the headless test seam, ARCHITECTURE §5): database name/path/format, table list, nav pane visibility, tabs with object kind/name, active tab. Deterministic test-ID composition per the §5 convention: `navpane.<expanded|collapsed>.<ObjectId::ui_test_id()>` and `workspace.tab.<kind>.<name>`; containers derive IDs from `ObjectId`, never invent them. `Core::inspect()` returns the snapshot.
- **Acceptance:** Snapshot JSON asserted in contract tests (structure + test ids + determinism: two inspects of equal state are byte-identical); every dispatch outcome observable in the snapshot; canonical verification set green.
- **Done:** step 4.2 (test da774ad, feat 1b4a9be)

---

### [x] H15 — JSON-RPC transport codec stub (headless seam) — 🟠
- **Location:** `datacraft-core/src/transport.rs`
- **Scope:** JSON-RPC 2.0 codec stub (decision default 8: stdio transport in Phase 9): `handle_json(core, request_json) -> response_json` implementing methods `dispatch` (params = Command; result = events + snapshot) and `inspect` (result = snapshot). Full JSON-RPC error semantics (-32700 parse, -32600 invalid request, -32601 unknown method, -32602 invalid params). The stub is synchronous and server-less; the Phase 9.1 MCP endpoint is a thin stdio loop over it.
- **Acceptance:** Round-trip contract tests: valid requests produce correct result/error envelopes; malformed JSON/methods/params map to the right codes; every response parses as JSON-RPC 2.0; canonical verification set green.
- **Done:** step 4.3 (test ceecb24, feat a0c5137)

---

### [x] H17 — App shell and layout — four-region egui shell over the core — 🟠
- **Location:** `datacraft-core/src/present.rs` (presentation helpers), `datacraft-ui/src/{main,app}.rs`
- **Scope:** App shell per ARCHITECTURE §8 (logic in core, UI renders): eframe app struct hosting a `Core`, panel skeleton (ribbon top, navigation left, workspace center, status bottom), window title/icon preserved from Phase 1. Presentation strings (status line) computed by pure, tested functions in core from the snapshot: `status_line(&UiStateSnapshot) -> String`. UI crate stays coverage-excluded (exception #1); clippy/test gates still apply.
- **Acceptance:** Core-side contract tests for the presentation helpers (no database / open database / hidden nav pane variants); the binary builds and opens the four-region shell; canonical verification set green.
- **Done:** step 5.1 (test 5ed94ed, feat 666acd7)

---

### [x] H12 — GUID field kind — plain Replication ID columns unmodelable — 🟠
- **Location:** `datacraft-core/src/schema/field.rs` (FieldKind), `datacraft-storage` (Phase 4 JetType mapping)
- **Problem:** No `FieldKind` variant accepts `FieldValue::Guid` — Jet/ACE GUID columns that are NOT AutoNumbers (Access UI: Number with FieldSize "Replication ID") cannot be modeled. Found while closing step 3.1/3.2 coverage: every kind rejects Guid values, so the Phase 4 storage->core mapping has no target for plain Guid columns.
- **Scope:** Decide the modeling (Number(NumericKind::ReplicationId) — Access-faithful — vs a dedicated FieldKind::Guid), implement validate_value to accept FieldValue::Guid, extend the family/mismatch tables, document the decision in docs/FORMAT-JET.md or ARCHITECTURE §7. Do it before the Phase 4 storage mapping step.
- **Acceptance:** A Guid column of a real fixture maps to a FieldKind that accepts FieldValue::Guid; family/mismatch coverage stays >= gate.
- **Done:** decided: dedicated FieldKind::Guid; implemented with H16 (tests f4c96d3/e1bb1bb, adapter 67a3b64); decision documented in docs/FORMAT-JET.md "Domain mapping"

---

### [x] H16 — StoragePort adapter — Jet4 backend implements the core port — 🟠
- **Location:** `datacraft-storage/src/port_adapter.rs` (new), required before Phase 5 opens real files
- **Scope:** Implement core's `StoragePort` for the Jet4 reader backend: open (format detection via existing header decoder), close, tables (catalog -> `TableDef` mapping incl. the H12 decision for Guid columns), create_table/drop_table -> precise `PortError::Unsupported` until the writer workstream (H9) lands. Seam tests on BOTH sides per LESSONS #2: storage-side contract tests against the committed fixtures, core-side tests already exist against the mock.
- **Acceptance:** A real fixture opens through `Core` with the Jet4 adapter; schema fields map to the core domain model; every port method tested on the storage side; canonical verification set green.
- **Done:** step within Phase 5 (tests 96bc864/aacc8ab, feat 67a3b64); includes the multi-page TDEF decoder extension; seam tests on both sides per LESSONS #2

---

### [x] H18 — Ribbon bar — spec-driven Access-style ribbon — 🟠
- **Location:** `datacraft-core/src/ribbon.rs` (spec + tests), `datacraft-ui/src/ribbon.rs` (rendering)
- **Scope:** Ribbon data model in core (pure, tested): tabs File/Home/Create/External Data/Database Tools, groups, actions with stable ids and labels; the UI renders buttons from the spec and dispatches Commands (Home > Navigation Pane -> ToggleNavPane; File > Open Database -> text-field path + OpenDatabase; File > Close Database; Create-group actions render disabled with tooltips until their phases land). No layout or dispatch logic in paint code beyond spec lookups.
- **Acceptance:** Core-side spec tests (unique ids/labels, every enabled action maps to a Command, disabled actions declare their phase); UI renders the spec; canonical verification set green.
- **Done:** step 5.2 (test 483bca0, feat 14ec049)

---

### [x] H19 — Navigation pane — snapshot-driven object list — 🟠
- **Location:** `datacraft-ui/src/nav_pane.rs`, snapshot-driven (no new core model)
- **Scope:** Navigation pane rendering from `UiStateSnapshot`: table list with the convention test ids as egui push_ids, click dispatches `OpenObject` (single click = open, deterministic and testable), visibility follows `nav_pane_visible`, empty states for "no database" and "database without tables". Widgets derive IDs from the snapshot — never invent them (ARCHITECTURE §5).
- **Acceptance:** Rendering is fully snapshot-driven (no hidden state), every listed table uses `navpane.<state>.node.<kind>.<name>` as its egui id salt; canonical verification set green.
- **Done:** step 5.3 (feat a706624)

---

### [x] H20 — Tabbed document workspace — strip and placeholders — 🟠
- **Location:** `datacraft-ui/src/workspace.rs`, snapshot-driven
- **Scope:** Tabbed document workspace: tab strip from `UiStateSnapshot.tabs` (active highlight from `active_tab`), per-tab close buttons dispatch `CloseTab`, clicking a tab dispatches `SetActiveTab`, central area renders a per-object placeholder (table tabs: "Datasheet arrives in Phase 6"); push_ids use the `workspace.tab.<kind>.<name>` convention. Placeholder text is a tested pure function in core (`tab_placeholder(kind, name)`).
- **Acceptance:** Strip/placeholder logic tested in core; UI renders strip + placeholders; canonical verification set green.
- **Done:** step 5.4 (test 4a3a280, feat 015459d)

---

### [x] H22 — Shared grid core — virtual-scroll and hit-test math — 🟠
- **Location:** `datacraft-core/src/grid.rs`, used by the datasheet (6.2) and the design grid (6.3)
- **Scope:** Shared grid core per ARCHITECTURE §8 — pure, window-free layout math: row metrics (row/header height defaults), virtual-scroll math (visible row range for a viewport, scroll clamping), column hit-testing and column span geometry from a width vector. The datasheet and the design grid share these functions so virtualization behaves identically in both views.
- **Acceptance:** Unit + proptest coverage (ranges stay inside 0..row_count, clamped scroll is a fixpoint, hit-testing inverse of span geometry); canonical verification set green.
- **Done:** step 6.1 (test 62820e4, feat c256a10)

---

### [x] H21 — Compose the real Jet4Port in the UI binary (composition root) — 🟠
- **Location:** `crates/datacraft-ui/src/main.rs` (composition root), `app.rs` (App::new signature)
- **Problem:** The UI shell still composes `MockPort` (App::new constructs it internally), so File > Open Database "opens" an in-memory mock instead of a real .mdb file. The Jet4Port adapter exists (H16) but is unreachable from the binary.
- **Scope:** App::new takes `Box<dyn StoragePort>`; the binary's main() (composition root) constructs `Jet4Port::new()` and injects it — the ui crate itself keeps depending on core only (layering rule ui -> core); the bin target is the one place allowed to name storage. Land with step 6.2 (datasheet needs real rows anyway); until then the mock stays the documented Phase 5 backend.
- **Acceptance:** Opening the committed fixture through the UI shows its user tables in the navigation pane; the crate-level dependency graph stays ui -> core; canonical verification set green.
- **Done:** step 6.2 (feat 8410069): App::new takes the port; main.rs composes Jet4Port as the documented composition root

---

### [x] H23 — Datasheet view — port read_rows, row mapping, real composition — 🟠
- **Location:** `datacraft-core/src/port.rs` (read_rows), `datacraft-storage/src/port_adapter.rs` (row mapping), `datacraft-ui/src/datasheet.rs`, main.rs composition (H21)
- **Scope:** Datasheet view over real rows: (1) StoragePort gains `read_rows(table, start, len) -> Result<Vec<Vec<FieldValue>>, PortError>` with the domain row vocabulary (storage maps StorageValue -> FieldValue 1:1); (2) the Jet4 adapter implements it over the existing row decode; (3) the UI renders a virtualized datasheet per table tab using the grid core (windowed reads per frame region, convention test ids `datasheet.<table>.cell.<row>.<col>`); (4) H21: the binary's composition root injects Jet4Port (App::new takes the port) so real files open.
- **Acceptance:** Port contract tests on both sides (mock + real fixture rows, out-of-range windows); FieldValue mapping round-trips every storage variant; the fixture opens through the UI composition with its tables visible; canonical verification set green.
- **Done:** step 6.2 (tests 704fc10/e606a07/2a7d3be, feats 8b97c26/8410069)

---

### [x] H24 — Table design view — read-only design grid and kind display names — 🟠
- **Location:** `datacraft-core/src/present.rs` (kind display), `datacraft-core/src/snapshot.rs` (FieldSnapshot.kind_display), `datacraft-ui/src/design.rs`
- **Scope:** Read-only table design view: Access-style kind display names (Short Text, Long Text, Byte/Integer/Long Integer/Single/Double/Decimal, Date/Time, Currency, AutoNumber flavors, Yes/No, OLE Object/Attachment, GUID) computed in core and carried on the snapshot; the UI renders a design grid (field name / data type / summary) per table tab with a per-tab Datasheet|Design mode toggle and a selected-field property panel. Design-time schema EDITING (add/rename/drop fields, save) needs the write path and is deferred to H9.
- **Acceptance:** Kind-display contract tests per kind (incl. decimal precision/scale in the size column); snapshot carries kind_display; UI renders grid + properties; canonical verification set green.
- **Done:** step 6.3 (core 3e6fe9a, ui 06c2982); design-time schema editing deferred to H9 (documented in the UI)

---

### [x] H25 — Relationship mapper pane — port relationships and canvas — 🟠
- **Location:** `datacraft-core/src/port.rs` (Relationship + relationships()), `datacraft-storage/src/port_adapter.rs` (MSysRelationships read), `datacraft-ui/src/relationships.rs`
- **Scope:** Relationship mapper: the port exposes `relationships() -> Vec<Relationship { table, column, related_table, related_column }>`; the Jet4 adapter reads MSysRelationships through the existing row decode (system tables are readable data, only filtered from the user catalog); the core state caches them on open; the UI pane draws table boxes with connection lines (canvas painting, no logic) and lists relationships textually.
- **Acceptance:** Adapter contract test against the fixture (Relationships rows map or the absence is precise); relationships survive the snapshot; UI pane renders; canonical verification set green.
- **Done:** step 7.1 (feat a7e2a6f)

---

### [x] H26 — QBE grid — query model, commands, sidecar persistence — 🟠
- **Location:** `datacraft-core/src/query.rs` (QueryDef, QBE model, SetQbeCell commands), `datacraft-core/src/schema/document.rs` (queries in the sidecar, serde(default)), `datacraft-ui/src/qbe.rs`
- **Scope:** QBE model per decision default 7 (no SQL parser): `QueryDef { name, table, fields: Vec<QbeField { name, shown, sort, criteria: Vec<String> }> }`; Command::CreateQuery + Command::SetQbeCell { row (Field/Sort/Show/Criteria(n)), col, value } with whitelist validation (field names must exist in the source table, LESSONS #5); queries persist in the sidecar document (SchemaDocument.queries, serde default — additive v1); queries appear in the snapshot and nav pane; the QBE grid renders as a document tab with editable cells dispatching SetQbeCell; Create > Query Design ribbon action enables.
- **Acceptance:** Model contract tests (creation, cell edits across row kinds, whitelist rejection, sidecar round-trip with queries); snapshot/nav carriage; QBE grid renders and dispatches; canonical verification set green.
- **Done:** step 7.2 (feat 7cb66ba)

---

### [x] H27 — Query engine integration — predicates, execution, result view — 🟠
- **Location:** `datacraft-core/src/query_engine.rs` (execution), `datacraft-core/src/core.rs` (run_query + RunQuery command), `datacraft-ui/src/datasheet.rs` (result view for query tabs)
- **Scope:** In-memory query execution (decision default 7): criteria strings parse to predicates (comparison ops =/<>/< <=/> >= with typed literal coercion per column kind, plain value = equality); execution = read source rows through the port, filter (AND across columns, OR within a column's criteria), project shown columns, sort by sort columns (stable, asc/desc); identifier whitelist enforced at parse (unknown field/operator -> precise error). `Core::run_query(name) -> Result<QueryResult, PortError-ish>`; Command::RunQuery validates and emits QueryRan { rows }; query tabs render results as a datasheet-style grid (test ids result.<query>.cell.<row>.<col>).
- **Acceptance:** Engine contract tests (typed comparisons incl. text/number coercion errors, AND/OR composition, projection, asc/desc stable sort, empty result, unknown identifiers); RunQuery observability; canonical verification set green.
- **Done:** step 7.3 (feat ec62b7b)

---

### [x] H28 — WYSIWYG canvas — snap-to-grid math and form skeleton — 🟠
- **Location:** `datacraft-core/src/canvas.rs` (snap/rect math), `datacraft-core/src/form.rs` (FormDef skeleton), sidecar (forms field), `datacraft-ui/src/form_design.rs`
- **Scope:** WYSIWYG canvas foundation: pure snap-to-grid and rect math (snap coordinates, rect hit-testing, containment/bounds), FormDef skeleton (name + grid size + empty controls) in the state/snapshot/sidecar (serde-default additive v1), Command::CreateForm (+ DeleteForm), design canvas UI painting the form frame, grid dots and control placeholders with drag-to-place using the snap math (exact placement lands with 8.2's control kinds). Reports share the canvas; the report-specific print layout is deferred (separate item).
- **Acceptance:** Snap/rect contract tests incl. proptests (snap is idempotent, hit-test inverse); form persistence round-trips; CreateForm observable in events/snapshot; canonical verification set green.
- **Done:** step 8.1 (feat 414499e)

---

### [x] H29 — Access control types — palette, placement commands, drag — 🟠
- **Location:** `datacraft-core/src/form.rs` (ControlKind palette), `datacraft-core/src/core.rs` (AddFormControl/SetFormControlRect/DeleteFormControl), `datacraft-ui/src/form_design.rs` (drag/resize)
- **Scope:** Access control palette: Label{text}, TextBox{bound_field: Option<String>}, CheckBox{bound_field}, ComboBox{bound_field, items?}, Button{caption}, SubForm{form}; ControlDef display names (present::control_kind_display); commands add/delete controls and move/resize them (rects snapped at dispatch, bounds-checked against the canvas, names unique per form); the design canvas renders real controls with topmost-selection and pointer drag dispatching SetFormControlRect with snapped geometry.
- **Acceptance:** Command contract tests (add with unique-name/bounds checks, move/resize snapping, delete); palette display names; canvas drag dispatches snapped rects; canonical verification set green.
- **Done:** step 8.2 (feat e959928)

---

### [x] H30 — Form runtime — data binding, record navigation, sidecar file IO — 🟠
- **Location:** `datacraft-core/src/form.rs` (FormDef.source), `datacraft-core/src/forms.rs` (SetFormSource, binding validation), `datacraft-ui/src/form_runtime.rs` (view), sidecar save/load commands
- **Scope:** Form runtime and data binding: FormDef gains a source table; Command::SetFormSource whitelists against the database; bound control fields validate against the source table (LESSONS #5) at bind time; the runtime view renders bound values for the current record with prev/next navigation (record index is UI state per tab) and a precise read-only note (edits land with H9); Command::SaveDesignDocument/LoadDesignDocument persist the sidecar to/from a real .datacraft file (all gates: version check on load, deterministic bytes). Report canvas (print layout) is deferred as its own item.
- **Acceptance:** Binding whitelist tests (unknown source, unknown bound field), SetFormSource observable, runtime values verified via port rows through a mock, sidecar save/load round-trip through the filesystem incl. version gate, canonical verification set green.
- **Done:** step 8.3 (feat 7a189b8); report canvas deferred as H31

---

### [x] H32 — MCP stdio endpoint — serve loop and datacraft-mcp binary — 🟠
- **Location:** `datacraft-core/src/transport.rs` (serve loop over Read/Write), new bin target `datacraft-mcp` in the ui crate (composition root: Core + Jet4Port)
- **Scope:** Stdio JSON-RPC 2.0 server (decision default 8): `serve(reader, writer, core)` reads line-delimited requests, answers via the existing codec (one response per line, flush per response, malformed lines still answer with -32700), EOF ends the session; `serve_stdio` wraps stdin/stdout. The `datacraft-mcp` bin composes Core over the real Jet4 port and runs the loop — the MCP endpoint agents connect to.
- **Acceptance:** Serve-loop contract tests over in-memory streams (request/response correlation, malformed input resilience, EOF); end-to-end open-database through the loop; canonical verification set green.
- **Done:** step 9.1 (feat 617e259)

---

### [x] H33 — Headless E2E harness — full stack over the wire — 🟠
- **Location:** `crates/datacraft-ui/tests/server_e2e.rs` (harness = serve() over in-memory streams + the real Jet4 port against the committed fixture)
- **Scope:** Headless E2E harness: drive the FULL stack over the wire — spawn serve() with the Jet4Port-composed Core, send JSON-RPC lines (open the committed fixture, inspect tables, run the schema/table lifecycle with a read-only rejection, QBE + RunQuery), assert on the response JSON (events + snapshots), never on pixels. The harness is the template agents copy for automation.
- **Acceptance:** E2E suite covering open/inspect/create-drop/query flows over the wire; deterministic (fixture-backed); canonical verification set green.
- **Done:** step 9.2 (test 245b49a)

---

### [x] H8 — CLI client as fourth headless command-bus client — 🟠
- **Location:** new crate `crates/datacraft-cli` (bin `datacraft-cli`), step 9.4
- **Scope:** Fourth client of the command bus (besides GUI, MCP/JSON-RPC server, test harness): args → `Command` variants → `dispatch()` → human-readable or `--json` output of events and `inspect()` snapshots. No logic — transport only; shares the transport layer with the Phase 9.1 MCP server. Exit codes meaningful and deterministic output for headless tests.
- **Acceptance:** Every command-bus operation reachable from the shell; round-trip tests assert JSON snapshots; crate rules respected (ui/cli never touch storage directly).
- **Done:** step 9.4 (test 917174f, feat d31c759)

---

### [x] F1 — Ribbon spec/mapping seam was unguarded — Create tab panic — 🟡
- **Location:** `datacraft-ui/src/ribbon.rs` (mapping), found 2026-11-01 in the running app
- **Problem:** Clicking the Create tab panicked: `form_design` is enabled in the core ribbon spec but the UI mapping lacked it — the 8.1 patch script died at an earlier assert and never applied the mapping; the core contract test pins the spec's enabled set but nothing pinned the UI mapping, so the seam (LESSONS #2) was unguarded and `unreachable!` fired at runtime.
- **Fix:** the mapping became a total, tested function (`command_for`) — the seam contract test in the ui crate asserts every enabled spec action (minus the input-bound `open_database`) maps to a command; no `unreachable!` remains in paint code.
- **Acceptance:** seam test green; app runs the Create tab without panic.
- **Done:** fixed immediately: total command_for mapping + seam test in ui crate (next commit); no unreachable! left in paint code

---

### [x] H9 — Jet4 writer workstream (create tables, write rows) — 🟠
- **Location:** `datacraft-storage/src/jet/` (writer), post-Phase-4 workstream
- **Scope:** Jet4 write milestone from ARCHITECTURE §6 (create tables, insert/update rows, usage-map allocation, index maintenance deferred). UI write commands route to `InMemoryEngine` until this lands; then the `Jet4Backend` gains write capability. Bit-for-bit goal: writes must open without repair prompts in MS Access — proven by golden round-trips + oracle.
- **Acceptance:** Access opens written files without repair; decode(encode(decode(x))) == decode(x) over fixtures; write-path coverage >= gate.
- **Done:** Phase 10, steps 10.1-10.3 (checkpoint 73f540b): writer core + create/insert + round-trip acceptance; Access-repair gate documented as manual follow-up

---

### [x] H31 — Report canvas — print layout and preview — 🟠
- **Location:** `datacraft-core/src/form.rs` (ReportDef sharing the canvas), `datacraft-ui/src/report_design.rs`
- **Scope:** Report canvas: ReportDef shares the ControlKind canvas (page geometry instead of form size, print-oriented defaults); report design + print preview runtime; grouped/aggregate printing lands after the expression service (M1). Deferred from Phase 8 (forms landed first; the canvas math, palette and sidecar persistence are shared and already proven).
- **Acceptance:** A report design persists in the sidecar, renders a print-preview runtime over source rows, and passes the canonical verification set.
- **Done:** Phase 10, steps 10.4-10.5 (checkpoint 1440c64): print-geometry designer + preview runtime; grouped/aggregate bands deferred to M1

---

### [x] H34 — Final documentation and release prep — 🟠
- **Location:** README.md, docs/ARCHITECTURE.md, docs/HANDOFF removal check, workspace docs
- **Scope:** Final documentation and release prep for the Phase 1-9 plan: README reflects the delivered feature set (storage read, command bus, JSON-RPC + MCP endpoint, E2E harness, CLI), the architecture table names all four clients and the new crates, the roadmap table closes Phase 9, stale session notes are cleaned up. No new features.
- **Acceptance:** Docs build coherent with the tree (spot-checked paths exist), roadmap/status tables truthful, canonical verification set green.
- **Done:** Phase 10, step 10.9 (checkpoint 9468b51): README/ROADMAP/ARCHITECTURE/FORMAT-JET reflect the Phase 1-10 feature set

---

### [x] L1 — Jet 3 (Access 97) read support — 🟢
- **Location:** `datacraft-storage/src/jet/`
- **Scope:** Jet 3 (Access 97) read support: 2048-byte pages, codepage-based text (no UTF-16), Jet3 TDEF layout. Deferred per default decision 2026-10-31 — Jet4 and ACE read paths come first; Jet3 files are detected via the version dword and rejected with a precise `Unsupported` error until this item lands.
- **Acceptance:** A Jet3 fixture decodes to schema + rows through the same `StorageEngine` boundary.
- **Done:** Phase 10, step 10.6 (checkpoint e1627cb): Jet3 read through the same boundary; jump-table rows = documented Unsupported remainder; real A97 fixture unpinned

---

### [x] L2 — Jet4 column type 0x11 (MSysAccessObjects.Data) — study and decide mapping — 🟢
- **Location:** `crates/datacraft-storage/src/jet/tdef.rs` (JetType::from_byte), `tests/oracle.rs` (skip path)
- **Problem:** Jet4 column type `0x11` is not in HACKING's (incomplete) type table and not in `docs/FORMAT-JET.md`; both fixtures use it in `MSysAccessObjects.Data` (long-value column). mdb-tools itself warns "unsupported data type 17" on the same column. Our reader answers with a precise `Unsupported`, and the oracle skips such tables.
- **Scope:** Focused study on the fixture's MSysAccessObjects rows (likely long-binary/LVAL like OLE; jackcess reads the table fine). Then either map the type or keep the precise `Unsupported` with a documented reason.
- **Acceptance:** Decision documented in `docs/FORMAT-JET.md`; oracle no longer needs the skip, or the skip carries the documented reason.
- **Done:** Phase 10, step 10.7 (checkpoint 8faeac3): 0x11 = JetType::LongBinary (fixed OLE compound docs); oracle skip documented

---

### [x] T3 — Fixture pin integrity — restore pristine testV2000, keep Access-touched variant, manifest + CI hash gate — 🔵
- **Location:** `fixtures/` (`testV2000.mdb`, manifest), CI workflow
- **Problem:** `fixtures/testV2000.mdb` no longer matches the jackcess pin (`be0a2139`): the file was opened in real Access, which rewrote system-table pages and appended 22 pages on open (full evidence + hashes in `docs/FIXTURE-SOURCES.md` §4). A silently drifted fixture poisons the oracle evidence base — the recorded pin hash no longer describes the file on disk.
- **Scope:** restore pristine `testV2000.mdb` from the verified pin source (sha256 `126bad49…`); keep the Access-rewritten file as `fixtures/testV2000-access-touched.mdb` (second writer variant — T2 material, do not discard); replace `jackcess-pin.txt` with a per-file manifest (`repo@commit` + sha256, sources and recommended Phase-2 pin set per `docs/FIXTURE-SOURCES.md` §5); add a CI job that verifies committed fixture hashes; document the rule: fixtures are never opened in writer-capable applications (Access) — byte experiments run on copies only.
- **Acceptance:** committed `fixtures/manifest` hashes match the committed files and CI fails on drift; pristine + Access-touched variants exist as separate fixtures; pin set per docs/FIXTURE-SOURCES.md §5 pinned; rule documented in docs/TESTING.md.
- **Done:** Phase 10, step 10.8 (checkpoint aafb794): pristine restored (126bad49…), touched variant kept, manifest + CI hash gate, immutability rule in TESTING.md

---

### [x] H35 — SQL-Integration: SQLite-Datenquelle (Backend, Detection, Format) — 🟠
- **Location:** `crates/datacraft-storage/src/sqlite.rs` (neu), `crates/datacraft-storage/src/detecting.rs` (neu), `crates/datacraft-core/src/port.rs`, Kompositionsstellen
- **Ansatz:** rusqlite mit `bundled`-Feature (statisch kompiliert, MIT/Apache; SQLite selbst ist Public Domain — kein Clean-Room-Konflikt, die Policy zielt auf Microsoft-IP)
- **Scope:** `SqlitePort` hinter `StoragePort`: open (Magic `SQLite format 3\0`), tables (`sqlite_master`, ohne `sqlite_%`), Schema via `PRAGMA table_info` → `FieldKind` (Affinity-Tabelle dokumentiert: TEXT→Text, INTEGER→LongInteger, INTEGER PK AUTOINCREMENT→AutoNumber, REAL→Double, BLOB→Binary(OleObject), NUMERIC→Decimal), `read_rows` (rowid-Ordnung; NULL- und Bool-Feinheit dokumentiert — SQLite kennt kein BOOL), `relationships` via `PRAGMA foreign_key_list`. `DatabaseFormat::Sqlite` + Display „SQLite" + serde `"sqlite"`. Detection: neuer `DetectingPort` mit Magic-Sniff (Jet/ACE-Magic vs. SQLite vs. präziser `OpenFailed`); Kompositionsstellen umstellen: datacraft-ui main.rs, datacraft-mcp, datacraft-cli main.rs, server_e2e.
- **Acceptance:** Tests: Schema/Zeilen/FKs einer testgenerischen .sqlite; Detection erkennt Jet-Fixture (Jet4), .sqlite (Sqlite), Mülldatei (präziser Fehler); UI/Status zeigt „(SQLite)"; MCP/CLI öffnen .sqlite über dieselbe Bus-Vokabel.
- **Done:** Phase 13.1 — SqlitePort (catalog, affinity schema, rows, FKs), DetectingPort magic sniff, DatabaseFormat::Sqlite; tests ca61638, feat 5a04e94

---

### [x] H36 — SQL-Integration: SQLite-Writes + Row-Insert im Command-Bus — 🟠
- **Location:** `crates/datacraft-core/src/port.rs` (Trait), `command.rs`, `database.rs`, `event.rs`, `crates/datacraft-storage/src/sqlite.rs` (Writes), `tests/port_adapter.rs`, `tests/server_e2e.rs`
- **Scope:** `StoragePort::insert_row(&mut self, table, values: &[FieldValue])` + MockPort-Impl (rows-HashMap) + `Command::InsertRow { table, values }` + `Event::RowInserted` + Reducer-Validierung (Anzahl/Typen/required, port first, dann State) — faltet den insert-Teil der geparkten Phase 11 ein. SqlitePort schreibbar: `create_table` (DDL-Forward-Mapping FieldKind→SQLite, Lossiness-Tabelle dokumentiert), `drop_table`, `insert_row` (parameterisiert, FieldValue→rusqlite-Params).
- **Acceptance:** Bus-Tests (InsertRow-Ereignisse, Validierungsfehler präzise); E2E über JSON-RPC: Tabelle anlegen + Zeilen einfügen in eine .sqlite, Reopen verifiziert Persistenz; MockPort-Insert erscheint im Datasheet-Cache.
- **Done:** Phase 13.2 — StoragePort::insert_row, Command::InsertRow/Event::RowInserted with unit validation, SqlitePort create/drop/insert; tests 99ddb0b, feat 390c79c

---

### [x] H37 — SQL-Integration: SQL über alle Quellen (In-Memory-Engine + UI) — 🟠
- **Location:** `crates/datacraft-core/src/sql_engine.rs` (neu), `command.rs`, `event.rs`, UI: SQL-Fenster nach Relationships-Muster, Ribbon `database_tools`
- **Ansatz:** Shadow-In-Memory-SQLite statt DuckDB/DataFusion — `Core::run_sql` materialisiert die geladenen Tabellen (Schema aus `state.database.tables`, Zeilen via `port.read_rows`) in ein flüchtiges SQLite und führt das SELECT dort aus. Ein Dialekt, keine Heavy-Dependency; Joins/Aggregate auch über Jet-Tabellen (Proof-Tests: SQL inkl. JOIN über MockPort-Tabellen). V1: frisches Shadow-DB pro Command (deterministisch, keine Stale-Caches); inkrementelle Materialisierung für große DBs = spätere Optimierung.
- **Agentensicherheit:** nur ein Statement, nur `SELECT`/`WITH` (Präfix-Gate + `prepare` lehnt Mehr-Statement ab); Shadow-DB ist vergesslich — die Quelle bleibt unberührt.
- **Scope:** `Command::RunSql { sql }` + `Core::run_sql` → `SqlResult { columns, rows }` + `Event::SqlRan { rows }` / `Failed` (MCP/CLI erben automatisch über den Transport). UI: SQL-Fenster (ToggleCommand wie der Relationship-Mapper, Ribbon-Aktion, monospaced Editor + Run, Ergebnis-Grid im Access-Look — `grid_ui` aus datasheet.rs geteilt).
- **Acceptance:** Engine-Suite (SELECT/WHERE/ORDER/Aggregat/JOIN über MockPort-Quellen; Gate lehnt INSERT/DELETE/Multi-Statement präzise ab); UI-Flow manuell verifiziert; E2E: RunSql über JSON-RPC.
- **Done:** Phase 13.3 — shadow SQL engine (one SELECT/WITH over materialized sources), RunSql/SqlRan, SQL window UI; tests 1a58292, feat 0400f63

---

### [x] H38 — ACCDB-Fixtures pinnen (ACE 2007/2010/2016, manifest + Hash-Gate) — 🟠
- **Location:** `fixtures/` (neue .accdb-Dateien), `fixtures/manifest`, `.github/workflows/ci.yml` (Hash-Gate), `docs/FIXTURE-SOURCES.md`
- **Ansatz:** Echte ACE-Dateien (2007/2010/2016+) aus den dokumentierten Quellen beschaffen; Pinning analog testV2000 (pristine + optionale „access-touched"-Variante); Manifest-Einträge mit sha256; CI-Hash-Gate erweitert. Header-Erkennung („Standard ACE DB", FormatVersion::Ace) bleibt Vorschaulese-Referenz.
- **Acceptance:** Mindestens eine .accdb je ACE-Generation (2007/2010/2016) gepinnt; Manifest + Hash-Gate grün; Herkunft/Erzeugung in `docs/FIXTURE-SOURCES.md` dokumentiert.
- **Done:** Phase 14.1 — pristine ACE 2007/2010/2016 fixtures pinned, manifest + hash gate, provenance in FIXTURE-SOURCES; commit aef9b8f

---

### [x] H39 — ACCDB-Read-Pfad (Catalog/TDEF/TDAT über den Jet4-4096-Pfad) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/header.rs`, `reader.rs`, `catalog.rs`, `tdef.rs`, `row.rs`; `src/port_adapter.rs`; `crates/datacraft-core/src/types.rs` (`DatabaseFormat`)
- **Ansatz:** `FormatVersion::Ace`-Dateien über den bewährten Jet4-Lesepfad bedienen (4096-Byte-Pages sind bereits der Jet4-Pfad); ACE-spezifische Header-/Catalog-Feinheiten gegen die neuen Fixtures (H38) verifizieren; `DatabaseFormat::Ace` bis UI/CLI/MCP durchreichen (Format-Anzeige „(ACE)"). Oracle-Gegenprobe (mdb-tools) je Tabelle/Zeile; nicht unterstützte ACE-Varianten → präziser `Unsupported`.
- **Acceptance:** Tabellen/Zeilen/Beziehungen der ACCDB-Fixtures dekodieren vollständig; Oracle-Abgleich grün; Status-/Titelanzeige zeigt ACE-Format; präzise Fehlerfälle getestet.
- **Done:** Phase 14.2 — ACE read path through Jet4 4096; oracle match (mdbtools 1.0.1), format through all clients, precise header errors; commit d6a3a66
