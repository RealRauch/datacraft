# BACKLOG.md — Open items (As of: 2026-11-03T19:11

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

> **Model references** (`model:` lines) point at `storytold/wordcraft` @ `fd7f662` (2026-11-01); the named paths and values are verified against that revision.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

---

## 🟡 MEDIUM

> No open items.

### [ ] R1 — Cross-cutting code review round (Phase 34) — 🟡
- **Location:** whole workspace (review only — findings become BACKLOG items; fixes only via the inline-fix lane or packaged follow-ups)
- **Problem:** Thirty-plus phases of feature work have never had a deliberate cross-cutting review pass; the per-phase gates verify the canonical set, but nobody has walked the LESSONS checklist across module boundaries since the driver model, the capability vocabulary and the CSV driver landed.
- **Approach:** One review phase before publishing (Phase 34), ordered per `docs/LESSONS.md`: (1) storage & drivers — security self-review (LESSONS #7: crafted `.mdb`/`.accdb`/`.csv` against the parsers), error precision and locatability (LESSONS #4), clean-room spot check (LESSONS #12); (2) core — dispatch/journal/capability-gate consistency, event/snapshot shape stability; (3) UI — paint-code discipline (no logic in paint), i18n parity, affordance coverage; (4) public API surface audit for Phase 31 (every `pub` item that would be published: needed, documented, semver-sane). Findings are triaged in-phase: Low-class ≤10-liners via the inline-fix lane (with F-retrospectives), everything else packaged.
- **Acceptance:** Every LESSONS checklist item walked with a written verdict; findings triaged (fixed inline or BACKLOG items with locations + acceptance criteria); no HIGH finding left unassigned; canonical verification set green.

### [ ] R2 — Documentation sweep before the release (Phase 35) — 🟡
- **Location:** `AGENTS.md` (§1 workspace list + dependency diagram lack `datacraft-csv`), `README.md` (architecture table + diagram), `docs/ARCHITECTURE.md` (§5 command-enum preview is the Phase-4 target shape, §6 "remaining write-path work" still lists M7/M8/ACE-write although done, §13 phase-mapping table ends at Phase 9), `docs/FORMAT-JET.md` (no CSV-driver note), `docs/TESTING.md` (taxonomy vs. capability-invariant/kittest suites)
- **Problem:** The documentation has drifted across phases 10-30: the entry points an agent or contributor reads first (AGENTS.md, README architecture) describe a four-crate workspace, ARCHITECTURE carries stale milestone and mapping sections, and the driver model (ADR-7) is documented in the ADR but not in the navigation docs. A release must not ship with entry-point docs that contradict the repository.
- **Approach:** One sweep phase right before the release (Phase 35), after Phase 31 freezes the crate structure: (1) AGENTS.md + README architecture/diagrams reflect the real crate graph; (2) ARCHITECTURE §5/§6/§13 rewritten to the current truth (preview sections replaced or explicitly dated); (3) FORMAT-JET gains the CSV driver section, TESTING taxonomy matches the live suites; README compatibility matrix re-verified date-stamped. Append-only archives stay untouched; every claimed number is checked against the code.
- **Acceptance:** No tracked doc contradicts the repository state (crate lists, phase mappings, remaining-work claims); the compatibility matrix is date-accurate; a fresh-contributor walkthrough (clone → read AGENTS/README → build → open a CSV directory) works without hitting stale guidance; canonical verification set green.

### [ ] M29 — UI row caches go stale after Undo/Redo and other row-mutating events — 🟡
- **Location:** `crates/datacraft-ui/src/app.rs:366-497` (`maintain_caches` invalidates `rows` only for `RowInserted`, `DatabaseClosed` and `TableDropped`).
- **Problem:** The datasheet renders from the per-tab row cache and only reloads a tab whose cache is missing. After `Undo`/`Redo` from the Quick Access Toolbar (rows change, events are `Undone`/`Redone`) the grid keeps showing the pre-undo rows; the same applies to `RowUpdated`/`RowDeleted`, `WriteApplied`/`WriteRolledBack` and `TableAltered` if those commands are ever driven from this process. Form runtime tabs bound to an inserted table are also not refreshed (the invalidation branch only matches `object_kind == "table"`).
- **Approach:** Derive the affected table from every row-mutating event (`Undone`, `Redone`, `RowUpdated`, `RowDeleted`, `WriteApplied`, `WriteRolledBack`, `TableAltered`) and drop the caches of all open tabs bound to it — table tabs and form tabs whose source matches. One shared helper instead of per-event special cases.
- **Acceptance:** Undo/Redo and a journal apply refresh every grid bound to the table; a kittest drives insert → undo and asserts the row disappears; no cache survives a schema change.

### [ ] M31 — Presentation cache loads swallow read errors — 🟡
- **Location:** `crates/datacraft-ui/src/app.rs:462-470` (`if let Ok(rows) = self.core.read_rows(…)`) and `:489-495` (`if let Ok(result) = self.core.run_query(…)`), rendered by `crates/datacraft-ui/src/datasheet.rs:59-61` ("Loading rows…").
- **Problem:** A failing row read or query run is discarded: the cache stays empty and the datasheet displays "Loading rows…" forever, with no reason anywhere. LESSONS #4 forbids silent swallowing — the other command paths surface failures in the status bar, this one does not.
- **Approach:** Store the failure reason per tab (or in `last_error`) and render it in place of the loading text; retry when the next mutation pass runs.
- **Acceptance:** A port read failure shows a precise, localized reason in the affected view instead of an endless loading state; kittest with a failing mock port pins it.

### [ ] M33 — datacraft-jet publishes its raw format layer without a stability contract — 🟡
- **Location:** `crates/datacraft-jet/src/lib.rs:9-13` (`pub mod jet`, `pub mod port_write`), the `pub` items inside `src/jet/**` (`JetTableDef`, `JetColumn`, `JetType`, `Jet4TableReader`, `Jet4Writer`, `decode_row`, `encode_row`, page/usage-map/crypto helpers); `docs/PUBLISHING.md` (the jet row names the crate but not its supported surface).
- **Problem:** The crate publishes its entire raw format layer together with the supported `Jet4Port`/`StorageError`/`types` API, and the integration tests need that visibility (`datacraft_jet::jet::{...}`). Published `pub` items carry a semver promise, so every internal parser refactor would force a minor bump, and docs.rs presents the internals as equal to the port. The publishing policy neither claims nor excludes this surface, so a consumer cannot tell what is stable.
- **Approach:** Decide and document: either keep the raw layer as a deliberately public low-level API (state it in PUBLISHING, keep it documented and semver-covered) or hide it behind a `#[doc(hidden)]`/feature-gated `internals` module used only by the test targets. The port-facing API stays the supported contract either way.
- **Acceptance:** `docs/PUBLISHING.md` names the supported surface of `datacraft-jet`; docs.rs shows the supported API as primary; integration tests keep compiling without a behaviour change; the decision is recorded (ADR-7 addendum).

---

## 🟢 LOW

> No open items.

### [ ] I6 — Failure-reason localization (post-V1) — 🟢
- **Location:** `crates/datacraft-core/src/core.rs` + dispatch modules (stable failure codes first), `crates/datacraft-ui/src/i18n/` (UI-side mapping); ADR-4
- **Problem:** `Event::Failed.reason` strings are English on the wire (~120 sites plus the Phase-30 capability rejections, e.g. `unsupported: the {driver} driver cannot write rows for this source`); localizing them inside core would put language into core (ADR-4 violation), so V1 keeps them English — agents key on data, not prose.
- **Approach:** Two steps, post-V1: (1) core gains stable machine-readable failure codes (enum or code prefix) alongside the English text — a domain concern, not a setting; the capability-rejection reasons map onto the `DisabledReason` vocabulary where they overlap; (2) the UI maps known codes to localized text via its catalog; unknown reasons render in English unchanged.
- **Acceptance:** Every dispatch failure carries a stable code (including the capability gates); the UI renders localized text for known codes and English otherwise; the wire gains codes without breaking existing consumers; English-substring tests (command_errors.rs, cli_contract.rs, server_e2e.rs, driver_caps.rs) switch to code assertions.

### [ ] L8 — Server sources: SourceSpec for the open commands (ADR-7 future path) — 🟢
- **Location:** `crates/datacraft-core/src/command.rs` (`OpenDatabase`/`CreateDatabase`), `state.rs`/`snapshot.rs` (path → source), UI backstage, sidecar path derivation (ADR-1)
- **Problem:** The driver model is ready for server drivers (Postgres/SQL Server per ADR-7's documented future path), but the wire and the UI assume a filesystem path (`path: PathBuf`, file-stem names, a sidecar next to the file).
- **Approach:** Additive `SourceSpec { Path, Uri }` on the open commands (the existing field keeps working for file sources), the snapshot gains a source display, the sidecar applies to `Path` sources only; the first actual server driver decides credentials/SSL and stays synchronous behind the no-async rule (ADR-7 records the constraint honestly — OCI is C-only, pure-Rust TDS/Postgres clients are async-first).
- **Acceptance:** A URI source opens through all four clients; file behavior is byte-identical; ADR-7 gains the extension note when the first server driver is planned; canonical verification set green.

### [ ] L10 — CI: macOS jobs infra-cancelled — rerun for a fully green run — 🟢
- **Location:** `.github/workflows/ci.yml` (macos-latest matrix entries), GitHub runner capacity/billing (account side)
- **Problem:** Since 2026-11-03 ~14:00 UTC every `Tests (macos-latest)` and `Oracle (mdb-tools cross-check) (macos-latest)` job is cancelled in the runner queue after exactly 15m01s with no logs (five consecutive attempts across runs 41645734997/41653027497/41666011189, including two explicit reruns). Earlier the same day both macOS jobs ran green (oracle 28s, tests 1m25s). All non-macOS jobs are green; the macOS suite is the same code that passes on ubuntu/windows, and the ubuntu oracle covers the mdbtools cross-check.
- **Approach:** No repository change — retry `gh run rerun --failed` when GitHub macOS runner capacity/billing recovers (check account Settings → Billing → Actions minutes: the macOS 10× multiplier is the prime suspect). Optionally consider moving the oracle job to ubuntu-only (it already carries the mdbtools cross-check) if macOS capacity stays scarce.
- **Acceptance:** A CI run on `main` completes with the macOS jobs green (or the oracle matrix consciously narrowed with a documented decision); nothing in the repo changes until then.

### [ ] L11 — Mechanical-edit artifacts in messages and a doc comment — 🟢
- **Location:** `crates/datacraft-core/src/journal.rs:237`, `crates/datacraft-core/src/schema/document.rs:139` and `:152`, `crates/datacraft-jet/src/port_write.rs:174`, `crates/datacraft-jet/src/jet/header.rs:99` and `:106` (long space runs); `crates/datacraft-core/src/transport.rs:89-90` (doc comment lost its parameter names: "requests from , writing … to  (flushed …)").
- **Problem:** Six user-facing messages carry long runs of spaces mid-sentence (e.g. "engine-assigned                          value", "is not a                      supported combination"), and one public doc comment lost two identifiers. All are leftovers of a mechanical edit (LESSONS #8): they render as gaps in toasts/errors and read as broken documentation.
- **Approach:** Replace each run with a single space and restore the two parameter names in the doc comment; no wording changes otherwise. If any test asserts on the exact string, update it in the same step.
- **Acceptance:** `grep -nE '[a-z] {2,}[a-z]'` over the message strings returns only intentional formatting; the transport doc comment names its reader/writer parameters; affected tests are green.

### [ ] L13 — SQLite affinity edge cases: AUTOINCREMENT substring and parameterized type names — 🟢
- **Location:** `crates/datacraft-sqlite/src/mapping.rs:224` (`autoincrement` detected by uppercasing the whole `CREATE TABLE` text and searching for the substring `AUTOINCREMENT`), `:279` (marker types compared by exact uppercase name).
- **Problem:** Two affinity edge cases misclassify schemas. A table whose DDL mentions `AUTOINCREMENT` anywhere (a column default, a quoted name) turns every `INTEGER PRIMARY KEY` into an AutoNumber and rejects user values on write. Conversely a declared type with a length/precision suffix (`DATETIME(6)`, `CURRENCY(10)`) misses the marker match and falls through to `Decimal`.
- **Approach:** Detect `AUTOINCREMENT` on the primary-key column's definition (token-aware, or via `PRAGMA table_info` + the DDL segment belonging to the PK column), and strip a trailing `(…)` before the marker-type comparison. Affinity fallbacks stay unchanged.
- **Acceptance:** A DDL mentioning the word outside the PK column does not create an AutoNumber; `DATETIME(6)` maps to `DateTime` (and `CURRENCY(10)` to `Currency`); unit tests pin both directions, real fixture catalogs unchanged.

### [ ] L17 — Native query flavor renders as a Rust Debug name — 🟢
- **Location:** `crates/datacraft-ui/src/qbe.rs:201` (`let kind = format!("{:?}", native.kind)`), rendered at `:235-243`.
- **Problem:** The native-query flavor is rendered from the Rust `Debug` name ("Select", "Crosstab", "MakeTable", …) inside an otherwise translated sentence. In German the line reads "Select — read-only (MSysQueries)"; renaming the enum variant would silently change user-visible text. The i18n parity scanner cannot see it because it is not a `t()` key.
- **Approach:** Add a `NativeQueryKind` label function in the UI (match every variant to a `t()` key), mirroring `labels::field_kind_label`; keep the wire enum untouched (ADR-4).
- **Acceptance:** Every native query flavor renders from the catalog in both languages; the German catalog gains the entries and the parity test stays green.

### [ ] L18 — Design-time schema edits are not capability-gated in the UI — 🟢
- **Location:** `crates/datacraft-ui/src/design.rs:111-168` (Add Field / Rename / Delete / Required are always enabled), `crates/datacraft-core/src/availability.rs` (`DisabledReason` has no `AlterTables` variant and no affordance id).
- **Problem:** The design view offers schema edits regardless of the driver's `alter_table` capability. SQLite reports `alter_table=false`, so every click there produces a `Failed` toast ("unsupported: the sqlite driver cannot alter the schema for this source") instead of a disabled control with the reason — the affordance vocabulary covers create/drop/write/relationships/passwords but not alter. The backstage "Create Database" button is likewise enabled from the selected format rather than `create_database`.
- **Approach:** Add the `AlterTables` reason plus the design-view affordance ids to the core availability map, render the controls disabled with the localized tooltip, and gate the Create Database button on the same map; keep the dispatcher gates as the second line of defence.
- **Acceptance:** On a SQLite session the design-view edit controls (and on a creation-disabled composition the Create Database button) are disabled with a localized reason; kittest pins the SQLite case; no command is required to discover the restriction.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T6 — Semantic-Conformance-Hinweise fürs Code-Review (Phase 34) — 🔵
- **Location:** Review-Ziele: `crates/datacraft-core/src/expr/` (coerce/funcs), `query_engine.rs`, `sql_engine.rs` + Shadow-SQLite-Materialisierung; Referenz-Oberflächen: Expression-Service, QBE-Engine, SQL-Fenster
- **Hinweis 1 — Currency/Präzision:** Shadow-SQLite hält Currency als REAL → `SUM`/`AVG` über Money rundet binär statt auf 4 Dezimalstellen. Spot-Check: SUM über 3 Cent-Beträge, Multiplikation Geld×Zahl.
- **Hinweis 2 — Kollation:** SQLite `BINARY` ≠ Access kulturabhängig (ä/ö/ü/ß, case-insensitiv) in ORDER BY/WHERE/Vergleichen. Spot-Check: Sortiere [Ärger, Ass, Berg], Filter `Name = 'müller'` case-insensitiv.
- **Hinweis 3 — Datum:** SQLite hat keinen DATE-Typ → Literale `#…#`, `Datum + 1`, Komponenten-/Format-Funktionen über Text-Realität. Spot-Check: Datum+1, Jahresfilter, Format „Langes Datum".
- **Hinweis 4 — LIKE-Wildcards:** Access `* ? # [..]` vs SQL `% _` im SQL-Fenster — Übersetzung oder dokumentierte Einzäunung. Spot-Check: `LIKE 'B*'` in QBE und im SQL-Fenster.
- **Hinweis 5 — Promotion/Overflow:** Int-Overflow, Mixed-Type-Promotion-Matrix (Tiefe von `coerce.rs`). Spot-Check: 32767+1, Int×Int, Text+Zahl.
- **Hinweis 6 — NULL-Kanten:** `NOT IN` mit NULL, `Nz()`-Rückgabetypen, IIf-Zweig-Auswertung. Spot-Check: je ein Fall pro Kante.
- **Hinweis 7 — Multivalue-Prädikate im SQL-Pfad:** Attachment-/Multivalue-Felder in SQL → mögliche stille Leerergebnisse. Spot-Check: WHERE über Multivalue-Spalte.
- **Acceptance:** Jeder Cluster mit 3–5 Vektoren gegen alle drei Oberflächen gespielt; Divergenzen werden Findings (→ 34.5 Triage) oder bewusst eingezäunt/dokumentiert; Ergebnis landet als Folge-Items bzw. in TESTING/FORMAT-JET-Doku.

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
- H11 — Versioned schema sidecar document (serde persistence) — done (steps 3.2 (test 5432ef9, feat a72c5e6))
- H10 — Access data types and field properties — core domain model — done (step 3.1 (test 7cf9a31, feat 1271f33))
- H13 — Command dispatch core — commands, events, state, storage port — done (step 4.1 (test baa4f16, feat f796480))
- H14 — State query API and deterministic test IDs — done (step 4.2 (test da774ad, feat 1b4a9be))
- H15 — JSON-RPC transport codec stub (headless seam) — done (step 4.3 (test ceecb24, feat a0c5137))
- H17 — App shell and layout — four-region egui shell over the core — done (step 5.1 (test 5ed94ed, feat 666acd7))
- H12 — GUID field kind — plain Replication ID columns unmodelable — done (decided: dedicated FieldKind::Guid; implemented with H16 (tests f4c96d3/e1bb1bb, adapter 67a3b64); decision documented in docs/FORMAT-JET.md "Domain mapping")
- H16 — StoragePort adapter — Jet4 backend implements the core port — done (step within Phase 5 (tests 96bc864/aacc8ab, feat 67a3b64); includes the multi-page TDEF decoder extension; seam tests on both sides per LESSONS #2)
- H18 — Ribbon bar — spec-driven Access-style ribbon — done (step 5.2 (test 483bca0, feat 14ec049))
- H19 — Navigation pane — snapshot-driven object list — done (step 5.3 (feat a706624))
- H20 — Tabbed document workspace — strip and placeholders — done (step 5.4 (test 4a3a280, feat 015459d))
- H22 — Shared grid core — virtual-scroll and hit-test math — done (step 6.1 (test 62820e4, feat c256a10))
- H21 — Compose the real Jet4Port in the UI binary (composition root) — done (step 6.2 (feat 8410069): App::new takes the port; main.rs composes Jet4Port as the documented composition root)
- H23 — Datasheet view — port read_rows, row mapping, real composition — done (step 6.2 (tests 704fc10/e606a07/2a7d3be, feats 8b97c26/8410069))
- H24 — Table design view — read-only design grid and kind display names — done (step 6.3 (core 3e6fe9a, ui 06c2982); design-time schema editing deferred to H9 (documented in the UI))
- H25 — Relationship mapper pane — port relationships and canvas — done (step 7.1 (feat a7e2a6f))
- H26 — QBE grid — query model, commands, sidecar persistence — done (step 7.2 (feat 7cb66ba))
- H27 — Query engine integration — predicates, execution, result view — done (step 7.3 (feat ec62b7b))
- H28 — WYSIWYG canvas — snap-to-grid math and form skeleton — done (step 8.1 (feat 414499e))
- H29 — Access control types — palette, placement commands, drag — done (step 8.2 (feat e959928))
- H30 — Form runtime — data binding, record navigation, sidecar file IO — done (step 8.3 (feat 7a189b8); report canvas deferred as H31)
- H32 — MCP stdio endpoint — serve loop and datacraft-mcp binary — done (step 9.1 (feat 617e259))
- H33 — Headless E2E harness — full stack over the wire — done (step 9.2 (test 245b49a))
- H8 — CLI client as fourth headless command-bus client — done (step 9.4 (test 917174f, feat d31c759))
- F1 — Ribbon spec/mapping seam was unguarded — Create tab panic — done (fixed immediately: total command_for mapping + seam test in ui crate (next commit); no unreachable! left in paint code)
- H9 — Jet4 writer workstream (create tables, write rows) — done (Phase 10, steps 10.1-10.3 (checkpoint 73f540b): writer core + create/insert + round-trip acceptance; Access-repair gate documented as manual follow-up)
- H31 — Report canvas — print layout and preview — done (Phase 10, steps 10.4-10.5 (checkpoint 1440c64): print-geometry designer + preview runtime; grouped/aggregate bands deferred to M1)
- H34 — Final documentation and release prep — done (Phase 10, step 10.9 (checkpoint 9468b51): README/ROADMAP/ARCHITECTURE/FORMAT-JET reflect the Phase 1-10 feature set)
- L1 — Jet 3 (Access 97) read support — done (Phase 10, step 10.6 (checkpoint e1627cb): Jet3 read through the same boundary; jump-table rows = documented Unsupported remainder; real A97 fixture unpinned)
- L2 — Jet4 column type 0x11 (MSysAccessObjects.Data) — study and decide mapping — done (Phase 10, step 10.7 (checkpoint 8faeac3): 0x11 = JetType::LongBinary (fixed OLE compound docs); oracle skip documented)
- T3 — Fixture pin integrity — restore pristine testV2000, keep Access-touched variant, manifest + CI hash gate — done (Phase 10, step 10.8 (checkpoint aafb794): pristine restored (126bad49…), touched variant kept, manifest + CI hash gate, immutability rule in TESTING.md)
- H35 — SQL-Integration: SQLite-Datenquelle (Backend, Detection, Format) — done (Phase 13.1 — SqlitePort (catalog, affinity schema, rows, FKs), DetectingPort magic sniff, DatabaseFormat::Sqlite; tests ca61638, feat 5a04e94)
- H36 — SQL-Integration: SQLite-Writes + Row-Insert im Command-Bus — done (Phase 13.2 — StoragePort::insert_row, Command::InsertRow/Event::RowInserted with unit validation, SqlitePort create/drop/insert; tests 99ddb0b, feat 390c79c)
- H37 — SQL-Integration: SQL über alle Quellen (In-Memory-Engine + UI) — done (Phase 13.3 — shadow SQL engine (one SELECT/WITH over materialized sources), RunSql/SqlRan, SQL window UI; tests 1a58292, feat 0400f63)
- H38 — ACCDB-Fixtures pinnen (ACE 2007/2010/2016, manifest + Hash-Gate) — done (Phase 14.1 — pristine ACE 2007/2010/2016 fixtures pinned, manifest + hash gate, provenance in FIXTURE-SOURCES; commit aef9b8f)
- H39 — ACCDB-Read-Pfad (Catalog/TDEF/TDAT über den Jet4-4096-Pfad) — done (Phase 14.2 — ACE read path through Jet4 4096; oracle match (mdbtools 1.0.1), format through all clients, precise header errors; commit d6a3a66)
- H40 — Jet4 Update/Delete-Writes (RowOp am echten Dateipfad) — done (done in `ea76d2d` (step 11.10; in-place/repack updates, 0x8000 deletes, precise Unsupported for overflow/LVAL/oversized rows, port + bus commands))
- M2 — Journaled/restringierte Write-Profile (Preview, Bestätigung, Feldfamilien) — done (done in `dcb6852` (step 11.11; preview→apply→rollback journal, atomic Jet batch, field-family profile))
- H41 — Expression-Service (Ring 1 / Roadmap M1) — done (15.1 (b6c7c73 RED, a657762 GREEN) + 15.2 (7eb3bbc RED, 6cb634d GREEN): evaluator suite, NULL/coercion contracts, defaults/rules/calculated wiring, RunExpression over the bus and grouped/aggregate report bands)
- M3 — Native Saved Queries (MSysQueries → QueryDef) — done (15.3 (78203c3 RED, d914484 GREEN): MSysQueries decoder with all nine flavors, native queries in the nav pane, read-only QBE tab, mappable-subset Run, documented rests (parameters/criteria/joins/grouping), queryTestV2000.mdb pinned (51bd3c5))
- F2 — Jet3 database passwords are plain text (M5 fact correction) — done (done in 9b05ab7)
- M4 — Complex Values/Multivalue (Attachment-Subtabellen real decodieren) — done (done in 6d96561 (RED 5480122))
- M5 — Verschlüsselung/Passwort (RC4-Decode, Passwortpfad UI/CLI/MCP) — done (done in fd30e5a (RED 3a3bb15; Jet3 password fact fix 9b05ab7))
- L3 — Jet3-Reste (Jump-Table-Zeilen >255 Bytes, verkettete TDEFs) — done (done in a8df705 (RED bd8663a))
- M6 — Doku-Genauigkeit (Kompatibilitätsaussagen ↔ verifizierter Stand) — done (verified compatibility matrix + Phase 17 docs accuracy, commit 19854e6)
- L4 — Makro-Inventur (Ring 3 / Roadmap M5) — done (read-only macro inventory, commit 8da3acb)
- L5 — VBA-Extraction + Viewer + statische Analyse (Roadmap M6) — done (VBA extraction/viewer (ccf2107) + static analysis (63ec95c), no execution)
- I1 — i18n kernel in the UI crate — done (dbc513a)
- I2 — All UI literals through t() — done (b616e9c)
- I3 — German catalog complete — done (6486786)
- I4 — App settings store (ui.json) — done (d898120)
- I5 — Language switcher in the UI — done (745f84f)
- U1 — Theme rebase: WordCraft-style neutral tokens, Teal accent — done (done in `c9ea70e` (RED `849c428`) — neutral `Tokens`/`GEOMETRY`, teal accent derivation pinned by tests, all paint sites rebased; before/after screenshots stay with the Phase 20 gate (20.5))
- U2 — Typography: Inter + type rungs — done (done in `aef71e9` (RED `e8a6c6c`) — Inter 4.1 and JetBrains Mono 2.304 embedded unmodified with SIL OFL licenses and attribution; rung scale installed in both themes; German layout budgets now measured on the shipped fonts)
- U3 — Original vector iconography (ribbon, nav pane, tabs) — done (done in `824f546` (RED `0c2ca4e`) — one name-keyed painter dispatcher in `icons.rs` with `file`/`data`/`objects` section painters; every ribbon action id and object kind is dispatched and paints; ribbon buttons, nav-pane rows and document tabs carry icons; AccessKit labels preserved)
- U4 — Dark mode: Appearance Light/Dark/System — done (done in `0866cd9` (RED `731155a`) — Appearance Light/Dark/System (default System) persisted in `ui.json`, both palettes installed per theme, tokens picked per frame, dark teal pendant #3FA799 with WCAG contrast tests for both palettes; Linux live-follow polls the XDG portal every 4 s via `gdbus` with a tested parser)
- F3 — F3 — Empty workspace never painted the canvas token — done (fixed inline in `612b114` during the Phase 20 gate — the empty workspace now paints the canvas token in both palettes)
- U13 — ui.json hardening: atomic save + full window geometry — done (done in `1462e99` (atomic save, RED `5c2be6b`) and `21044b2` (geometry + rescue, RED `aabd453`) — sibling temp file renamed over the target, legacy files still load, position/maximized round-trip, off-screen positions rescued onto the visible monitor; runtime smoke test: (9000,9000) → (820,380))
- U5 — Contextual ribbon tabs + „Views" split-button — done (done in `de70a21` (RED `904cc7c`) — contextual tabs (Table/Query/Form/Report Tools) spec-driven in core after Database Tools; `SetTabMode` and `TabSnapshot.mode`/`has_design_view` move the view mode into core state; the Home Views group renders the split-button and the tab-strip toggle is removed)
- U6 — Quick Access Toolbar — done (done in `0050cf2` (RED `5900968`) — QAT strip from core `quick_access` entries above the ribbon; Save dispatches through the shared `command_for` mapping, Undo/Redo stay disabled with a tooltip naming follow-up U14; undo/redo icons and German labels added; UI parity scanner now walks submodules)
- U7 — File backstage page — done (done in `9041874` (RED `3e12e06`) — full-page backstage with Home/Open/Save/Info/Options; open/close/new-table/sidecar flows dispatch their existing commands, the path fields left the ribbon, and the language/appearance switchers moved to Options; Recent Databases deferred to follow-up U15)
- U8 — Datasheet UX: column resize, header sort, row shading — done (done in `78e496b` (RED `165539c`) — drag-resizable columns with core hit-test/min-width/padding math, header-click and context-menu sorting through one SetQbeCell per action, alternating row shading plus the accent cursor marker; widths are per-session App state shared by the new-record row and the SQL window)
- U9 — Nav pane: category dropdown, context menu, group sort — done (done in `05dd600` (RED `6371c93`) — localized single-entry category dropdown, Open/Design/Delete context menu replacing the always-visible ✕, and name/type group sorting via the pure `nav::sort_entries`; Design opens in one command (`OpenObjectInMode`) with wire coverage)
- U10 — Tabs: middle-click close, context menu — done (done in `f2ae2a1` (RED `7e3c5f8`) — middle-click closes a tab; Close / Close Others / Close All context menu with the latter two as bus commands (CloseOtherTabs/CloseAllTabs) and JSON-RPC wire tests; closed tabs drop their presentation caches)
- U11 — Window title & status-bar conventions — done (done in `f049073` (RED `93c1760`) — `<db> : Database (<format>)` viewport title localized in both languages, sent only on change and cleared to the app name on close; status bar gains a disabled filter placeholder)
- U12 — Keyboard: shortcuts & ribbon key tips — done (done in `33f6e23` (RED `e108ae7`) — Ctrl+S/Ctrl+W/Esc dispatch/bind the same single actions as their buttons; Alt shows spec-derived key tips (Tabs phase selects, Commands phase badges the actions); derivation (`core::keys::key_tips`) and shortcut mapping are unit-tested)
- T4 — CLI-Bin-E2E über echten Subprozess (assert_cmd) — done (done in `c382d2f` (RED `ec77a8d`) — real-subprocess E2E with assert_cmd/assert_fs/predicates covers open/exec/non-interactive repl, the 0/1/2 exit-code matrix and stderr precision; binary routes failures/usage to stderr; TESTING.md registry #2 re-evaluated)
- F4 — Datasheet/design looked tables up by the wrong test id — done (fixed inline in `0e86d46` during step 23.2 — datasheet/design now resolve tables by name (case-insensitive) instead of comparing incompatible test-id schemes)
- T5 — egui/eframe headless UI-Tests (egui_kittest, Layout-Regressionen) — done (done in `0e86d46` (RED `3831560`) — datacraft-ui is a lib + thin bin; egui_kittest 0.37 drives ribbon/nav-pane/datasheet/SQL window/backstage via AccessKit, with committed insta semantic-tree snapshots (no pixel compares); TESTING.md registry #1 and taxonomy updated)
- M10 — Native-query decode errors are dropped at open — done (done in `3c7b1fb` (RED `5fb6f96`) — native-query decode errors are stored as `native_queries_error` in the snapshot instead of silently yielding an empty list; the Northwind attribute-12 error is pinned at the port and rendered at the Queries group)
- M8 — Jet AutoNumber counter maintenance on insert — done (done in `af07979` (RED `39e95ad`) — Jet4 writer assigns LongInteger AutoNumbers (Null = engine-assigned), persists the TDEF `+12` counter across reopen and rejects user values; GUID flavor stays Unsupported; mdb-export oracle test added)
- M7 — Jet table drops through the writer (DropTable on .mdb/.accdb) — done (done in `24f5033` (RED `e3c4601`) — writer/drop path deletes the catalog row, frees used-map + TDEF-chain + unreferenced usage pages into the global free map and 0xFF-fills freed pages; port tests cover reopen visibility, neighbour survival and failure atomicity)
- M9 — Design-time schema editing (ALTER TABLE) in the table design view — done (done in `6780b3c` (RED `588fe4f`) — AddField/DropField/RenameField/SetFieldProperties via one `alter_table` port method; TDEF rewrite on a scratch writer with row re-encoding (dense column ids on drop), design-grid edit controls dispatch one command each)
- H42 — ACCDB-Write-Verträge (create/insert/update/delete/drop/alter auf .accdb) — done (done in `7e7f755` (RED `3b4a4fb`) — ACE 2007/2010 write contracts green (insert/update/delete/create/drop/alter with byte preservation and map consistency, 2016 fixture as read-only reference); office-encrypted ACCDB now rejected at the writer boundary)
- H43 — ACCDB-Port-Write-Through + Bus-E2E an .accdb — done (done in `d53c054` — ACE write-through proven at the port (create/insert/update/delete/alter/drop on testV2010 copies, reopen persistence, format stays Ace), over the JSON-RPC wire (server_e2e_ace.rs) and through the CLI vocabulary; no format gate existed to open (pinned fact))
- H44 — Ship sample databases and templates (Phase 29) — done (done in `34dbba7` (create `35b2895`, templates `c09d192`, samples `ea98127`, auto-load/backstage `34dbba7`) — CreateDatabase (Jet4 + SQLite), CreateDatabaseFromTemplate over the deterministic core registry, six shipped samples with byte-compare generator check and manifest, sidecar auto-load with sidecar_error, backstage New/Samples pages; the manual Access-repair gate of the shipped samples is a documented follow-up (README matrix))
- M17 — Driver model: declarative capabilities, registry and core-computed affordances (Phase 30) — done (done in `2d73dae` (RED `a30037c`/`42296a4`; jet `14c6702`+`57d754f`, sqlite `be1a8d2`+`f080b4a`, registry `909c5ca`+`626099c`, rendering pin `c5427ab`) — DriverId + per-session DriverCapabilities/Support on StoragePort, snapshot carries driver/capabilities/availability, early dispatch rejection, registry-driven composite with with_fallback, capability-disabled UI affordances (ribbon/nav-pane/datasheet) with German reasons and kittest pins)
- M18 — CSV directory driver as the out-of-tree proof (Phase 30) — done (done in `5cee066` (RED `0a86718`; gates ① workspace member ② csv dependency released by the human 2026-11-02) — crates/datacraft-csv depends only on datacraft-core: directory = database, *.csv = tables, one atomic rewrite per write (temp + rename); DatabaseFormat::Csv with its single Display arm; composed at the UI/MCP/CLI roots via DetectingPort::with_fallback; server_e2e_csv pins the full stack over the wire)
- M12 — Native Query-Execution (MSysQueries ausführen, begrenzter Dialekt) — done (done in `3fd07a9` (RED `02b091a`; coverage `d668c7b`/`bf872eb`) — native selects execute the qualified Access dialect: equality joins (inner/left/right), WHERE expressions via the phase-15 service, Table.* projection and ORDER BY; fixture SelectQuery runs over the real port, UI Run is kittest-pinned, everything else answers a precise Unsupported)
- M14 — Encrypted Save-As + Passwortwechsel (verschlüsselte Kopien) — done (done in `8b2dccc` (RED `9614c5a`; coverage `bf872eb`) — SaveDatabaseAs writes unencrypted/protected Jet4 copies with a deterministic page key while the source stays untouched; ChangePassword rewrites in place atomically; ACE keeps its Office-encryption boundary and Jet3 stays read-only; backstage/CLI/JSON-RPC flows and capability gating are pinned)
- U14 — Command history for Undo/Redo — done (done in `a4ca640` (RED `926cbca`) — bounded per-database row-write history; Undo/Redo bus commands address rows by value image, restore_row preserves AutoNumber identities, journal/schema/file lifecycle are barriers, QAT enables from snapshot depths; Jet4 delete/restore verified across reopen, SQLite restore pinned)
- U15 — Recent databases in the backstage — done (done in `da1a0e8` (RED `2ad9d26`) — ui.json persists a bounded, de-duplicated recent list on DatabaseOpened; stale paths are pruned on load with a localized notice; the backstage Home/Open pages dispatch one OpenDatabase per click; kittest and updated Home snapshot pin the flow)
- M20 — Journal batches gate on the declared atomic_batch capability (Phase 30 follow-up) — done (done in `fc2b623` (RED `905ed72`) — preview_write and apply_write refuse multi-op batches on drivers with atomic_batch=false with the precise one-at-a-time reason; single-op batches stay available; Jet's atomic path unchanged; core and SQLite contracts pin both sides)
- H45 — Persist field properties (defaults, validation, captions, formats, input masks, index flags) — done (done in `c5abd00` (RED `c5176bd`) — the .datacraft sidecar is the property authority (ADR-1 addendum): tables are saved with the full FieldProperties sheet, overlaid on explicit load and sibling auto-load with precise stale/invalid handling, preserved across Jet/ACE schema refreshes; the Jet writer keeps NULL LvProp (Access display deferred to the LvProp roadmap item); design view renders the restored sheet (kittest))
- M15 — Index creation in the Jet writer — done (done in `c85b1b1` — TDEF index metadata encode/decode (physical + logical blocks, names), domain IndexMode round-trip, uniqueness on insert/update/batches, alter add/remove/renumber with duplicate guard, SQLite CREATE [UNIQUE] INDEX; physical Jet index pages remain the roadmap follow-up)
- M16 — Relationship and foreign-key creation — done (done in `1729228` — CreateRelationship/DeleteRelationship bus commands; Jet lazily creates MSysRelationships + parent/child indexes and writes/deletes rows; SQLite rebuilds the child table with enforced FOREIGN KEYs (orphans/duplicates refused); Jet/SQLite capabilities flip to Yes, CSV stays No)
- L6 — Ship samples next to the binary and in release archives — done (done in `7ba87f1` (step 26.1) — release.yml copies samples/ into every archive and the packaged smoke opens the bike-shop demo from the extracted archive; resolution order documented at backstage.rs)
- M13 — Release-Engineering (Tags, release.yml, Checksums, Smoke) — done (done in `7ba87f1` + `d3a008c` (steps 26.1/26.2) — release.yml matrix/packaging/per-artifact smoke/checksums, draft-release publishing via gh, PLAYBOOK §9 tag policy; local packaged smoke green on windows (jet fixture, samples demo, csv directory))
- M21 — Retire the vestigial StorageEngine trait (ADR-7 single seam) — done (done in `45a852d` + `44b1fff` (step 31.1) — trait/mock/boundary suite removed after porting the contract to tests/port_contract.rs (Jet4 + SQLite cross-driver); coverage >= 95 held; ARCHITECTURE §6 cites the removal; finding M24 (tri-state Boolean NULL) filed from the porting run)
- M22 — Publish-ready API vocabulary: non_exhaustive and the capabilities default (Phase 31 prep) — done (done in `5dad505` — driver_id/capabilities required, none()/full() constructors, Support/PortError/Command non_exhaustive, ADR-7 addendum item 6; datacraft-csv compiles unchanged (acceptance proof); deviation documented: DriverCapabilities keeps constructible pub fields (non_exhaustive struct would break field reads))
- M19 — Engine/driver publishing: crate split, standalone MCP binary, Semver policy (Phase 31) — done (done in steps 31.2-31.4 (`10689e6`, `360028e`, `49b1456`) — jet/sqlite/mcp crates extracted, storage is the composite, core gained the default-on sql-engine feature gate (jet is C-free), publishing order + semver/msrv/changelog policy documented in docs/PUBLISHING.md; workspace builds as one unit, no behavior change)
- L9 — Composite fallback keeps the unknown-file format hint — done (done in `3e68e2f` (step 36.2) — the composite attaches the sniff's family hint to the fallback's rejection; pinned by detecting_edges.rs (unknown file names path, expected families and the fallback verdict); unknown paths without a fallback behave unchanged)
- T7 — Coverage gate red on main (csv/detecting/mcp) — done (done in steps 36.1-36.3 (`dbb3ae8`, `3e68e2f`, `8d3e348`) — csv 66.8% -> 96.6%, detecting 80.7% -> 96.9%, mcp bin covered by the subprocess suite (100% locally; counted without exclusion per the human's call); CI run 41653027497 fully green including the coverage gate)
- M23 — Registered drivers visible in the snapshot — done (done in `c4bb804` (step 36.4) — DriverInfo + driver_inventory()/source_format() vocabulary, inventory fixed at Core::new, composite lists built-ins + fallback; the real mcp binary lists jet/sqlite/csv over the wire (pinned by mcp_process.rs); wire change additive, capability invariants unaffected)
- L7 — CSV type inference beyond Short Text — done (done in `bd71194` + `bf175e9` (step 36.5) — deterministic full-scan inference (LongInteger/Double/DateTime/Short Text), one quoted cell pins the column to text, empty cells don't vote, strict spellings; values read as text, only the schema kind inferred; rule documented in the crate docs + README row; pinned by tests/inference.rs (6 suites))
- M24 — Jet writer cannot encode tri-state Boolean NULL — done (decided 2026-11-03 (step 36.6, `de1feb4`): option B2 — the verified knowledge base documents no second-bit tri-state encoding (option A = clean-room violation), a creation-time rejection would break the pinned H44 samples/templates (option B1); creation and reading keep working, the NULL write answers a precise boundary message naming column/consequence/alternative; documented in FORMAT-JET 'Domain mapping', pinned by jet_row_writes; SQLite keeps tri-state NULL — the cross-driver difference is deliberate and documented)
- H46 — Jet read/write paths panic on crafted structures instead of answering a precise error — done (fixed in `0dd8d43` (RED `dfe2fe0`): per-type widths in decode_bytes, checked var-table ranges in the memo probe, checked LVAL/map slices, column-id mask validation and u16 eod guard in encode_row, checked free-map window growth; ACE offset coordinates stay permissive (legacy bits on real pages) and the consumers bound them)
- H48 — Stale write journals survive the database lifecycle and can write into the next database — done (fixed in `c50c248` (RED `4d5db2e`): open/close clear pending_write and last_applied; a stale token answers the precise "no pending/applied journal" failure and leaves the reopened database untouched)
- H47 — Database and sidecar writes replace the live file non-atomically — done (fixed in `e138361` (RED `0a37cb1`): datacraft_core::atomic_write (sibling-temp replace, exclusive create) used by the Jet write-through, file/template creation, Save-As, password change and both sidecar saves; a failed replace leaves the live file byte-identical and a failed create leaves no partial)
- M25 — SQLite create_table leaves an open transaction when CREATE INDEX fails — done (fixed in `e1b18c2` (RED `47ef985`): the error path of the DDL batch issues ROLLBACK, the connection stays usable and the half-created table is gone)
- M27 — CreateDatabaseFromTemplate steals the open database's port session — done (fixed in `4da8d63` (RED `618f720`): the command is refused while a database is open ("close the open database …"), so the live port session can never be replaced; a later fresh-session API can lift the restriction)
- M28 — Non-finite Jet floats break JSON-RPC serialization — done (fixed in `9e9bc56` (RED `1637690`): Single/Double cells with NaN or Infinity answer a precise Corrupt error naming the column; the read path now enforces the same finite-float rule as the write path, so no non-finite value can reach serde_json)
- M26 — Crafted memo and VBA lengths drive unchecked allocations — done (fixed in `e47edbe`: memo-chain capacity capped by the input size, VBA chunk limit enforced per token; existing hostile-length suites pin the error behavior (the bound itself is not observable))
- L12 — CSV create_table skips TableDef validation — done (fixed in `9d4d865` (RED `788f739`): CSV create_table validates the definition before writing; an invalid header leaves no file and the directory still opens)
- L14 — Capability gates missing on the journal and template commands — done (fixed in `9d4d865` (RED `788f739`): preview/apply/rollback gate on write_rows and template creation on create_database, all before the port; the capability reason is the stable reply)
- L15 — Undo history is cleared even when the barrier command fails — done (fixed in `9d4d865` (RED `788f739`): the undo barrier fires only when dispatch produced no failure event, so a rejected barrier command keeps the history)
- L16 — DropTable leaves relationships that reference the dropped table — done (fixed in `9d4d865` (RED `788f739`): DropTable prunes relationships whose child or parent is the dropped table, so the snapshot carries no dangling edge)
- L19 — rustdoc: missing public docs and broken intra-doc links — done (fixed in `66afb36`: six variant docs added, seven doc links repaired, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` exits 0 and is now a release-checklist step in PUBLISHING)
- L20 — Core public surface: empty public modules and an undocumented MockPort API — done (fixed in `7ee7954`: the impl-only modules are private, and PUBLISHING documents MockPort as a supported testing surface (not a production backend))
- M32 — Wire vocabularies beyond M22 lack non_exhaustive — done (fixed in `011bc76`: Event, DisabledReason, DatabaseFormat and SchemaChange are non_exhaustive; Jet and UI wildcards answer precise/localized fallbacks; PUBLISHING lists the vocabulary and states the read-only snapshot contract)
