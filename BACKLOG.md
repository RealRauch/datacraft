# BACKLOG.md — Open items (As of: 2026-11-02T01:29

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

### [ ] H45 — Persist field properties (defaults, validation, captions, formats, input masks, index flags) — 🟠
- **Location:** `crates/datacraft-storage/src/jet/writer_tables.rs`, `crates/datacraft-storage/src/port_write.rs`, `crates/datacraft-storage/src/port_schema.rs`, `crates/datacraft-core/src/forms.rs` (sidecar tables)
- **Problem:** `create_table` writes `NULL` into `MSysObjects.LvProp`, and `port_schema.rs` reconstructs only the column type plus `required`. Defaults, validation rules, captions, formats, input masks and `Indexed` are lost on every reopen. The sidecar saves `tables: Vec::new()` and ignores `document.tables` on load, so it cannot carry them either. Templates therefore ship as structure + data only.
- **Approach:** Decide the authority (ADR): Jet `LvProp` property blocks so Access sees the properties, or the sidecar's table list as DataCraft's own metadata. Implement write + read round-trip for every modeled `FieldProperties` entry, keep the file Access-readable, and reject unsupported property encodings precisely.
- **Acceptance:** Every modeled property survives a Jet4 reopen (and/or sidecar load); Access displays defaults/validation/captions; tests pin each property and the rejection cases; no silent loss remains.

---

## 🟡 MEDIUM

> No open items.

### [ ] U14 — Command history for Undo/Redo — 🟡
- **Location:** `crates/datacraft-core/src/state.rs`, `command.rs`, dispatch modules; `crates/datacraft-ui/src/ribbon.rs` (QAT enablement)
- **Problem:** The Quick Access Toolbar declares Undo/Redo (step 21.2, item U6) but renders them disabled: the command bus keeps no history, so there is nothing to undo.
- **Approach:** a bounded per-database history stack of applied commands with deterministic inverse operations (the M2 journal already produces inverse write ops); `Undo`/`Redo` are ordinary bus commands that pop and push through the same reducer, so GUI, CLI and MCP share them; the snapshot exposes the history depth and the QAT entries enable from it.
- **Acceptance:** Undo after a row insert/update/delete restores the previous row data (verified by reopening the file); Redo re-applies it; each action is exactly one command over the wire; QAT entries enable/disable from the snapshot; canonical verification set green.

### [ ] M12 — Native Query-Execution (MSysQueries ausführen, begrenzter Dialekt) — 🟡
- **Location:** `crates/datacraft-core/src/` (QueryDef aus nativen Queries → Engine-Ausführung), `crates/datacraft-storage/src/port_native.rs` (falls Attribut-Erweiterung), UI: QBE-Ausführung nativer Queries
- **Ansatz:** Native Saved Queries sind lesbar (M10) — jetzt **ausführbar** über die eigene Engine: QueryDef (Tabellen/Spalten/Sort/Criteria, Joins im begrenzten Umfang) wird in die In-Memory-Engine überführt und über den Bus ausgeführt (`RunQuery`-Pfad); Konstrukte außerhalb des qualifizierten Dialekts (Ausdrücke, Parameter, Unterabfragen) → präziser `Unsupported` mit Grund statt stiller Fehler.
- **Acceptance:** Fixture-Queries aus `queryTestV2000.mdb` laufen durch die Engine (Ergebnisse = QBE-Äquivalent); `Unsupported`-Grundfälle getestet; UI führt native Queries aus; MCP/CLI erben über den Bus.

### [ ] M13 — Release-Engineering (Tags, release.yml, Checksums, Smoke) — 🟡
- **Location:** `.github/workflows/release.yml` (neu), `Cargo.toml` (Versions-Politik), `docs/PLAYBOOK.md` (Release-Abschnitt), Third-Party-Notices
- **Ansatz:** Tag-getriggerter Release-Workflow (Mirror der CI-Gates): Build von `datacraft`, `datacraft-cli`, `datacraft-mcp` für Windows/macOS/Linux (statisch gebundeltes rusqlite über `bundled`), Archive mit sha256-Checksums + Third-Party-Notices; Smoke je Artefakt (CLI öffnet ein Fixture headless, `--version`); erste Tags `v0.1.x` nach Gate.
- **Acceptance:** Workflow-Datei lintet (YAML/Actions-Konventionen wie ci.yml); lokaler Release-Smoke (Release-Build des CLI öffnet `testV2000.mdb` headless); Tag-Politik dokumentiert; kein Binary im Repo.

### [ ] M14 — Encrypted Save-As + Passwortwechsel (verschlüsselte Kopien) — 🟡
- **Location:** `crates/datacraft-storage/src/port_adapter.rs` (`with_writer` speichert bereits verschlüsselt zurück — Schritt 16.2), `crates/datacraft-core/src/command.rs` (neu: Save-As/Passwortwechsel), UI Backstage, CLI/MCP-Flags
- **Ansatz:** Der Write-Pfad re-encryptet beim Speichern bereits korrekt (M11). Rest: **Save-as** — Kopie unter neuem Pfad (optional mit anderem/gesetztem Passwort: unverschlüsselt → verschlüsselt, Passwortwechsel) als Command über den Bus; alle vier Clients bedienen es; präzise Fehler bei falschem Altpasswort.
- **Acceptance:** Save-as auf unverschlüsselte/verschlüsselte Kopie getestet (Reopen mit/ohne Passwort); Passwortwechsel-Tests; UI/CLI/MCP-Fluss; Original bleibt unangetastet.

### [ ] M15 — Index creation in the Jet writer — 🟡
- **Location:** `crates/datacraft-storage/src/jet/encode.rs` (`num_idx`/`num_real_idx`), `crates/datacraft-storage/src/jet/writer_tables.rs`, `crates/datacraft-storage/src/port_write.rs`, `crates/datacraft-storage/src/sqlite/mapping.rs`
- **Problem:** `encode_tdef` writes zero index blocks; `FieldProperties.indexed` is serialization/UI metadata only. Generated tables and samples cannot demonstrate indexed lookups or uniqueness, and a template's `Indexed` flag does nothing.
- **Approach:** Encode TDEF index descriptors and index pages for `IndexMode::{DuplicatesOk, NoDuplicates}`, maintain them on insert/update/delete, and emit `CREATE INDEX`/`UNIQUE` for SQLite. Keep existing index blocks readable and untouched on unrelated writes.
- **Acceptance:** Indexes survive reopen and are visible in Access; uniqueness is enforced with a precise error; SQLite emits the matching DDL; tests cover create/insert/update/delete and the unsupported shapes.

### [ ] M16 — Relationship and foreign-key creation — 🟡
- **Location:** Jet writer (`MSysRelationships` rows and the required indexes), `crates/datacraft-storage/src/sqlite/mapping.rs` (foreign-key DDL), `crates/datacraft-core/src/port.rs` (create/delete relationship surface)
- **Problem:** Relationships are read-only on both backends. Generated samples cannot populate the relationship mapper, and a template cannot express referential integrity.
- **Approach:** Add a create/delete relationship path through the port and a matching bus command. Jet writes `MSysRelationships` plus the required primary/foreign indexes; SQLite emits `FOREIGN KEY` constraints. Reads must round-trip through the existing `relationships()` API.
- **Acceptance:** Created relationships survive reopen and appear in the mapper; Access shows them; SQLite enforces the constraint; tests cover create/delete, duplicates and unsupported shapes; canonical gates green.

---

## 🟢 LOW

> No open items.

### [ ] I6 — Failure-reason localization (post-V1) — 🟢
- **Location:** `crates/datacraft-core/src/core.rs` + Dispatch-Module (erst stabile Failure-Codes), `crates/datacraft-ui/src/i18n/` (UI-seitige Zuordnung); ADR-4
- **Problem:** `Event::Failed.reason` strings are English on the wire (~120 sites); localizing them inside core would put language into core (ADR-4 violation), so V1 keeps them English — agents key on data, not prose.
- **Approach:** two steps, post-V1: (1) core gains stable machine-readable failure codes (enum or code prefix) alongside the English text — a domain concern, not a setting; (2) the UI maps known codes to localized text via its catalog; unknown reasons render in English unchanged.
- **Acceptance:** every dispatch failure carries a stable code; the UI renders localized text for known codes and English otherwise; the wire gains codes without breaking existing consumers; English-substring tests (command_errors.rs, cli_contract.rs, server_e2e.rs) switch to code assertions.

### [ ] U15 — Recent databases in the backstage — 🟢
- **Location:** `crates/datacraft-ui/src/settings.rs` (persisted list), `crates/datacraft-ui/src/backstage.rs` (Recent page)
- **Problem:** The File backstage (step 21.3, U7) has no Recent Databases entry: `ui.json` (I4) persists language, appearance and window geometry, but no opened-file history. U7's scope named Recent "once I4 lands"; I4 shipped without the list, so the page is deliberately absent.
- **Approach:** persist a bounded, de-duplicated recent-paths list on successful `DatabaseOpened` (atomic save via U13's helper), render it on the backstage Home/Open page, and dispatch one `OpenDatabase` per click; missing files are pruned on load with a precise notice instead of disappearing silently.
- **Acceptance:** opening a database adds it to the recent list; the list survives restarts; clicking an entry dispatches exactly one open command; stale paths are pruned visibly; canonical verification set green.

### [ ] L6 — Ship samples next to the binary and in release archives — 🟢
- **Location:** release packaging (Phase 26, M13), `crates/datacraft-ui/src/samples.rs` (runtime resolution)
- **Problem:** The backstage Samples page resolves `samples/` from `DATACRAFT_SAMPLES_DIR`, the executable directory, or the dev workspace. Installed builds have no installer step that places the folder next to the binary, so the page falls back to a path that does not exist.
- **Approach:** Copy `samples/` into every release archive next to `datacraft`/`datacraft-cli`/`datacraft-mcp`, document the resolution order, and add a smoke check that an extracted archive finds and opens the demo.
- **Acceptance:** Release archives contain `samples/`; the backstage page resolves them without repo paths; a packaged smoke test opens the demo; no binary or sample is committed into the repo twice.

---

## 🔵 TEST GAPS

> No open items.

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
