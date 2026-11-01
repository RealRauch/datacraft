# BACKLOG.md — Open items (As of: 2026-11-01T19:43

> **This file contains only OPEN items.** Completed items are moved to `docs/archive/BACKLOG_ARCHIVE.md` unchanged after completion; this file keeps only a one-liner per item in the Done Index (below). Findings/fix-ideas/decisions are not deleted — moved to the archive.
> Legend: 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · 🔵 test gap
> Working order: sequential by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), each item test-first, commit per item.

---

## 🔴 CRITICAL

> No open items.

---

## 🟠 HIGH

> No open items.

### [ ] U1 — Theme rebase: WordCraft-style neutral tokens, Teal accent — 🟠
- **Location:** `crates/datacraft-ui/src/theme.rs` (token system), paint sites referencing theme constants; ADR-6 in `docs/ARCHITECTURE.md` §15; model: WordCraft `crates/ui-egui/src/theme.rs`
- **Approach:** token-based theme adopting the model's **neutral** system: chrome/title bar `#F0F0F0`, ribbon `#F9F9F9`, canvas `#E6E6E6`, panel `#FAFAFA`, text `#242424`/`#616161`/`#A8A8A8`, borders `#E1E1E1`/`#C8C8C8`, hover `#E6E6E6`, pressed `#D6D6D6`. **The accent family stays DataCraft teal and is derived from it — no accent colour is copied from the model (its blue is explicitly rejected):** accent `#0F8B7E`, accent_text `#0B6B60`, checked/selection `#DCE9E4` (existing TEAL_WASH), on_accent `#F8F4EA` (ON_TEAL). Geometry from the model: item_spacing (6,4), button_padding (8,3), corner radii windows 8 / menus 6 / widgets 4, 1 px strokes. The derivation rule ("accent tokens are teal derivatives, never the model's blue") is pinned by a unit test.
- **Acceptance:** all four regions render from tokens; accent-token test green (teal derivatives only); the brand icon's teal still matches the UI accent; before/after screenshots reviewed in the phase gate; canonical verification set green.

### [ ] U2 — Typography: Inter + type rungs — 🟠
- **Location:** `assets/fonts/` (neu: Inter-Regular/-Medium/-SemiBold TTF, JetBrainsMono-Regular optional — OFL, attribution file), `crates/datacraft-ui/src/theme.rs` (font definitions); model: WordCraft `theme.rs` font setup
- **Approach:** embed the fonts via `include_bytes!` and register families like the model: proportional Inter first (egui defaults stay as fallback), named families `"medium"`/`"semibold"` (InterMedium/InterSemiBold → Inter → egui defaults), mono for the SQL editor. Type rungs: Chrome 12.5 px (tracking −0.012 em), Control 11.5 px, Caption 11.0 px (+0.018 em); egui styles Body/Button 12.5, Small 10.5, Heading 17 semibold. **New font assets = Medium content gate** (license/attribution required before landing).
- **Acceptance:** all four regions use the rung scale; German strings lay out without truncation (gate check against I3's catalog); licences recorded in an attribution file; canonical verification set green.

---

## 🟡 MEDIUM

> No open items.

### [ ] M7 — Jet table drops through the writer (DropTable on .mdb/.accdb) — 🟡
- **Location:** `crates/datacraft-storage/src/jet/writer_tables.rs`, `writer.rs`, `catalog.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Problem:** `Jet4Port::drop_table` answers a precise `Unsupported` ("BACKLOG M7"), so the navigation-pane delete flow (step 11.4) works for SQLite but not for Jet/ACE files.
- **Approach:** Remove the table's TDEF/TDAT pages and its catalog row, return every freed page to the global free map and keep the per-table usage maps consistent; deterministic page selection (LESSONS #6). Verify with `mdb-export` and a reopen; the SQLite drop path stays unchanged.
- **Acceptance:** Drop on a private fixture copy; reopen and the catalog no longer lists the table; free-map and usage-map consistency checked; the oracle reads the file without errors; failure leaves the file untouched.

### [ ] M8 — Jet AutoNumber counter maintenance on insert — 🟡
- **Location:** `crates/datacraft-storage/src/jet/writer_rows.rs` (new or `writer_tables.rs`), `port_write.rs`; `src/port_adapter.rs`; `crates/datacraft-storage/tests/`
- **Problem:** `port_write` maps `FieldKind::AutoNumber` to a precise `Unsupported` because the writer does not maintain the table's counter; `InsertRow` on a Jet table with an AutoNumber column therefore fails.
- **Approach:** Persist the AutoNumber seed/increment in the table's counter metadata, assign the next value on insert (Null = engine-assigned) and keep the assignment deterministic per input state (LESSONS #6); GUID AutoNumbers stay `Unsupported` until qualified.
- **Acceptance:** Inserts assign 1, 2, 3…; reopen keeps the counter; `mdb-export` and Access show the assigned values; user-supplied AutoNumber values stay rejected; GUID flavor answers a precise `Unsupported`.

### [ ] M9 — Design-time schema editing (ALTER TABLE) in the table design view — 🟡
- **Location:** `crates/datacraft-core/src/command.rs`, `database.rs`; `crates/datacraft-storage/src/jet/writer_tables.rs`, `src/port_adapter.rs`; `crates/datacraft-ui/src/design.rs`
- **Problem:** The table design grid is read-only; step 6.3 deferred editing to the writer workstream (H9), but no alter path exists yet.
- **Approach:** `AddField`/`DropField`/`RenameField`/`SetFieldProperties` commands through the port, rewriting the TDEF while preserving row data; the design grid becomes editable with save/cancel and dispatches exactly one command per edit; shapes the writer cannot alter answer a precise `Unsupported`.
- **Acceptance:** Design-view edits persist across a reopen with row data preserved; oracle check on the changed file; UI edits dispatch one command each; failures leave the file and state untouched.

### [ ] M10 — Native-query decode errors are dropped at open — 🟡
- **Location:** `crates/datacraft-core/src/database.rs` (`open_database` calls `native_queries().unwrap_or_default()`), `state.rs`/`snapshot.rs`, `port_native.rs`; `crates/datacraft-ui/src/nav_pane.rs`
- **Problem:** A database whose `MSysQueries` contains an undecoded attribute (observed: attribute code 12 on `fixtures/northwind2007.accdb`) opens successfully but silently shows **no** native queries — the precise `Unsupported` reason is discarded. This contradicts LESSONS #4 and makes the compatibility matrix's "unsupported constructs answer a precise `Unsupported`" claim misleading at open time.
- **Approach:** store the decode result and its error in `DatabaseState`/`UiStateSnapshot` (`native_queries_error`), exactly like `macros_error`/`vba_error`; keep the open succeeding with an empty list; render the reason in the navigation pane's Queries group and expose it to inspect/MCP. Add a fixture-backed test on `northwind2007.accdb` pinning attribute 12.
- **Acceptance:** opening the Northwind fixture yields `native_queries` empty plus a visible `native_queries_error` containing "attribute code 12"; `inspect()` carries it; a database with decodable queries has no error; canonical verification set green.

### [ ] I4 — App settings store (ui.json) — 🟡
- **Location:** `crates/datacraft-ui/src/settings.rs` (neu), Config-Dir-Auflösung handgerollt (`%APPDATA%`/`XDG_CONFIG_HOME`/`~/Library/Application Support`, Fallback Home — keine neue Dependency); ADR-5 in `docs/ARCHITECTURE.md` §15
- **Approach:** serde JSON at `<config>/datacraft/ui.json` storing `language` (`"auto"`/`"de"`/`"en"`) and window geometry now, appearance later (U4). Unknown values fall back to defaults instead of failing (WordCraft `ui.json` pattern). Strictly UI-owned — settings are **never** written into the `.datacraft` sidecar or any core state (ADR-5); loaded at startup, saved on change/close.
- **Acceptance:** settings round-trip across restarts; corrupt/empty file → defaults, no panic; precedence per ADR-5 (setting > env > detection > auto); sidecar files untouched by the settings path; canonical verification set green.

### [ ] I5 — Language switcher in the UI — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (File tab group; moves into the backstage with U7), `app.rs`, `settings.rs` (I4); ADR-4/ADR-5
- **Approach:** a language menu (Auto/Deutsch/English) in the File ribbon tab; selection persists to `ui.json`; precedence Setting > Env > Detection > Auto. Switching applies immediately — `t()` reads the current language each frame (WordCraft per-frame refresh pattern), no restart required.
- **Acceptance:** switch applies live; persistence across restarts; `Auto` resolves per detection; headless test for the full precedence chain; canonical verification set green.

### [ ] U3 — Original vector iconography (ribbon, nav pane, tabs) — 🟡
- **Location:** `crates/datacraft-ui/src/icons.rs` (neu, reines Painter-Modul), `ribbon.rs`, `nav_pane.rs`, `workspace.rs`; model: WordCraft `crates/ui-egui/src/icons.rs`
- **Approach:** original egui vector painter functions per icon (open/save/close/table/query/form/report/relationships/SQL/design …), mapped from the existing stable ids (ribbon action ids, `ObjectKind`) — never invented keys. Monochrome ink strokes with teal accents, no external image assets (clean-room and asset-free besides fonts).
- **Acceptance:** every ribbon action, nav-pane object kind and document tab shows an icon; all icons live in one module and are painted at paint time; label-based addressing stays intact (kittest-ready, coordinate with T5); canonical verification set green.

### [ ] U4 — Dark mode: Appearance Light/Dark/System — 🟡
- **Location:** `crates/datacraft-ui/src/theme.rs` (dark token set + `Appearance`), `settings.rs` (I4) for persistence; model: WordCraft `desktop_theme.rs`/`theme.rs`; ADR-6
- **Approach:** `Appearance { Light, Dark, System }` (default `System`) — both palettes installed per frame and egui picks by OS preference, exactly like the model. **Dark accents use the teal pendant (≈ `#3FA799`, dark on-accent ink), never the model's light blue.** WCAG contrast tests (4.5:1 text on accent, 3:1 borders) against the teal variants; unknown persisted appearance values fall back to the default.
- **Acceptance:** System mode follows OS appearance live; persistence via `ui.json` (I4); contrast tests green for both palettes; canonical verification set green.

### [ ] U5 — Contextual ribbon tabs + „Views" split-button — 🟡
- **Location:** `crates/datacraft-core/src/ribbon.rs`, `command.rs`, `state.rs`, `snapshot.rs` (SetTabMode — Domänenzustand); `crates/datacraft-ui/src/ribbon.rs`, `workspace.rs`
- **Approach:** the Home ribbon gains a "Views" group with a View split-button switching the active tab's mode (Datasheet/Design per object kind) — the Access convention, replacing the tab-strip toggle which is removed. When an object tab is active, contextual tabs (e.g. "Table Tools") render after "Database Tools", spec-driven from core. `tab_mode` moves from the UI-local HashMap into core state (`SetTabMode` command + `TabSnapshot.mode`) so agents see and drive it — this is document state, **not** a UI setting (ADR-4 boundary respected).
- **Acceptance:** mode switch only via the Views button; snapshot exposes per-tab mode; contextual tabs appear per active kind; ribbon seam test updated; headless wire tests for `SetTabMode`; canonical verification set green.

### [ ] U6 — Quick Access Toolbar — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (QAT strip), `crates/datacraft-core/src/ribbon.rs` (QAT spec entries)
- **Approach:** a compact top strip per the Access QAT convention, spec-driven from core like every ribbon region, starting with Save Design. Undo/Redo entries are declared but rendered disabled until the command bus gains a history (explicitly out of scope here; follow-up item owns them). QAT buttons dispatch exactly the same commands as their ribbon counterparts.
- **Acceptance:** QAT renders and dispatches Save; Undo/Redo render disabled with a tooltip naming the follow-up; test ids per convention; canonical verification set green.

### [ ] U7 — File backstage page — 🟡
- **Location:** `crates/datacraft-ui/src/backstage.rs` (neu), `workspace.rs`/`ribbon.rs`, `settings.rs` (I4) für Recent; model: WordCraft `crates/ui-egui/src/backstage/`
- **Approach:** activating the File tab renders a full-page backstage view (Open, Close Database, Recent Databases once I4 lands, New Table flow entry) instead of a ribbon tab with inline path inputs; the path TextEdits leave the ribbon. Every flow dispatches the same single commands as today.
- **Acceptance:** open/close flows work from the backstage (one command each); the File tab shows the backstage page; test ids per convention; canonical verification set green.

### [ ] U8 — Datasheet UX: column resize, header sort, row shading — 🟡
- **Location:** `crates/datacraft-core/src/grid.rs` (pure math), `command.rs`/`qbe.rs` (sort mapping), `crates/datacraft-ui/src/datasheet.rs`
- **Approach:** draggable column separators (per-session widths as presentation state in the App; hit-test/min-width math as pure functions in core `grid.rs` per the §8 principle), header click/right-click sort asc/desc mapped onto the query sort model — one command per action —, Access-style alternating row shading plus an edit marker on the selected/edited row.
- **Acceptance:** columns resize by drag; header sort dispatches one command per action; grid math unit-tested in core; canonical verification set green.

### [ ] U9 — Nav pane: category dropdown, context menu, group sort — 🟡
- **Location:** `crates/datacraft-ui/src/nav_pane.rs`; pure ordering helpers in core; model: WordCraft `panes.rs` patterns
- **Approach:** the "All Access Objects" label becomes a (single-entry) category dropdown per the Access convention; objects are opened/deleted via a context menu (Open/Design/Delete), replacing the always-visible ✕ button; group headers gain a sort menu (name/type) implemented as a pure ordering function over the snapshot-derived list.
- **Acceptance:** delete is reachable via context menu only; the dropdown renders localized (rides on I2); the sort menu reorders deterministically; test ids for menu items; canonical verification set green.

---

## 🟢 LOW

> No open items.

### [ ] I6 — Failure-reason localization (post-V1) — 🟢
- **Location:** `crates/datacraft-core/src/core.rs` + Dispatch-Module (erst stabile Failure-Codes), `crates/datacraft-ui/src/i18n/` (UI-seitige Zuordnung); ADR-4
- **Problem:** `Event::Failed.reason` strings are English on the wire (~120 sites); localizing them inside core would put language into core (ADR-4 violation), so V1 keeps them English — agents key on data, not prose.
- **Approach:** two steps, post-V1: (1) core gains stable machine-readable failure codes (enum or code prefix) alongside the English text — a domain concern, not a setting; (2) the UI maps known codes to localized text via its catalog; unknown reasons render in English unchanged.
- **Acceptance:** every dispatch failure carries a stable code; the UI renders localized text for known codes and English otherwise; the wire gains codes without breaking existing consumers; English-substring tests (command_errors.rs, cli_contract.rs, server_e2e.rs) switch to code assertions.

### [ ] U10 — Tabs: middle-click close, context menu — 🟢
- **Location:** `crates/datacraft-ui/src/workspace.rs`, `crates/datacraft-core/src/command.rs` (CloseAllTabs/CloseOtherTabs)
- **Approach:** middle-click closes a tab; a right-click menu offers Close / Close Others / Close All — the latter two as new bus commands (one command per action, no UI-side loops over tabs), matching the §3 principle that every GUI action is exactly one command. Kind icons ride in with U3.
- **Acceptance:** all three actions dispatch commands; headless wire tests for the new commands; test ids per convention; canonical verification set green.

### [ ] U11 — Window title & status-bar conventions — 🟢
- **Location:** `crates/datacraft-ui/src/main.rs`/`shell.rs`, `app.rs`
- **Approach:** the viewport title follows the Access convention `{db} : Database ({format})` (localized via I2's catalog), updated on open/close without per-frame churn; with no database open the plain app name shows. The status bar gains an indicator slot (filter state placeholder) consistent with the Access status bar.
- **Acceptance:** the title reflects the open database and clears on close; localized in both languages; the status indicator renders its disabled state; canonical verification set green.

### [ ] U12 — Keyboard: shortcuts & ribbon key tips — 🟢
- **Location:** `crates/datacraft-ui/src/` (new input handling in shell.rs or a small keys.rs), `ribbon.rs` (key tip overlay); model: WordCraft `crates/ui-egui/src/keytips.rs`
- **Approach:** Ctrl+S (Save Design), Ctrl+W (Close Tab), Esc (cancel cell edit / close menu) mapped to the same single commands as their buttons; Alt reveals ribbon key tips rendered from the spec ids (never invented labels). Keyboard documentation rides with M6's README accuracy pass, not here.
- **Acceptance:** shortcuts dispatch one command each; key tips visible after Alt and match the spec; headless-testable where egui permits (coordinate with T5); canonical verification set green.

---

## 🔵 TEST GAPS

> No open items.

### [ ] T4 — CLI-Bin-E2E über echten Subprozess (assert_cmd) — 🔵
- **Location:** `crates/datacraft-cli/tests/` (neu: `cli_process.rs`), `crates/datacraft-cli/Cargo.toml` (dev-deps `assert_cmd`, `assert_fs`, `predicates`); `docs/TESTING.md` §3 Registry #2
- **Problem:** Der CLI-Bin wird heute nur über injizierte Streams (`run_with_streams`) getestet — genau deshalb steht `main.rs` mit 17 Zeilen als ungedeckte Ausnahme in der Coverage-Registry (#2, „there is nothing to assert without a subprocess"). Prozessgrenze, echtes stdin/stdout, Arg-Parsing und Exit-Codes sind ungetestet, obwohl README deterministische Exit-Codes als Feature nennt.
- **Ansatz:** Bin als echten Kindprozess starten (`assert_cmd::Command::cargo_bin`), gegen `fixtures/testV2000.mdb` sowie Scratch-SQLite (`assert_fs`/`tempfile`); open/exec/repl-Abläufe über echte stdio-Rohre fahren; Exit-Code-Matrix je Fehlerklasse (fehlende Datei, kaputtes Argument, Read-only-Write) explizit assertieren; Timeouts so setzen, dass ein hängendes REPL den Lauf nicht blockiert.
- **Acceptance:** Mindestens open+exec+nicht-interaktives REPL über den echten Prozess getestet; jeder dokumentierte Exit-Code einmal getroffen und geprüft; Fehlerfälle präzise auf stderr; Registry-Ausnahme #2 in `docs/TESTING.md` neu bewertet (verkleinert oder begründet bestehen bleiben); canonical verification set grün inkl. Coverage-Gate.

### [ ] T5 — egui/eframe headless UI-Tests (egui_kittest, Layout-Regressionen) — 🔵
- **Location:** `crates/datacraft-ui/tests/` (neu: `ui_kittest.rs`), `crates/datacraft-ui/Cargo.toml` (dev-dep `egui_kittest`), `docs/TESTING.md` §3 Registry #1, `docs/ARCHITECTURE.md` (Schichtgrenze ui/core)
- **Problem:** Die UI-Schicht ist komplett aus der Coverage ausgeschlossen (Registry #1: „eframe startet keinen headless Event-Loop"). Damit sind alle Render-Pfade — Datasheet, SQL-Fenster, Nav-Pane, Ribbon, Statusleiste — ungetestet: Layout-Regressionen und kaputte Widget-Bindings fallen erst im manuellen Lauf auf. `egui_kittest` (offiziell im egui-Repo, AccessKit-basiert, Snapshot-Renderer) schließt genau diese Lücke.
- **Ansatz:** Harness mit `Harness::new_ui`/`new_state` gegen den echten `Core` mit MockPort aufsetzen; Widgets über AccessKit-Knoten (Label/Rolle) ansprechen statt über Pixelkoordinaten; Abläufe fahren (Tabelle in Nav-Pane wählen → Datasheet zeigt Zeilen, Query ausführen → Ergebnis-Grid, Ribbon-Aktion → Bus-Event); Snapshot-Vergleiche (`insta`-Backend von egui_kittest) für die Hauptfenster als Layout-Gate. **Versionskopplung vorab klären:** `egui_kittest` muss exakt zur gepinnten `eframe`/`egui`-Version (0.37) passen — falls für 0.37 nicht verfügbar, Item auf ⛔ setzen und Alternative (Upgrade-Kopplung an egui-Release) im Item notieren statt zu improvisieren.
- **Acceptance:** Harness startet headless ohne Fenster/GPU-Zwang und ohne echtes Event-Loop; je Hauptbereich (Datasheet, SQL/QBE-Fenster, Nav-Pane, Ribbon) mindestens ein Interaktions- plus ein Snapshot-Test; Assertions auf Domain-Zustand (`UiStateSnapshot`) statt auf Pixel; Registry-Ausnahme #1 in `docs/TESTING.md` neu bewertet (Verkleinerung dokumentieren oder Restbegründung); canonical verification set grün, Snapshot-Artefakte reproduzierbar (kein Flaky-Vergleich durch Font-/DPI-Unterschiede in CI).

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
