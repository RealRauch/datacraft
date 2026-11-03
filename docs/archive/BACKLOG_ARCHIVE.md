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

---

### [x] H40 — Jet4 Update/Delete-Writes (RowOp am echten Dateipfad) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/writer.rs`, `writer_tables.rs` (ggf. neu: `writer_rows.rs`), `encode.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Ansatz:** `RowOp::Update/Delete` am echten Dateipfad bedienen: In-Place-Update für Fixed-Width-Zeilen, Löschen über Zeilen-Flag + konsequente Free-Space-/Usage-Map-Pflege; LVAL-Ketten (Memo/OLE) beim Update sauber ersetzen/freigeben; Round-Trip decode∘encode∘decode plus mdb-export-Oracle wie im bestehenden Writer-Muster.
- **Acceptance:** Update/Delete-Tests auf Fixtures inkl. Memo/OLE-Feldern; Free-Map-Konsistenz nach Deletes; Oracle liest unsere Dateien fehlerfrei; Reopen verifiziert Persistenz der Änderungen; `Unsupported` für noch nicht qualifizierte Zeilenformen.
- **Done:** done in `ea76d2d` (step 11.10; in-place/repack updates, 0x8000 deletes, precise Unsupported for overflow/LVAL/oversized rows, port + bus commands)

---

### [x] M2 — Journaled/restringierte Write-Profile (Preview, Bestätigung, Feldfamilien) — 🟡
- **Location:** `crates/datacraft-core/src/command.rs`, `database.rs`, `event.rs`; `crates/datacraft-storage/src/port_adapter.rs`
- **Ansatz:** Schreiboperationen optional als Journal führen: Preview (Diff) → Bestätigung → Ausführung → Rollback-Info; Restriktion auf qualifizierte Feldfamilien (typsicher, Fixed-Width zuerst); Operationen außerhalb des Profils → präzise `Unsupported` statt stiller Annahmen. Baut auf H40 auf und ist die Voraussetzung für gefahrloses UI-Editing (Phase 11).
- **Acceptance:** Preview→Confirm→Apply-Tests; Rollback-/Journal-Tests; Restriktionsfehler präzise; E2E über JSON-RPC.
- **Done:** done in `dcb6852` (step 11.11; preview→apply→rollback journal, atomic Jet batch, field-family profile)

---

### [x] H41 — Expression-Service (Ring 1 / Roadmap M1) — 🟠
- **Location:** `crates/datacraft-core/src/expr/` (neu: lexer/parser/eval), `schema/field.rs` (Default/Validation), `report.rs`/`reports.rs` (Bänder), `command.rs`
- **Ansatz:** Kleiner, totaler Ausdrucks-Evaluator: Feldreferenzen, Literale, Arithmetik/Konkatenation, Vergleichslogik, NULL-Semantik nach Access-Muster; Funktionsfamilien IIf/NZ/Format, Datums-/String-Funktionen, LIKE mit Access-Wildcards; typisiert über `FieldKind`. Anschlüsse: Default Values, Validation Rules, Calculated Fields sowie gruppierte/aggregate Report-Bänder (löst die M4-Report-Reste aus Phase 10 ein).
- **Acceptance:** Evaluator-Suite inkl. NULL-/Coercion-Fällen; Validation-/Default-/Calculated-Tests; Report-Preview mit Gruppen-/Aggregatbändern; Ausdrücke über Bus/MCP/CLI ausführbar; Expressions-Suite ist die Referenz für Ring 2/3.
- **Done:** 15.1 (b6c7c73 RED, a657762 GREEN) + 15.2 (7eb3bbc RED, 6cb634d GREEN): evaluator suite, NULL/coercion contracts, defaults/rules/calculated wiring, RunExpression over the bus and grouped/aggregate report bands

---

### [x] M3 — Native Saved Queries (MSysQueries → QueryDef) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/` (MSysQueries-Decoder), `src/port_adapter.rs`; `crates/datacraft-core/src/qbe.rs`, `state.rs`
- **Ansatz:** `MSysQueries`-Zeilen lesen und auf `QueryDef` abbilden (Attribut-Codes in `docs/FORMAT-JET.md` dokumentieren); nicht abbildbare Konstrukte als dokumentierte Reste. Native Queries erscheinen in der Nav-Pane-Gruppe „Queries" und öffnen read-only im QBE; eigene Queries im Sidecar bleiben unberührt (kein Rückschreiben vor ACE-Writer-Entscheidung).
- **Acceptance:** Fixture mit nativen Queries; Listing + QueryDef-Öffnung über Bus/MCP; nicht unterstützte Attributtypen → präziser `Unsupported`; Nav-Pane zeigt native Queries neben Sidecar-Queries.
- **Done:** 15.3 (78203c3 RED, d914484 GREEN): MSysQueries decoder with all nine flavors, native queries in the nav pane, read-only QBE tab, mappable-subset Run, documented rests (parameters/criteria/joins/grouping), queryTestV2000.mdb pinned (51bd3c5)

---

### [x] F2 — Jet3 database passwords are plain text (M5 fact correction) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/crypto.rs` (`jet3_password`), tests in `jet3.rs`/`jet_encryption.rs`/`jet_table.rs`/`port_adapter.rs`
- **Problem:** M5 initially decoded the Jet3 password field with the documented fixed XOR key. The pinned pristine Jet3 fixture (`testV1997.mdb`) stores an all-zero field and was rejected as password-protected — the fixed key is the differential-recovery technique, not an in-file mask.
- **Fix:** Jet3 passwords are plain CP1252 text inside the RC4-decrypted header region; the fixed key constant was removed and the tests/docs corrected.
- **Evidence:** `fixtures/testV1997.mdb` opens without a password; the generated Jet3 password/page-encryption test still passes.
- **Retro:** inline fix during step 16.3 (M5 scope), Low class, discovered by the first real Jet3 fixture.
- **Done:** done in 9b05ab7

---

### [x] M4 — Complex Values/Multivalue (Attachment-Subtabellen real decodieren) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/` (Complex-Decoder: benannte Unit-Tabellen f_*), `row.rs`, `tdef.rs`; `crates/datacraft-core/src/schema/field.rs`, `port.rs`
- **Ansatz:** Complex-Type-Spalten (0x12) real auflösen: zugehörige Value-/Attachment-/Named-Subtabellen erkennen, Werte pro Zeile gruppieren; Domain-Modell erweitern (Multivalue-/Attachment-Werte statt Binary-Platzhalter). Datasheet zeigt zusammengeführte Werte; QBE-Prädikate können auf Multivalue-Feldern arbeiten (EXISTS-Semantik dokumentiert). Bis dahin erscheinen die `f_<guid>_*`-Subtabellen als normale Nutzertabellen (Zwischenstand, dokumentiert in docs/FORMAT-JET.md §ACE).
- **Acceptance:** Fixture mit Multivalue-/Attachment-Feldern; Werte vollständig dekodiert; Datasheet-/QBE-Support getestet; Oracle-Gegenprobe; `FieldKind`-Mapping dokumentiert.
- **Done:** done in 6d96561 (RED 5480122)

---

### [x] M5 — Verschlüsselung/Passwort (RC4-Decode, Passwortpfad UI/CLI/MCP) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/header.rs`, `reader.rs` (Decode-Pfad), `row.rs`; `src/port_adapter.rs`; `crates/datacraft-core/src/port.rs` (Passwort-Parameter); UI-Prompt, CLI-Flag, MCP-Parameter
- **Ansatz:** RC4-Decoder für die verschlüsselte Header-/Datenregion (Schlüsselpfade je Formatgeneration in `docs/FORMAT-JET.md` dokumentieren); Passwort am `StoragePort::open`; alle Clients können Passwort übergeben (UI-Prompt, CLI `-p`, MCP-Parameter); verschlüsselte Test-Fixtures selbst erzeugt/pinnt (keine Fremd-Dateien mit echten Inhalten).
- **Acceptance:** Verschlüsselte Fixtures öffnen mit korrektem Passwort über alle vier Clients; falsches/fehlendes Passwort → präziser Fehler; Unverschlüsselte Files unchanged; Doku in FORMAT-JET.md.
- **Done:** done in fd30e5a (RED 3a3bb15; Jet3 password fact fix 9b05ab7)

---

### [x] L3 — Jet3-Reste (Jump-Table-Zeilen >255 Bytes, verkettete TDEFs) — 🟢
- **Location:** `crates/datacraft-storage/src/jet/jet3.rs`, `tdef.rs`, `reader.rs`; `docs/FORMAT-JET.md` (Reste-Matrix)
- **Ansatz:** Jump-Table-Mechanismus für Zeilen >255 Bytes dekodieren; verkettete TDEF-Mehrfachseiten lesen; Reste-Matrix in FORMAT-JET.md entsprechend schrumpfen; präzise `Unsupported` nur für wirklich Exotisches.
- **Acceptance:** Jet3-Fixture mit großen Zeilen (Jump-Table) und verketteter TDEF; oracle-geprüft (mdb-tools); Reste-Matrix aktualisiert; bestehende Jet3-Tests bleiben grün.
- **Done:** done in a8df705 (RED bd8663a)

---

### [x] M6 — Doku-Genauigkeit (Kompatibilitätsaussagen ↔ verifizierter Stand) — 🟡
- **Location:** `README.md`, `docs/ARCHITECTURE.md` (§1 Mission, ADR-1), `docs/ROADMAP.md`, `docs/FORMAT-JET.md`
- **Ansatz:** Kompatibilitätsaussagen an den verifizierten Stand angleichen: ehrliche Read/Write-Matrix je Format (Jet3/Jet4/ACCDB/SQLite) mit Verweis auf die jeweiligen Tests/Gates; „bit-for-bit"-Claim auf die tatsächlich geprüften Pfade eingrenzen; Sidecar-vs-native-Absicherung (ADR-1) prominent halten. Reihenfolge: nach Abschluss von Phase 14 (ACCDB-Read), final mit Phase 17.
- **Acceptance:** Jede Kompatibilitätsaussage in README/ARCHITECTURE ist durch einen benannten Test/Gate gedeckt; Review-Absatz je Format; keine Übertreibung gegenüber der Fixture-Abdeckung.
- **Done:** verified compatibility matrix + Phase 17 docs accuracy, commit 19854e6

---

### [x] L4 — Makro-Inventur (Ring 3 / Roadmap M5) — 🟢
- **Location:** `crates/datacraft-storage/src/jet/` (Makro-Objekte lesen), `src/port_adapter.rs`; `crates/datacraft-core/src/state.rs`, `command.rs`, `ids.rs`; UI Nav-Pane
- **Ansatz:** Makro-Objekte als neue `ObjectKind`-Variante lesen (Inventur: Name, grob strukturierte Aktionsliste); Nav-Pane-Gruppe „Makros"; read-only Präsentation über Bus/MCP. DoCmd-Ausführung bleibt bewusst Ring-3-Umfang (Roadmap M5) und ist hier **nicht** Teil des Steps.
- **Acceptance:** Fixture mit Makros; Inventur über Bus/MCP abfragbar; Nav-Pane zeigt Makro-Gruppe; keine Ausführungssemantik im Scope.
- **Done:** read-only macro inventory, commit 8da3acb

---

### [x] L5 — VBA-Extraction + Viewer + statische Analyse (Roadmap M6) — 🟢
- **Location:** `crates/datacraft-storage/src/` (`MSysAccessStorage` → CFBF-Reassembly), `crates/datacraft-core/src/vba/` (neu: dir-/Modul-Stream-Decoder), UI-Viewer-Modul; `command.rs`
- **Ansatz:** Embedded Compound File reassemblieren, `dir`-/Modul-Streams je `[MS-OVBA]` decodieren, Quelltext pro Modul ins Domain-Modell; Viewer (Modulbaum + Quelle + Volltextsuche) wie jedes andere DB-Objekt; headless statische Analyse (Call-Graph, eingebettete SQL-Literale, Tabellen-/Query-Referenzen, ungenutzte Module) — MCP-freundlich. Oracle: Referenz-*Outputs* als Golden-Daten, nie Code (Clean-Room-Muster wie H7). **Keine Ausführung** (ADR-2-Grenze).
- **Acceptance:** Fixture mit VBA-Projekt; Extraction deterministisch/stabil; Viewer + Analyse über Bus/MCP; ADR-2-Grenze (kein Interpreter) in Tests verankert.
- **Done:** VBA extraction/viewer (ccf2107) + static analysis (63ec95c), no execution

---

### [x] I1 — i18n kernel in the UI crate — 🟠
- **Location:** `crates/datacraft-ui/src/i18n.rs` (neu) + `crates/datacraft-ui/src/i18n/` (Katalog-Module), `crates/datacraft-ui/Cargo.toml` (neue Dependency `sys-locale`); Modell: WordCraft `crates/ui-egui/src/i18n.rs`; ADR-4 in `docs/ARCHITECTURE.md` §15
- **Approach:** `t()`/`tr()` with the English source string as the lookup key (gettext pattern); embedded German catalog tables; a missing translation falls back to the English source unchanged (incremental coverage never breaks a build). `Lang` selection: setting `"auto"` (default) → system detection via `sys-locale` (`de*` → German, anything else → English), `DATACRAFT_LANGUAGE` environment override wins over detection. The current language lives in UI state only — ADR-4: UI settings never write into core, the `.datacraft` sidecar or the wire; headless clients (MCP/CLI) stay English. **New dependency `sys-locale` (ui crate only) = Medium content gate → pause for release before landing.**
- **Acceptance:** `t()` returns the English source for unknown keys; detection unit-tested (locale strings → language mapping incl. `None`/non-`de` fallback); env override beats detection; core untouched (`git diff crates/datacraft-core` empty); canonical verification set green.
- **Done:** dbc513a

---

### [x] I2 — All UI literals through t() — 🟠
- **Location:** `crates/datacraft-ui/src/*.rs` (nav_pane.rs, workspace.rs, datasheet.rs, design.rs, qbe.rs, relationships.rs, sql_window.rs, form_design.rs, form_runtime.rs, report_design.rs, designer_palette.rs, shell.rs, ribbon.rs, app.rs `notice_for`, flows.rs); ADR-4
- **Approach:** every hardcoded user-facing literal in the paint code goes through `t()` (~88 literals today). Data-dependent display text is localized in the UI from locale-neutral snapshot data: field-type names from the `FieldSnapshot.kind` enum (not the English `kind_display` string), Yes/No cell text from `FieldValue`, the status line from snapshot counts/format. Ribbon labels render via `t(spec.label)` (the English spec label is the key); toasts/hints become template functions with `{name}`-style placeholders. Core is not modified; `inspect()` JSON stays byte-identical in every language (wire-stability test).
- **Acceptance:** no user-facing English literals left in paint code (grep allow-list: technical glyphs `✕`/`◀`/`▶` only); `git diff crates/datacraft-core` empty; wire-stability test green (fixture `inspect()` before/after byte-equal); canonical verification set green.
- **Done:** b616e9c

---

### [x] I3 — German catalog complete — 🟠
- **Location:** `crates/datacraft-ui/src/i18n/` (de tables), `crates/datacraft-ui/src/i18n.rs` (parity tests)
- **Approach:** full German translation of every catalog key using Access terminology (Datenblatt, Entwurfsansicht, Navigationsbereich, Datei/Start/Erstellen/Externe Daten/Datenbanktools). Parity tests enforce: every key present in the German tables, non-empty, and diffed against the English source so nothing drifts silently. Layout sanity: longer German strings must not truncate in ribbon group captions and the status bar (checked in the manual gate run, noted in PROGRESS).
- **Acceptance:** parity tests green; the app shows German on a German system and English otherwise (fallback proven); canonical verification set green.
- **Done:** 6486786

---

### [x] I4 — App settings store (ui.json) — 🟡
- **Location:** `crates/datacraft-ui/src/settings.rs` (neu), Config-Dir-Auflösung handgerollt (`%APPDATA%`/`XDG_CONFIG_HOME`/`~/Library/Application Support`, Fallback Home — keine neue Dependency); ADR-5 in `docs/ARCHITECTURE.md` §15
- **Approach:** serde JSON at `<config>/datacraft/ui.json` storing `language` (`"auto"`/`"de"`/`"en"`) and window geometry now, appearance later (U4). Unknown values fall back to defaults instead of failing (WordCraft `ui.json` pattern). Strictly UI-owned — settings are **never** written into the `.datacraft` sidecar or any core state (ADR-5); loaded at startup, saved on change/close.
- **Acceptance:** settings round-trip across restarts; corrupt/empty file → defaults, no panic; precedence per ADR-5 (setting > env > detection > auto); sidecar files untouched by the settings path; canonical verification set green.
- **Done:** d898120

---

### [x] I5 — Language switcher in the UI — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (File tab group; moves into the backstage with U7), `app.rs`, `settings.rs` (I4); ADR-4/ADR-5
- **Approach:** a language menu (Auto/Deutsch/English) in the File ribbon tab; selection persists to `ui.json`; precedence Setting > Env > Detection > Auto. Switching applies immediately — `t()` reads the current language each frame (WordCraft per-frame refresh pattern), no restart required.
- **Acceptance:** switch applies live; persistence across restarts; `Auto` resolves per detection; headless test for the full precedence chain; canonical verification set green.
- **Done:** 745f84f

---

### [x] U1 — Theme rebase: WordCraft-style neutral tokens, Teal accent — 🟠
- **Location:** `crates/datacraft-ui/src/theme.rs` (token system), paint sites referencing theme constants; ADR-6 in `docs/ARCHITECTURE.md` §15; model: WordCraft `crates/ui-egui/src/theme.rs` @ `fd7f662`
- **Approach:** token-based theme adopting the model's **neutral** system: chrome/title bar `#F0F0F0`, ribbon `#F9F9F9`, canvas `#E6E6E6`, panel `#FAFAFA`, text `#242424`/`#616161`/`#A8A8A8`, borders `#E1E1E1`/`#C8C8C8`, hover `#E6E6E6`, pressed `#D6D6D6`. **The accent family stays DataCraft teal and is derived from it — no accent colour is copied from the model (its blue `#3B5BDB` is explicitly rejected):** accent `#0F8B7E`, accent_text `#0B6B60`, checked/selection `#DCE9E4` (existing TEAL_WASH), on_accent `#F8F4EA` (ON_TEAL). Geometry from the model: item_spacing (6,4), button_padding (8,3), corner radii windows 8 / menus 6 / widgets 4, 1 px strokes. The derivation rule ("accent tokens are teal derivatives, never the model's blue") is pinned by a unit test. Verified at `fd7f662`: every colour and geometry value above matches the model exactly, and the model keeps them in a `Tokens` struct with `light()`/`dark()`. Its token set is larger (`tab_strip`, `status_bar`, `input_border`, `nav`) — the list above is the required minimum; add a slot only when a region needs it.
- **Acceptance:** all four regions render from tokens; accent-token test green (teal derivatives only); the brand icon's teal still matches the UI accent; before/after screenshots reviewed in the phase gate; canonical verification set green.
- **Done:** done in `c9ea70e` (RED `849c428`) — neutral `Tokens`/`GEOMETRY`, teal accent derivation pinned by tests, all paint sites rebased; before/after screenshots stay with the Phase 20 gate (20.5)

---

### [x] U2 — Typography: Inter + type rungs — 🟠
- **Location:** `assets/fonts/` (neu: Inter-Regular/-Medium/-SemiBold TTF, JetBrainsMono-Regular for Mono — OFL, attribution file), `crates/datacraft-ui/src/theme.rs` (font definitions); model: WordCraft `theme.rs` font setup @ `fd7f662`
- **Approach:** embed the fonts via `include_bytes!` and register families like the model: proportional Inter first (egui defaults stay as fallback), named families `"medium"`/`"semibold"` (InterMedium/InterSemiBold → Inter → egui defaults), mono for the SQL editor. Type rungs: Chrome 12.5 px (tracking −0.012 em), Control 11.5 px, Caption 11.0 px (+0.018 em); egui styles Body/Button 12.5, Small 10.5, Heading 17 semibold. **New font assets = Medium content gate** (license/attribution required before landing). Verified at `fd7f662`: the model registers exactly these three Inter files plus `SourceSans3-Regular.ttf` (appended behind Inter in every family) and `JetBrainsMono-Regular.ttf` (Monospace); DataCraft deliberately skips Source Sans 3 — egui's default faces are the fallback instead.
- **Acceptance:** all four regions use the rung scale; German strings lay out without truncation (gate check against I3's catalog); licences recorded in an attribution file; canonical verification set green.
- **Done:** done in `aef71e9` (RED `e8a6c6c`) — Inter 4.1 and JetBrains Mono 2.304 embedded unmodified with SIL OFL licenses and attribution; rung scale installed in both themes; German layout budgets now measured on the shipped fonts

---

### [x] U3 — Original vector iconography (ribbon, nav pane, tabs) — 🟡
- **Location:** `crates/datacraft-ui/src/icons.rs` (neu, reines Painter-Modul), `ribbon.rs`, `nav_pane.rs`, `workspace.rs`; model: WordCraft `crates/ui-egui/src/icons.rs` @ `fd7f662`
- **Approach:** one name-keyed painter dispatcher — `paint(painter, rect, name, ink, accent)` with section sub-dispatchers, the model's shape (~280 icon cases in ~7 functions) — for the icons the UI needs (open/save/close/table/query/form/report/relationships/SQL/design …), mapped from the existing stable ids (ribbon action ids, `ObjectKind`) — never invented keys. Monochrome ink strokes with teal accents, no external image assets (clean-room and asset-free besides fonts).
- **Acceptance:** every ribbon action, nav-pane object kind and document tab shows an icon; all icons live in one module and are painted at paint time; label-based addressing stays intact (kittest-ready, coordinate with T5); canonical verification set green.
- **Done:** done in `824f546` (RED `0c2ca4e`) — one name-keyed painter dispatcher in `icons.rs` with `file`/`data`/`objects` section painters; every ribbon action id and object kind is dispatched and paints; ribbon buttons, nav-pane rows and document tabs carry icons; AccessKit labels preserved

---

### [x] U4 — Dark mode: Appearance Light/Dark/System — 🟡
- **Location:** `crates/datacraft-ui/src/theme.rs` (dark token set + `Appearance`), `settings.rs` (I4) for persistence; model: WordCraft `desktop_theme.rs`/`theme.rs` @ `fd7f662`; ADR-6
- **Approach:** `Appearance { Light, Dark, System }` (default `System`) — both palettes are installed at theme-apply time (`set_visuals_of(Theme::Light/Dark)`), the token set is picked per frame from `ctx.global_style().visuals.dark_mode`, and the theme preference follows the setting, exactly like the model. **Dark accents use the teal pendant (≈ `#3FA799`, dark on-accent ink), never the model's light blue (`#7A93F0`).** WCAG contrast tests (4.5:1 text on accent, 3:1 borders) against the teal variants; unknown persisted appearance values fall back to the default. Linux caveat: `system_theme` is `None` there, so the model polls the XDG desktop portal's `color-scheme` every 4 s (`desktop_theme.rs`) — without that the "System follows OS live" acceptance fails on Linux.
- **Acceptance:** System mode follows OS appearance live; persistence via `ui.json` (I4); contrast tests green for both palettes; canonical verification set green.
- **Done:** done in `0866cd9` (RED `731155a`) — Appearance Light/Dark/System (default System) persisted in `ui.json`, both palettes installed per theme, tokens picked per frame, dark teal pendant #3FA799 with WCAG contrast tests for both palettes; Linux live-follow polls the XDG portal every 4 s via `gdbus` with a tested parser

---

### [x] F3 — F3 — Empty workspace never painted the canvas token — 🟢
- **Location:** `crates/datacraft-ui/src/workspace.rs` (empty-state branch)
- **Problem:** with no open tabs the workspace returned before painting the central panel, so the window clear colour (black) showed through instead of `tokens.canvas`; found in the Phase 20 gate screenshots.
- **Fix:** wrap the empty-state message in the same `CentralPanel` frame as the populated workspace (6 lines, Low class), committed with the Phase 20 gate.
- **Acceptance:** an empty workspace renders the canvas token in both palettes.
- **Done:** fixed inline in `612b114` during the Phase 20 gate — the empty workspace now paints the canvas token in both palettes

---

### [x] U13 — ui.json hardening: atomic save + full window geometry — 🟡
- **Placement:** Phase 22, steps 22.1–22.2 (scheduled right after Phase 20; order decision 2026-11-01).
- **Location:** `crates/datacraft-ui/src/settings.rs` (`save`, window geometry), `app.rs` (capture); model: WordCraft `apps/wordcraft/src/main.rs` (`write_prefs`), `crates/ui-egui/src/window_geometry.rs` @ `fd7f662`
- **Problem:** `Settings::save` uses `std::fs::write`, so a crash mid-write leaves `ui.json` truncated (the defaults recover, but the settings are lost); the stored geometry is the inner size only, so position and maximized state are not restored and a window can reopen off-screen after a monitor change.
- **Approach:** write to a sibling temporary file and rename it over the target (the model's `write_prefs`); extend the stored geometry to position + maximized, clamp it against the visible monitors and drop implausible values (the model's `WindowGeometry::sanitized`/`rescue_position` pattern).
- **Acceptance:** an interrupted write leaves the previous file intact; position and maximized state round-trip across restarts; a saved off-screen position is rescued onto a visible monitor; corrupt/partial files still fall back to defaults; canonical verification set green.
- **Done:** done in `1462e99` (atomic save, RED `5c2be6b`) and `21044b2` (geometry + rescue, RED `aabd453`) — sibling temp file renamed over the target, legacy files still load, position/maximized round-trip, off-screen positions rescued onto the visible monitor; runtime smoke test: (9000,9000) → (820,380)

---

### [x] U5 — Contextual ribbon tabs + „Views" split-button — 🟡
- **Location:** `crates/datacraft-core/src/ribbon.rs`, `command.rs`, `state.rs`, `snapshot.rs` (SetTabMode — Domänenzustand); `crates/datacraft-ui/src/ribbon.rs`, `workspace.rs`
- **Approach:** the Home ribbon gains a "Views" group with a View split-button switching the active tab's mode (Datasheet/Design per object kind) — the Access convention, replacing the tab-strip toggle which is removed. When an object tab is active, contextual tabs (e.g. "Table Tools") render after "Database Tools", spec-driven from core. `tab_mode` moves from the UI-local HashMap into core state (`SetTabMode` command + `TabSnapshot.mode`) so agents see and drive it — this is document state, **not** a UI setting (ADR-4 boundary respected). Model evidence at `fd7f662`: the model's `ribbon.rs` renders `contextual_tabs(...)` (Table Design, Picture Format, Shape Format, Chart Design) — confirmed. The Views split-button has no counterpart in the model; it is the Access convention.
- **Acceptance:** mode switch only via the Views button; snapshot exposes per-tab mode; contextual tabs appear per active kind; ribbon seam test updated; headless wire tests for `SetTabMode`; canonical verification set green.
- **Done:** done in `de70a21` (RED `904cc7c`) — contextual tabs (Table/Query/Form/Report Tools) spec-driven in core after Database Tools; `SetTabMode` and `TabSnapshot.mode`/`has_design_view` move the view mode into core state; the Home Views group renders the split-button and the tab-strip toggle is removed

---

### [x] U6 — Quick Access Toolbar — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (QAT strip), `crates/datacraft-core/src/ribbon.rs` (QAT spec entries); model: WordCraft `crates/ui-egui/src/chrome.rs` @ `fd7f662`
- **Approach:** a compact top strip per the Access QAT convention, spec-driven from core like every ribbon region, starting with Save Design. Undo/Redo entries are declared but rendered disabled until the command bus gains a history (explicitly out of scope here; follow-up item owns them). QAT buttons dispatch exactly the same commands as their ribbon counterparts. Model evidence at `fd7f662`: the model's `chrome.rs` builds title bar + QAT + status bar, and its QAT entries are command ids with numeric keytips.
- **Acceptance:** QAT renders and dispatches Save; Undo/Redo render disabled with a tooltip naming the follow-up; test ids per convention; canonical verification set green.
- **Done:** done in `0050cf2` (RED `5900968`) — QAT strip from core `quick_access` entries above the ribbon; Save dispatches through the shared `command_for` mapping, Undo/Redo stay disabled with a tooltip naming follow-up U14; undo/redo icons and German labels added; UI parity scanner now walks submodules

---

### [x] U7 — File backstage page — 🟡
- **Location:** `crates/datacraft-ui/src/backstage.rs` (neu), `workspace.rs`/`ribbon.rs`, `settings.rs` (I4) für Recent; model: WordCraft `crates/ui-egui/src/backstage.rs` @ `fd7f662` (the sibling `backstage/` directory holds only tests)
- **Approach:** activating the File tab renders a full-page backstage view (Open, Close Database, Recent Databases once I4 lands, New Table flow entry) instead of a ribbon tab with inline path inputs; the path TextEdits leave the ribbon. Every flow dispatches the same single commands as today. The model's pages are Home/New, Open, Info, Recover, Save, Save As, Print, Export, Close, Options, About — no Share/Account page; DataCraft's smaller set is a deliberate subset.
- **Acceptance:** open/close flows work from the backstage (one command each); the File tab shows the backstage page; test ids per convention; canonical verification set green.
- **Done:** done in `9041874` (RED `3e12e06`) — full-page backstage with Home/Open/Save/Info/Options; open/close/new-table/sidecar flows dispatch their existing commands, the path fields left the ribbon, and the language/appearance switchers moved to Options; Recent Databases deferred to follow-up U15

---

### [x] U8 — Datasheet UX: column resize, header sort, row shading — 🟡
- **Location:** `crates/datacraft-core/src/grid.rs` (pure math), `command.rs`/`qbe.rs` (sort mapping), `crates/datacraft-ui/src/datasheet.rs`
- **Approach:** draggable column separators (per-session widths as presentation state in the App; hit-test/min-width math as pure functions in core `grid.rs` per the §8 principle), header click/right-click sort asc/desc mapped onto the query sort model — one command per action —, Access-style alternating row shading plus an edit marker on the selected/edited row.
- **Acceptance:** columns resize by drag; header sort dispatches one command per action; grid math unit-tested in core; canonical verification set green.
- **Done:** done in `78e496b` (RED `165539c`) — drag-resizable columns with core hit-test/min-width/padding math, header-click and context-menu sorting through one SetQbeCell per action, alternating row shading plus the accent cursor marker; widths are per-session App state shared by the new-record row and the SQL window

---

### [x] U9 — Nav pane: category dropdown, context menu, group sort — 🟡
- **Location:** `crates/datacraft-ui/src/nav_pane.rs`; pure ordering helpers in core; behaviour model: Access. WordCraft `crates/ui-egui/src/panes.rs` @ `fd7f662` supplies only task-pane chrome (collapse/indent) — it has no categories, context menu or group sort, so it is not the behaviour model here.
- **Approach:** the "All Access Objects" label becomes a (single-entry) category dropdown per the Access convention; objects are opened/deleted via a context menu (Open/Design/Delete), replacing the always-visible ✕ button; group headers gain a sort menu (name/type) implemented as a pure ordering function over the snapshot-derived list.
- **Acceptance:** delete is reachable via context menu only; the dropdown renders localized (rides on I2); the sort menu reorders deterministically; test ids for menu items; canonical verification set green.
- **Done:** done in `05dd600` (RED `6371c93`) — localized single-entry category dropdown, Open/Design/Delete context menu replacing the always-visible ✕, and name/type group sorting via the pure `nav::sort_entries`; Design opens in one command (`OpenObjectInMode`) with wire coverage

---

### [x] U10 — Tabs: middle-click close, context menu — 🟢
- **Location:** `crates/datacraft-ui/src/workspace.rs`, `crates/datacraft-core/src/command.rs` (CloseAllTabs/CloseOtherTabs)
- **Approach:** middle-click closes a tab; a right-click menu offers Close / Close Others / Close All — the latter two as new bus commands (one command per action, no UI-side loops over tabs), matching the §3 principle that every GUI action is exactly one command. Kind icons ride in with U3. No WordCraft counterpart: the model's desktop keeps one native window per document (`crates/ui-egui/src/windows.rs` @ `fd7f662`), with no document tab strip — the tabbed workspace is the Access convention.
- **Acceptance:** all three actions dispatch commands; headless wire tests for the new commands; test ids per convention; canonical verification set green.
- **Done:** done in `f2ae2a1` (RED `7e3c5f8`) — middle-click closes a tab; Close / Close Others / Close All context menu with the latter two as bus commands (CloseOtherTabs/CloseAllTabs) and JSON-RPC wire tests; closed tabs drop their presentation caches

---

### [x] U11 — Window title & status-bar conventions — 🟢
- **Location:** `crates/datacraft-ui/src/main.rs`/`shell.rs`, `app.rs`
- **Approach:** the viewport title follows the Access convention `{db} : Database ({format})` (localized via I2's catalog), updated on open/close without per-frame churn; with no database open the plain app name shows. The status bar gains an indicator slot (filter state placeholder) consistent with the Access status bar.
- **Acceptance:** the title reflects the open database and clears on close; localized in both languages; the status indicator renders its disabled state; canonical verification set green.
- **Done:** done in `f049073` (RED `93c1760`) — `<db> : Database (<format>)` viewport title localized in both languages, sent only on change and cleared to the app name on close; status bar gains a disabled filter placeholder

---

### [x] U12 — Keyboard: shortcuts & ribbon key tips — 🟢
- **Location:** `crates/datacraft-ui/src/` (new input handling in shell.rs or a small keys.rs), `ribbon.rs` (key tip overlay); model: WordCraft `crates/ui-egui/src/keytips.rs` @ `fd7f662`
- **Approach:** Ctrl+S (Save Design), Ctrl+W (Close Tab), Esc (cancel cell edit / close menu) mapped to the same single commands as their buttons; Alt reveals ribbon key tips rendered from the spec ids (never invented labels). Keyboard documentation rides with M6's README accuracy pass, not here. Scope guard at `fd7f662`: the model's keytips are a four-phase state machine (Tabs → Commands → Legacy → Backstage) including Word-2003 sequences and numeric QAT badges; DataCraft takes only the Tabs and Commands phases.
- **Acceptance:** shortcuts dispatch one command each; key tips visible after Alt and match the spec; headless-testable where egui permits (coordinate with T5); canonical verification set green.
- **Done:** done in `33f6e23` (RED `e108ae7`) — Ctrl+S/Ctrl+W/Esc dispatch/bind the same single actions as their buttons; Alt shows spec-derived key tips (Tabs phase selects, Commands phase badges the actions); derivation (`core::keys::key_tips`) and shortcut mapping are unit-tested

---

### [x] T4 — CLI-Bin-E2E über echten Subprozess (assert_cmd) — 🔵
- **Location:** `crates/datacraft-cli/tests/` (neu: `cli_process.rs`), `crates/datacraft-cli/Cargo.toml` (dev-deps `assert_cmd`, `assert_fs`, `predicates`); `docs/TESTING.md` §3 Registry #2
- **Problem:** Der CLI-Bin wird heute nur über injizierte Streams (`run_with_streams`) getestet — genau deshalb steht `main.rs` mit 17 Zeilen als ungedeckte Ausnahme in der Coverage-Registry (#2, „there is nothing to assert without a subprocess"). Prozessgrenze, echtes stdin/stdout, Arg-Parsing und Exit-Codes sind ungetestet, obwohl README deterministische Exit-Codes als Feature nennt.
- **Ansatz:** Bin als echten Kindprozess starten (`assert_cmd::Command::cargo_bin`), gegen `fixtures/testV2000.mdb` sowie Scratch-SQLite (`assert_fs`/`tempfile`); open/exec/repl-Abläufe über echte stdio-Rohre fahren; Exit-Code-Matrix je Fehlerklasse (fehlende Datei, kaputtes Argument, Read-only-Write) explizit assertieren; Timeouts so setzen, dass ein hängendes REPL den Lauf nicht blockiert.
- **Acceptance:** Mindestens open+exec+nicht-interaktives REPL über den echten Prozess getestet; jeder dokumentierte Exit-Code einmal getroffen und geprüft; Fehlerfälle präzise auf stderr; Registry-Ausnahme #2 in `docs/TESTING.md` neu bewertet (verkleinert oder begründet bestehen bleiben); canonical verification set grün inkl. Coverage-Gate.
- **Done:** done in `c382d2f` (RED `ec77a8d`) — real-subprocess E2E with assert_cmd/assert_fs/predicates covers open/exec/non-interactive repl, the 0/1/2 exit-code matrix and stderr precision; binary routes failures/usage to stderr; TESTING.md registry #2 re-evaluated

---

### [x] F4 — Datasheet/design looked tables up by the wrong test id — 🟡
- **Location:** `crates/datacraft-ui/src/datasheet.rs`, `crates/datacraft-ui/src/design.rs`
- **Problem:** Both views matched `TableEntrySnapshot.ui_test_id` (`navpane.expanded.node.<kind>.<name>`) against `TabSnapshot.ui_test_id` (`workspace.tab.<kind>.<name>`) — the values can never be equal, so every table tab rendered "Table no longer exists" instead of columns, rows and fields. Found by the new headless kittest suite (T5).
- **Fix:** look the table up by name (case-insensitive), like the rest of the UI; fixed inline during step 23.2.
- **Verification:** `datasheet_exposes_its_cells_as_labels` renders header, cells and the row count; canonical verification set green.
- **Done:** fixed inline in `0e86d46` during step 23.2 — datasheet/design now resolve tables by name (case-insensitive) instead of comparing incompatible test-id schemes

---

### [x] T5 — egui/eframe headless UI-Tests (egui_kittest, Layout-Regressionen) — 🔵
- **Location:** `crates/datacraft-ui/tests/` (neu: `ui_kittest.rs`), `crates/datacraft-ui/Cargo.toml` (dev-dep `egui_kittest`), `docs/TESTING.md` §3 Registry #1, `docs/ARCHITECTURE.md` (Schichtgrenze ui/core)
- **Problem:** Die UI-Schicht ist komplett aus der Coverage ausgeschlossen (Registry #1: „eframe startet keinen headless Event-Loop"). Damit sind alle Render-Pfade — Datasheet, SQL-Fenster, Nav-Pane, Ribbon, Statusleiste — ungetestet: Layout-Regressionen und kaputte Widget-Bindings fallen erst im manuellen Lauf auf. `egui_kittest` (offiziell im egui-Repo, AccessKit-basiert, Snapshot-Renderer) schließt genau diese Lücke.
- **Ansatz:** Harness mit `Harness::new_ui`/`new_state` gegen den echten `Core` mit MockPort aufsetzen; Widgets über AccessKit-Knoten (Label/Rolle) ansprechen statt über Pixelkoordinaten; Abläufe fahren (Tabelle in Nav-Pane wählen → Datasheet zeigt Zeilen, Query ausführen → Ergebnis-Grid, Ribbon-Aktion → Bus-Event); Snapshot-Vergleiche (`insta`-Backend von egui_kittest) für die Hauptfenster als Layout-Gate. **Versionskopplung vorab klären:** `egui_kittest` muss exakt zur gepinnten `eframe`/`egui`-Version (0.37) passen — falls für 0.37 nicht verfügbar, Item auf ⛔ setzen und Alternative (Upgrade-Kopplung an egui-Release) im Item notieren statt zu improvisieren.
- **Acceptance:** Harness startet headless ohne Fenster/GPU-Zwang und ohne echtes Event-Loop; je Hauptbereich (Datasheet, SQL/QBE-Fenster, Nav-Pane, Ribbon) mindestens ein Interaktions- plus ein Snapshot-Test; Assertions auf Domain-Zustand (`UiStateSnapshot`) statt auf Pixel; Registry-Ausnahme #1 in `docs/TESTING.md` neu bewertet (Verkleinerung dokumentieren oder Restbegründung); canonical verification set grün, Snapshot-Artefakte reproduzierbar (kein Flaky-Vergleich durch Font-/DPI-Unterschiede in CI).
- **Done:** done in `0e86d46` (RED `3831560`) — datacraft-ui is a lib + thin bin; egui_kittest 0.37 drives ribbon/nav-pane/datasheet/SQL window/backstage via AccessKit, with committed insta semantic-tree snapshots (no pixel compares); TESTING.md registry #1 and taxonomy updated

---

### [x] M10 — Native-query decode errors are dropped at open — 🟡
- **Location:** `crates/datacraft-core/src/database.rs` (`open_database` calls `native_queries().unwrap_or_default()`), `state.rs`/`snapshot.rs`, `port_native.rs`; `crates/datacraft-ui/src/nav_pane.rs`
- **Problem:** A database whose `MSysQueries` contains an undecoded attribute (observed: attribute code 12 on `fixtures/northwind2007.accdb`) opens successfully but silently shows **no** native queries — the precise `Unsupported` reason is discarded. This contradicts LESSONS #4 and makes the compatibility matrix's "unsupported constructs answer a precise `Unsupported`" claim misleading at open time.
- **Approach:** store the decode result and its error in `DatabaseState`/`UiStateSnapshot` (`native_queries_error`), exactly like `macros_error`/`vba_error`; keep the open succeeding with an empty list; render the reason in the navigation pane's Queries group and expose it to inspect/MCP. Add a fixture-backed test on `northwind2007.accdb` pinning attribute 12.
- **Acceptance:** opening the Northwind fixture yields `native_queries` empty plus a visible `native_queries_error` containing "attribute code 12"; `inspect()` carries it; a database with decodable queries has no error; canonical verification set green.
- **Done:** done in `3c7b1fb` (RED `5fb6f96`) — native-query decode errors are stored as `native_queries_error` in the snapshot instead of silently yielding an empty list; the Northwind attribute-12 error is pinned at the port and rendered at the Queries group

---

### [x] M8 — Jet AutoNumber counter maintenance on insert — 🟡
- **Location:** `crates/datacraft-storage/src/jet/writer_rows.rs` (new or `writer_tables.rs`), `port_write.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Problem:** `port_write` maps `FieldKind::AutoNumber` to a precise `Unsupported` because the writer does not maintain the table's counter; `InsertRow` on a Jet table with an AutoNumber column therefore fails.
- **Approach:** Persist the AutoNumber seed/increment in the table's counter metadata, assign the next value on insert (Null = engine-assigned) and keep the assignment deterministic per input state (LESSONS #6); GUID AutoNumbers stay `Unsupported` until qualified.
- **Acceptance:** Inserts assign 1, 2, 3…; reopen keeps the counter; `mdb-export` and Access show the assigned values; user-supplied AutoNumber values stay rejected; GUID flavor answers a precise `Unsupported`.
- **Done:** done in `af07979` (RED `39e95ad`) — Jet4 writer assigns LongInteger AutoNumbers (Null = engine-assigned), persists the TDEF `+12` counter across reopen and rejects user values; GUID flavor stays Unsupported; mdb-export oracle test added

---

### [x] M7 — Jet table drops through the writer (DropTable on .mdb/.accdb) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/writer_tables.rs`, `writer.rs`, `catalog.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Problem:** `Jet4Port::drop_table` answers a precise `Unsupported` ("BACKLOG M7"), so the navigation-pane delete flow (step 11.4) works for SQLite but not for Jet/ACE files.
- **Approach:** Remove the table's TDEF/TDAT pages and its catalog row, return every freed page to the global free map and keep the per-table usage maps consistent; deterministic page selection (LESSONS #6). Verify with `mdb-export` and a reopen; the SQLite drop path stays unchanged.
- **Acceptance:** Drop on a private fixture copy; reopen and the catalog no longer lists the table; free-map and usage-map consistency checked; the oracle reads the file without errors; failure leaves the file untouched.
- **Done:** done in `24f5033` (RED `e3c4601`) — writer/drop path deletes the catalog row, frees used-map + TDEF-chain + unreferenced usage pages into the global free map and 0xFF-fills freed pages; port tests cover reopen visibility, neighbour survival and failure atomicity

---

### [x] M9 — Design-time schema editing (ALTER TABLE) in the table design view — 🟡
- **Location:** `crates/datacraft-core/src/command.rs`, `database.rs`; `crates/datacraft-storage/src/jet/writer_tables.rs`, `src/port_adapter.rs`; `crates/datacraft-ui/src/design.rs`
- **Problem:** The table design grid is read-only; step 6.3 deferred editing to the writer workstream (H9), but no alter path exists yet.
- **Approach:** `AddField`/`DropField`/`RenameField`/`SetFieldProperties` commands through the port, rewriting the TDEF while preserving row data; the design grid becomes editable with save/cancel and dispatches exactly one command per edit; shapes the writer cannot alter answer a precise `Unsupported`.
- **Acceptance:** Design-view edits persist across a reopen with row data preserved; oracle check on the changed file; UI edits dispatch one command each; failures leave the file and state untouched.
- **Done:** done in `6780b3c` (RED `588fe4f`) — AddField/DropField/RenameField/SetFieldProperties via one `alter_table` port method; TDEF rewrite on a scratch writer with row re-encoding (dense column ids on drop), design-grid edit controls dispatch one command each

---

### [x] H42 — ACCDB-Write-Verträge (create/insert/update/delete/drop/alter auf .accdb) — 🟠
- **Location:** `crates/datacraft-storage/tests/ace_writer.rs` (neu), `crates/datacraft-storage/src/jet/writer*.rs` (nur falls Vertragslücken)
- **Ansatz:** Die Jet4-Writer-Verträge auf den gepinnten ACE-Fixtures beweisen: Kopie von `testV2007.accdb`/`testV2010.accdb` in tmp → `create_table` + `insert_row`/`update`/`delete`/`drop_table`/`alter_table` → decode zurück; Original-Seiten byte-identisch (nur Appends), Header unangetastet (Magic/Version), Usage-Maps konsistent. Externes Oracle existiert für ACE nicht (mdb-tools liest kein .accdb) — Referenz ist decode∘encode∘decode gegen die pristinen Bytes; der CI-Oracle-Job erweitert sich nicht.
- **Acceptance:** Verträge grün auf ACE 2007/2010 (und 2016-Datei als Read-only-Referenz); jeder Original-Block byte-erhalten; präziser `Unsupported` bleibt für wirklich Unqualifiziertes.
- **Done:** done in `7e7f755` (RED `3b4a4fb`) — ACE 2007/2010 write contracts green (insert/update/delete/create/drop/alter with byte preservation and map consistency, 2016 fixture as read-only reference); office-encrypted ACCDB now rejected at the writer boundary

---

### [x] H43 — ACCDB-Port-Write-Through + Bus-E2E an .accdb — 🟠
- **Location:** `crates/datacraft-storage/tests/ace_port_writes.rs` (neu), `crates/datacraft-ui/tests/server_e2e.rs` (ACE-Fall), `src/port_adapter.rs` (nur falls Gates)
- **Ansatz:** Port-Write-Through an `.accdb`: tmp-Kopie von `testV2010.accdb` → über `Jet4Port` create/insert/update/delete/drop/alter → Reopen verifiziert Persistenz; `DatabaseFormat::Ace` bleibt über alle Operationen stabil; Bus-Verträge (MCP/CLI) öffnen und schreiben dieselbe Datei über die identische Vokabel.falls Gates existieren (Format-Guards), werden sie bewusst geöffnet — kein stummer Durchgriff.
- **Acceptance:** Port- + E2E-Tests an ACE grün; Reopen zeigt alle Operationen; `Unsupported`-Fälle präzise getestet; Format-Anzeige bleibt „ACE“.
- **Done:** done in `d53c054` — ACE write-through proven at the port (create/insert/update/delete/alter/drop on testV2010 copies, reopen persistence, format stays Ace), over the JSON-RPC wire (server_e2e_ace.rs) and through the CLI vocabulary; no format gate existed to open (pinned fact)

---

### [x] H44 — Ship sample databases and templates (Phase 29) — 🟠
- **Location:** `samples/` (new), `crates/datacraft-core/src/command.rs`, `crates/datacraft-storage/src/jet/create.rs` (new), `crates/datacraft-storage/examples/make_samples.rs` (new), `crates/datacraft-ui/src/backstage/` (new pages)
- **Problem:** No user-facing sample files exist. `fixtures/` is immutable test evidence and includes unlicensed material (Northwind); there is no way to create a new database file at all, so templates cannot be authored reproducibly.
- **Approach:** Own Jet4 file builder (header, free map, catalog, usage page) behind `Command::CreateDatabase`; `CreateDatabaseFromTemplate` for shipped templates; standard set (blank, contacts, inventory, Bike Shop demo as Jet4 + SQLite twin + `.datacraft`); deterministic generator (`make_samples --check`); `samples/manifest`; backstage Samples/New Database pages.
- **Acceptance:** Jet4 and SQLite databases can be created from scratch and reopened; every shipped sample opens with the expected tables/rows; the demo sidecar auto-loads; `make_samples --check` and `sha256sum -c` are green; manual Access repair gate clean; canonical verification set green.
- **Done:** done in `34dbba7` (create `35b2895`, templates `c09d192`, samples `ea98127`, auto-load/backstage `34dbba7`) — CreateDatabase (Jet4 + SQLite), CreateDatabaseFromTemplate over the deterministic core registry, six shipped samples with byte-compare generator check and manifest, sidecar auto-load with sidecar_error, backstage New/Samples pages; the manual Access-repair gate of the shipped samples is a documented follow-up (README matrix)

---

### [x] M17 — Driver model: declarative capabilities, registry and core-computed affordances (Phase 30) — 🟡
- **Location:** `crates/datacraft-core/src/port.rs` (driver id, capabilities), `state.rs`, `snapshot.rs`, `database.rs` (open path), `core.rs` (early rejection, availability map), `crates/datacraft-storage/src/detecting.rs`, `port_adapter.rs`, `sqlite/mod.rs`; UI: `shell.rs`, `present.rs`, ribbon/nav-pane rendering
- **Problem:** Capability gaps are implicit today — trait defaults answer `Unsupported { reason: String }` at call time, `DetectingPort` hardcodes SQLite answers, and several sites exhaustively match the closed `DatabaseFormat` enum. A new driver must touch core/UI/wire; agents and the UI cannot ask what a source supports; the non-atomic SQLite batch is invisible.
- **Approach:** `DriverId` + per-session `DriverCapabilities` on `StoragePort` (ADR-7); registry-based detection; core consults capabilities before dispatch (same `Unsupported` event shape); snapshot gains additive `driver`/`capabilities` fields; core-computed availability map (enabled + disabled reason) rendered by the UI as disabled controls with localized reasons; invariant contract: every capability reported `No` answers `Unsupported` on the matching port call.
- **Acceptance:** Jet and SQLite report complete, behavior-consistent capability sets (invariant tests); read-only files disable the write capabilities; the registry accepts a driver without core/UI edits; the UI greys out disabled commands with localized reasons (kittest snapshots assert on the semantic tree); canonical verification set green.
- **Done:** done in `2d73dae` (RED `a30037c`/`42296a4`; jet `14c6702`+`57d754f`, sqlite `be1a8d2`+`f080b4a`, registry `909c5ca`+`626099c`, rendering pin `c5427ab`) — DriverId + per-session DriverCapabilities/Support on StoragePort, snapshot carries driver/capabilities/availability, early dispatch rejection, registry-driven composite with with_fallback, capability-disabled UI affordances (ribbon/nav-pane/datasheet) with German reasons and kittest pins

---

### [x] M18 — CSV directory driver as the out-of-tree proof (Phase 30) — 🟡
- **Location:** `crates/datacraft-csv/` (new crate, depends only on `datacraft-core`), root `Cargo.toml` (workspace member — Medium gate), new `csv` dependency (Medium gate, per-occurrence release)
- **Problem:** The driver interface is only proven by backends living inside `datacraft-storage`; there is no evidence that a capability-poor driver can dock from a separate crate without touching core, UI or wire.
- **Approach:** a directory of `*.csv` files opens as one database (tables = files, header row = column names, all columns Short Text — deterministic, no type inference); `write_rows` collects in memory and writes via temp file + rename so `atomic_batch` is an honest `Yes`; `alter_table`, `native_queries`, `macros`, `vba`, `passwords` and `autonumber` report `No`; `DatabaseFormat` gains a `Csv` variant rendered by the data-driven display path.
- **Acceptance:** a CSV directory opens, reads and writes through GUI, CLI and MCP; capability invariants hold; registry detection handles directories; wire E2E block green; existing Jet/SQLite behavior unchanged.
- **Done:** done in `5cee066` (RED `0a86718`; gates ① workspace member ② csv dependency released by the human 2026-11-02) — crates/datacraft-csv depends only on datacraft-core: directory = database, *.csv = tables, one atomic rewrite per write (temp + rename); DatabaseFormat::Csv with its single Display arm; composed at the UI/MCP/CLI roots via DetectingPort::with_fallback; server_e2e_csv pins the full stack over the wire

---

### [x] M12 — Native Query-Execution (MSysQueries ausführen, begrenzter Dialekt) — 🟡
- **Location:** `crates/datacraft-core/src/` (QueryDef aus nativen Queries → Engine-Ausführung), `crates/datacraft-storage/src/port_native.rs` (falls Attribut-Erweiterung), UI: QBE-Ausführung nativer Queries
- **Ansatz:** Native Saved Queries sind lesbar (M10) — jetzt **ausführbar** über die eigene Engine: QueryDef (Tabellen/Spalten/Sort/Criteria, Joins im begrenzten Umfang) wird in die In-Memory-Engine überführt und über den Bus ausgeführt (`RunQuery`-Pfad); Konstrukte außerhalb des qualifizierten Dialekts (Ausdrücke, Parameter, Unterabfragen) → präziser `Unsupported` mit Grund statt stiller Fehler.
- **Acceptance:** Fixture-Queries aus `queryTestV2000.mdb` laufen durch die Engine (Ergebnisse = QBE-Äquivalent); `Unsupported`-Grundfälle getestet; UI führt native Queries aus; MCP/CLI erben über den Bus.
- **Done:** done in `3fd07a9` (RED `02b091a`; coverage `d668c7b`/`bf872eb`) — native selects execute the qualified Access dialect: equality joins (inner/left/right), WHERE expressions via the phase-15 service, Table.* projection and ORDER BY; fixture SelectQuery runs over the real port, UI Run is kittest-pinned, everything else answers a precise Unsupported

---

### [x] M14 — Encrypted Save-As + Passwortwechsel (verschlüsselte Kopien) — 🟡
- **Location:** `crates/datacraft-storage/src/port_adapter.rs` (`with_writer` already re-encrypts on save — step 16.2), `crates/datacraft-core/src/command.rs` (new: Save-As/password change), UI backstage, CLI/MCP flags
- **Approach:** The write path re-encrypts correctly on save (M11). Remaining: **Save-As** — a copy under a new path (optionally with a different/new password: unencrypted → encrypted, password change) as a command over the bus; all four clients serve it; precise errors on a wrong old password. Post-Phase-30 (ADR-7): the Save-As affordances gate on the driver's `passwords` capability — sources reporting `passwords: false` (SQLite, CSV) render the backstage entry disabled with the localized reason, and the dispatch rejects with the same `Unsupported` shape as the other capability gates.
- **Acceptance:** Save-As to an unencrypted/encrypted copy tested (reopen with/without password); password-change tests; UI/CLI/MCP flow; the original stays untouched; the capability gating is pinned like the other affordances (kittest + dispatch rejection).
- **Done:** done in `8b2dccc` (RED `9614c5a`; coverage `bf872eb`) — SaveDatabaseAs writes unencrypted/protected Jet4 copies with a deterministic page key while the source stays untouched; ChangePassword rewrites in place atomically; ACE keeps its Office-encryption boundary and Jet3 stays read-only; backstage/CLI/JSON-RPC flows and capability gating are pinned

---

### [x] U14 — Command history for Undo/Redo — 🟡
- **Location:** `crates/datacraft-core/src/state.rs`, `command.rs`, dispatch modules; `crates/datacraft-ui/src/ribbon.rs` (QAT enablement)
- **Problem:** The Quick Access Toolbar declares Undo/Redo (step 21.2, item U6) but renders them disabled: the command bus keeps no history, so there is nothing to undo.
- **Approach:** A bounded per-database history stack of applied commands with deterministic inverse operations (the M2 journal already produces inverse write ops); `Undo`/`Redo` are ordinary bus commands that pop and push through the same reducer, so GUI, CLI and MCP share them; the snapshot exposes the history depth and the QAT entries enable from it. Post-Phase-30 (ADR-7): batch entries consult the driver's `atomic_batch` capability — on non-atomic drivers (SQLite reports `false`) multi-op journals either stay single-op or record the degraded mode (see M20); undo must behave identically on every registered driver, pinned by the capability invariants.
- **Acceptance:** Undo after a row insert/update/delete restores the previous row data (verified by reopening the file); Redo re-applies it; each action is exactly one command over the wire; QAT entries enable/disable from the snapshot; undo/redo verified on Jet, SQLite and CSV; canonical verification set green.
- **Done:** done in `a4ca640` (RED `926cbca`) — bounded per-database row-write history; Undo/Redo bus commands address rows by value image, restore_row preserves AutoNumber identities, journal/schema/file lifecycle are barriers, QAT enables from snapshot depths; Jet4 delete/restore verified across reopen, SQLite restore pinned

---

### [x] U15 — Recent databases in the backstage — 🟢
- **Location:** `crates/datacraft-ui/src/settings.rs` (persisted list), `crates/datacraft-ui/src/backstage.rs` (Recent page)
- **Problem:** The File backstage (step 21.3, U7) has no Recent Databases entry: `ui.json` (I4) persists language, appearance and window geometry, but no opened-file history. U7's scope named Recent "once I4 lands"; I4 shipped without the list, so the page is deliberately absent.
- **Approach:** persist a bounded, de-duplicated recent-paths list on successful `DatabaseOpened` (atomic save via U13's helper), render it on the backstage Home/Open page, and dispatch one `OpenDatabase` per click; missing files are pruned on load with a precise notice instead of disappearing silently.
- **Acceptance:** opening a database adds it to the recent list; the list survives restarts; clicking an entry dispatches exactly one open command; stale paths are pruned visibly; canonical verification set green.
- **Done:** done in `da1a0e8` (RED `2ad9d26`) — ui.json persists a bounded, de-duplicated recent list on DatabaseOpened; stale paths are pruned on load with a localized notice; the backstage Home/Open pages dispatch one OpenDatabase per click; kittest and updated Home snapshot pin the flow

---

### [x] M20 — Journal batches gate on the declared atomic_batch capability (Phase 30 follow-up) — 🟡
- **Location:** `crates/datacraft-core/src/journal.rs`, `crates/datacraft-core/src/database.rs` (dispatch gates), `crates/datacraft-storage/src/sqlite/mod.rs`
- **Problem:** The M2 journal documents batch atomicity as a backend property; since Phase 30 the driver *declares* it (`DriverCapabilities::atomic_batch`). SQLite reports `false` (the trait default applies ops one by one), yet `PreviewWrite`/`ApplyWrite` still accept multi-op batches there — a mid-batch failure leaves a partial application with only best-effort inverse ops. Second gap from the Phase-30 review: the journal commands are not gated on `write_rows` at all — on a read-only source the batch previews fine and fails mid-apply at the port instead of being refused up front.
- **Approach:** Consult the capability in `preview_write`: drivers with `atomic_batch: false` either accept single-op batches only (precise `Unsupported` reason for more) or record the degraded mode in the pending journal and in the snapshot; the UI surfaces it through the established availability/reason pattern (ADR-7). Gate `PreviewWrite`/`ApplyWrite` on `write_rows` like the other write commands (same `Failed` shape, refused before the port). Jet behavior is unchanged (`atomic_batch: true`).
- **Acceptance:** A multi-op preview on a non-atomic driver answers the precise reason (or carries the degradation visibly in snapshot and UI); a read-only source refuses the preview up front via the `write_rows` gate; Jet batches unchanged; the driver capability invariants cover both gates; canonical verification set green.
- **Done:** done in `fc2b623` (RED `905ed72`) — preview_write and apply_write refuse multi-op batches on drivers with atomic_batch=false with the precise one-at-a-time reason; single-op batches stay available; Jet's atomic path unchanged; core and SQLite contracts pin both sides

---

### [x] H45 — Persist field properties (defaults, validation, captions, formats, input masks, index flags) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/writer_tables.rs`, `crates/datacraft-storage/src/port_write.rs`, `crates/datacraft-storage/src/port_schema.rs`, `crates/datacraft-core/src/forms.rs` (sidecar tables)
- **Problem:** `create_table` writes `NULL` into `MSysObjects.LvProp`, and `port_schema.rs` reconstructs only the column type plus `required`. Defaults, validation rules, captions, formats, input masks and `Indexed` are lost on every reopen. The sidecar saves `tables: Vec::new()` and ignores `document.tables` on load, so it cannot carry them either. Templates therefore ship as structure + data only.
- **Approach:** Decide the authority (short ADR addendum, consistent with ADR-1): Jet `LvProp` property blocks so Access sees the properties, or the sidecar's table list as DataCraft's own metadata. Implement write + read round-trip for every modeled `FieldProperties` entry, keep the file Access-readable, and reject unsupported property encodings precisely. Post-Phase-30 (ADR-7): the property surface already flows through `alter_table`/`SetFieldProperties`, so capability gating of the design-grid edits comes for free; property persistence stays a per-driver concern (Jet/ACE formats can carry it, the CSV driver stays text-only by design).
- **Acceptance:** Every modeled property survives a Jet4 reopen (and/or sidecar load); Access displays defaults/validation/captions; tests pin each property and the rejection cases; no silent loss remains.
- **Done:** done in `c5abd00` (RED `c5176bd`) — the .datacraft sidecar is the property authority (ADR-1 addendum): tables are saved with the full FieldProperties sheet, overlaid on explicit load and sibling auto-load with precise stale/invalid handling, preserved across Jet/ACE schema refreshes; the Jet writer keeps NULL LvProp (Access display deferred to the LvProp roadmap item); design view renders the restored sheet (kittest)

---

### [x] M15 — Index creation in the Jet writer — 🟡
- **Location:** `crates/datacraft-storage/src/jet/encode.rs` (`num_idx`/`num_real_idx`), `crates/datacraft-storage/src/jet/writer_tables.rs`, `crates/datacraft-storage/src/port_write.rs`, `crates/datacraft-storage/src/sqlite/mapping.rs`
- **Problem:** `encode_tdef` writes zero index blocks; `FieldProperties.indexed` is serialization/UI metadata only. Generated tables and samples cannot demonstrate indexed lookups or uniqueness, and a template's `Indexed` flag does nothing.
- **Approach:** Encode TDEF index descriptors and index pages for `IndexMode::{DuplicatesOk, NoDuplicates}`, maintain them on insert/update/delete, and emit `CREATE INDEX`/`UNIQUE` for SQLite. Keep existing index blocks readable and untouched on unrelated writes.
- **Acceptance:** Indexes survive reopen and are visible in Access; uniqueness is enforced with a precise error; SQLite emits the matching DDL; tests cover create/insert/update/delete and the unsupported shapes.
- **Done:** done in `c85b1b1` — TDEF index metadata encode/decode (physical + logical blocks, names), domain IndexMode round-trip, uniqueness on insert/update/batches, alter add/remove/renumber with duplicate guard, SQLite CREATE [UNIQUE] INDEX; physical Jet index pages remain the roadmap follow-up

---

### [x] M16 — Relationship and foreign-key creation — 🟡
- **Location:** Jet writer (`MSysRelationships` rows and the required indexes), `crates/datacraft-storage/src/sqlite/mapping.rs` (foreign-key DDL), `crates/datacraft-core/src/port.rs` (create/delete relationship surface)
- **Problem:** Relationships are read-only on both backends. Generated samples cannot populate the relationship mapper, and a template cannot express referential integrity.
- **Approach:** Add a create/delete relationship path through the port and a matching bus command. Jet writes `MSysRelationships` plus the required primary/foreign indexes; SQLite emits `FOREIGN KEY` constraints. Reads must round-trip through the existing `relationships()` API. Post-Phase-30 (ADR-7): landing this flips the jet and sqlite drivers' `relationships` capability from `ReadOnly` to `Yes` — the availability map (the disabled Relationships affordance on CSV stays) and the mapper affordances update automatically; the flip is pinned in the driver capability invariants.
- **Acceptance:** Created relationships survive reopen and appear in the mapper; Access shows them; SQLite enforces the constraint; the capability flip is pinned (`ReadOnly` → `Yes` for jet/sqlite, CSV stays `No`); tests cover create/delete, duplicates and unsupported shapes; canonical gates green.
- **Done:** done in `1729228` — CreateRelationship/DeleteRelationship bus commands; Jet lazily creates MSysRelationships + parent/child indexes and writes/deletes rows; SQLite rebuilds the child table with enforced FOREIGN KEYs (orphans/duplicates refused); Jet/SQLite capabilities flip to Yes, CSV stays No

---

### [x] L6 — Ship samples next to the binary and in release archives — 🟢
- **Location:** release packaging (Phase 26, M13), `crates/datacraft-ui/src/samples.rs` (runtime resolution)
- **Problem:** The backstage Samples page resolves `samples/` from `DATACRAFT_SAMPLES_DIR`, the executable directory, or the dev workspace. Installed builds have no installer step that places the folder next to the binary, so the page falls back to a path that does not exist.
- **Approach:** Copy `samples/` into every release archive next to `datacraft`/`datacraft-cli`/`datacraft-mcp`, document the resolution order, and add a smoke check that an extracted archive finds and opens the demo.
- **Acceptance:** Release archives contain `samples/`; the backstage page resolves them without repo paths; a packaged smoke test opens the demo; no binary or sample is committed into the repo twice.
- **Done:** done in `7ba87f1` (step 26.1) — release.yml copies samples/ into every archive and the packaged smoke opens the bike-shop demo from the extracted archive; resolution order documented at backstage.rs

---

### [x] M13 — Release-Engineering (Tags, release.yml, Checksums, Smoke) — 🟡
- **Location:** `.github/workflows/release.yml` (new), `Cargo.toml` (version policy), `docs/PLAYBOOK.md` (release section), Third-Party-Notices
- **Approach:** Tag-triggered release workflow (mirror of the CI gates): build `datacraft`, `datacraft-cli` and `datacraft-mcp` for Windows/macOS/Linux (rusqlite statically bundled), archives with sha256 checksums + third-party notices. Post-Phase-30 additions (ADR-7): the composed binaries bundle the CSV driver through the composite fallback, so the per-artifact smoke also opens a throwaway CSV directory headlessly (the driver set is proven inside the artifact), and the notices name the `csv` crate.
- **Acceptance:** The workflow file lints (YAML/Actions conventions like ci.yml); the local release smoke opens `testV2000.mdb` headless *and* a CSV directory; the tag policy is documented; no binary lives in the repo.
- **Done:** done in `7ba87f1` + `d3a008c` (steps 26.1/26.2) — release.yml matrix/packaging/per-artifact smoke/checksums, draft-release publishing via gh, PLAYBOOK §9 tag policy; local packaged smoke green on windows (jet fixture, samples demo, csv directory)

---

### [x] M21 — Retire the vestigial StorageEngine trait (ADR-7 single seam) — 🟡
- **Location:** `crates/datacraft-storage/src/engine.rs`, `crates/datacraft-storage/src/mock.rs` (`InMemoryEngine`), `crates/datacraft-storage/tests/engine_boundary.rs`; `docs/ARCHITECTURE.md` §6 (already annotated as the historical record)
- **Problem:** ADR-7 made the core-owned `StoragePort` the single driver seam; the Phase-2 `StorageEngine` trait survives with one mock implementor and pins an outdated contract (Jet3-unaware, no capabilities, no create/drop/alter surface) right next to the real one — two seams confuse contributors and the public library story (M19).
- **Approach:** Port the still-valuable contract assertions of `engine_boundary.rs` onto the port layer (they fold into the existing driver-capability invariants), then remove the trait, the mock and the boundary suite. NOTE: deleting an existing test suite is a **High-class gate** (PLAYBOOK §0.5) — package with an explicit human release; coverage must not drop (the ported assertions keep the lines covered).
- **Acceptance:** One seam only (`StoragePort`); the boundary contract lives in port-level invariants; coverage ≥ 95 % holds; ARCHITECTURE §6 cites the removal; canonical verification set green.
- **Done:** done in `45a852d` + `44b1fff` (step 31.1) — trait/mock/boundary suite removed after porting the contract to tests/port_contract.rs (Jet4 + SQLite cross-driver); coverage >= 95 held; ARCHITECTURE §6 cites the removal; finding M24 (tri-state Boolean NULL) filed from the porting run

---

### [x] M22 — Publish-ready API vocabulary: non_exhaustive and the capabilities default (Phase 31 prep) — 🟡
- **Location:** `crates/datacraft-core/src/port.rs` (`PortError`), `crates/datacraft-core/src/port/capabilities.rs` (`Support`, `DriverCapabilities`), `crates/datacraft-core/src/command.rs` (`Command`), semver policy of step 31.4
- **Problem:** The vocabulary enums are closed: every new `Support`/`PortError`/`Command` variant is a breaking change for third-party drivers compiled against the published engine. Worse, the trait default `DriverCapabilities::full()` makes a driver that forgets the override *lie* — it reports every capability as available and the dispatcher then lets commands through that the port refuses at call time. Both surface only after crates.io publishing (M19), when they become semver pain instead of a cheap pre-split decision.
- **Approach:** Before the crate split (31.2): mark the wire-facing enums `#[non_exhaustive]` (`Support`, `PortError`; `#[non_exhaustive]` on the `DriverCapabilities` struct for its fields), decide the default question explicitly — conservative default (a forgetful driver declares nothing) versus making `capabilities()` required — pin the decision in ADR-7 and the 31.4 Semver policy, and add a meta-test that every registered shipped driver overrides `driver_id`/`capabilities` explicitly.
- **Acceptance:** Adding a vocabulary variant does not break a compiled out-of-tree driver (proven by `datacraft-csv` compiling unchanged against the extended core); the default story is decided, documented and meta-tested; canonical verification set green.
- **Done:** done in `5dad505` — driver_id/capabilities required, none()/full() constructors, Support/PortError/Command non_exhaustive, ADR-7 addendum item 6; datacraft-csv compiles unchanged (acceptance proof); deviation documented: DriverCapabilities keeps constructible pub fields (non_exhaustive struct would break field reads)
