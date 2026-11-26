# Project tracking: DataCraft

> Source method: `docs/PLAYBOOK.md` (project-adapted)
> Progress file — updated after every step.
> Legend: ⬜ open · 🔄 in progress · ✅ done · ⛔ blocked
> Method retrospective (PLAYBOOK §8): every 5 phases — checkpointed at
> phase 98 (AGENTS §11), next at phase 103.

> **Structure (archive pattern):** This file contains the **full progress table**
> and detail blocks **only for active/open phases**. Detail blocks of completed phases
> are moved to `docs/archive/PROGRESS_ARCHIVE.md` unchanged after completion.

## Active Phases
### Phase 102 — Designer parity (roadmap §16 stage 2)

**Scope (Steps):**

- **102.1 Core: control property model with sidecar persistence and one command per edit, precise validation**
- **102.2 Core/UI: resize handles through the shared geometry rules**
- **102.3 UI/core: z-order and tab-order surface in both designers**
- **102.4 UI: preview zoom control and drag-and-drop field binding**
- **102.5 Documentation and phase gate (README designer row, COMPATIBILITY evidence, CHANGELOG)**
### Phase 103 — Migration readiness and benchmark instrument (roadmap §14/§18)

**Scope (Steps):**

- **103.1 Core: AnalyzeApp readiness report — ring coverage per object, VBA estimate, link inventory, unsupported list, effort summary**
- **103.2 UI and wire: read-only report surface with CLI/MCP parity**
- **103.3 Benchmarks: harness in tools/dao-oracle plus docs/BENCHMARKS.md (method, measurements, honest gaps)**
- **103.4 Documentation and phase gate; the phase-103 method retrospective checkpoint (PLAYBOOK §8) rides the gate**









































































































---

## Progress

| # | Step | Status |
|---|------|--------|
| 0.1 | Repository Bootstrap | ✅ |
| 1.1 | Architecture Blueprint | ✅ |
| 1.2 | Cargo Workspace & Dependencies | ✅ |
| 1.3 | CI Pipeline & Quality Gates | ✅ |
| 1.4 | Phase 1 Quality Gate Review | ✅ |
| 2.1 | StorageEngine Trait Boundary | ✅ |
| 2.2 | Jet Page Layer & File Header Decoder | ✅ |
| 2.3 | Table Definition & Data Decoder | ✅ |
| 2.4 | libmdb Test Oracle & Golden-File Tests | ✅ |
| 2.5 | Phase 2 Quality Gate Review | ✅ |
| 3.1 | Access Data Types & Field Properties | ✅ |
| 3.2 | Schema Persistence with Serde | ✅ |
| 3.3 | Phase 3 Quality Gate Review | ✅ |
| 4.1 | Command Dispatch Core | ✅ |
| 4.2 | State Query API & Test IDs | ✅ |
| 4.3 | JSON-RPC Transport Stub | ✅ |
| 4.4 | Phase 4 Quality Gate Review | ✅ |
| 5.1 | App Shell & Layout | ✅ |
| 5.2 | Ribbon Bar | ✅ |
| 5.3 | Navigation Pane | ✅ |
| 5.4 | Tabbed Document Workspace | ✅ |
| 5.5 | Phase 5 Quality Gate Review | ✅ |
| 6.1 | Shared Grid Core | ✅ |
| 6.2 | Datasheet View & Record Navigation | ✅ |
| 6.3 | Table Design View | ✅ |
| 6.4 | Phase 6 Quality Gate Review | ✅ |
| 7.1 | Relationship Mapper Pane | ✅ |
| 7.2 | QBE Grid | ✅ |
| 7.3 | Query Engine Integration | ✅ |
| 7.4 | Phase 7 Quality Gate Review | ✅ |
| 8.1 | WYSIWYG Canvas & Snap-to-Grid | ✅ |
| 8.2 | Access Control Types | ✅ |
| 8.3 | Form Runtime & Data Binding | ✅ |
| 8.4 | Phase 8 Quality Gate Review | ✅ |
| 9.1 | MCP/JSON-RPC Server Endpoint | ✅ |
| 9.2 | Headless E2E Test Harness | ✅ |
| 9.3 | Final Documentation & Release Prep | ✅ |
| 9.4 | CLI Client (Headless Transport) | ✅ |
| 10.1 | Jet4 Writer Core & Usage-Map Allocation (h9) | ✅ |
| 10.2 | Jet4 Create Tables & Write Rows (h9) | ✅ |
| 10.3 | Jet4 Writer Round-Trip Acceptance (h9) | ✅ |
| 10.4 | Report Canvas — Print Layout & Design (h31) | ✅ |
| 10.5 | Report Print Preview Runtime (h31) | ✅ |
| 10.6 | Jet 3 (Access 97) Read Support (l1) | ✅ |
| 10.7 | Jet4 Column Type 0x11 — Study & Mapping Decision (l2) | ✅ |
| 10.8 | Fixture Pin Integrity — Manifest & CI Hash Gate (t3) | ✅ |
| 10.9 | Final Documentation & Release Prep (h34) | ✅ |
| 10.10 | Phase 10 Quality Gate Review | ✅ |
| 11.1 | Port Write-Through (insert_row, Jet4Port, port_write.rs) | ✅ |
| 11.2 | Core: InsertRow-Command | ✅ |
| 11.3 | Ribbon + New-Table Flow | ✅ |
| 11.4 | Nav-Pane Delete Flows | ✅ |
| 11.5 | QBE: Run + Criteria Clear | ✅ |
| 11.6 | Designer Palettes + Source Choice | ✅ |
| 11.7 | Datasheet: New-Record Row | ✅ |
| 11.8 | Sidecar UI + Cleanup + access_gate Example | ✅ |
| 11.9 | Phase 11 Quality Gate Review | ✅ |
| 12.1 | Theme Module: Original Palette + egui Visuals | ✅ |
| 12.2 | Ribbon: Access Structure (Tab Row, Group Captions, Separators) | ✅ |
| 12.3 | Nav Pane: Access-Style Collapsing Groups | ✅ |
| 12.4 | Status Bar: Context + Record Navigation (core-side) | ✅ |
| 12.5 | Datasheet in the Icon Look (Teal Header, Row Selector) | ✅ |
| 12.6 | Phase 12 Quality Gate Review | ✅ |
| 11.10 | Jet4 Update/Delete-Writes (H40) | ✅ |
| 11.11 | Journaled Write-Profile (M2) | ✅ |
| 13.1 | SQLite Data Source & Detection (H35) | ✅ |
| 13.2 | SQLite Writes & InsertRow Command (H36) | ✅ |
| 13.3 | SQL Engine & UI Window (H37) | ✅ |
| 13.4 | Phase 13 Quality Gate Review | ✅ |
| 14.1 | Pin the ACCDB Fixtures (H38) | ✅ |
| 14.2 | ACCDB Read Path (H39) | ✅ |
| 14.3 | Phase 14 Quality Gate Review | ✅ |
| 15.1 | Expression Core: Evaluator & Functions (H41) | ✅ |
| 15.2 | Wiring: Default Values, Validation Rules, Calculated Fields (H41) | ✅ |
| 15.3 | Native Saved Queries: MSysQueries (M3) | ✅ |
| 15.4 | Phase 15 Quality Gate Review | ✅ |
| 16.1 | Complex Values/Multivalue (M4) | ✅ |
| 16.2 | Encryption/Password (M5) | ✅ |
| 16.3 | Jet3 Remainders: Jump Table & TDEF Chains (L3) | ✅ |
| 16.4 | Phase 16 Quality Gate Review | ✅ |
| 17.1 | Macro Inventory (L4) | ✅ |
| 17.2 | VBA-Extraction & Viewer (L5) | ✅ |
| 17.3 | Headless Static Analysis (L5) | ✅ |
| 17.4 | Documentation Accuracy: Compatibility Matrix (M6) | ✅ |
| 17.5 | Phase 17 Quality Gate Review | ✅ |
| 18.1 | i18n kernel in the UI crate (I1) | ✅ |
| 18.2 | All UI literals through t() (I2) | ✅ |
| 18.3 | German catalog complete (I3) | ✅ |
| 18.4 | Phase 18 Quality Gate Review | ✅ |
| 19.1 | App settings store ui.json (I4) | ✅ |
| 19.2 | Language switcher in the UI (I5) | ✅ |
| 19.3 | Phase 19 Quality Gate Review | ✅ |
| 20.1 | Theme rebase: neutral tokens, Teal accent (U1) | ✅ |
| 20.2 | Typography: Inter + type rungs (U2) | ✅ |
| 20.3 | Original vector iconography (U3) | ✅ |
| 20.4 | Dark mode: Appearance Light/Dark/System (U4) | ✅ |
| 20.5 | Phase 20 Quality Gate Review | ✅ |
| 21.1 | Contextual ribbon tabs + Views split-button (U5) | ✅ |
| 21.2 | Quick Access Toolbar (U6) | ✅ |
| 21.3 | File backstage page (U7) | ✅ |
| 21.4 | Datasheet UX: column resize, header sort, row shading (U8) | ✅ |
| 21.5 | Nav pane: category dropdown, context menu, group sort (U9) | ✅ |
| 21.6 | Tabs: middle-click close, context menu (U10) | ✅ |
| 21.7 | Window title & status-bar conventions (U11) | ✅ |
| 21.8 | Keyboard: shortcuts & ribbon key tips (U12) | ✅ |
| 21.9 | Phase 21 Quality Gate Review | ✅ |
| 22.1 | Atomic ui.json save (temp + rename) | ✅ |
| 22.2 | Window position, maximized state and off-screen rescue | ✅ |
| 22.3 | Phase 22 Quality Gate Review | ✅ |
| 23.1 | CLI binary E2E over a real subprocess (T4) | ✅ |
| 23.2 | egui headless UI tests with egui_kittest (T5) | ✅ |
| 23.3 | Native-query decode errors surface at open (M10) | ✅ |
| 23.4 | Phase 23 Quality Gate Review | ✅ |
| 24.1 | Jet AutoNumber counter maintenance on insert (M8) | ✅ |
| 24.2 | Jet table drops through the writer (M7) | ✅ |
| 24.3 | Design-time schema editing, ALTER TABLE (M9) | ✅ |
| 24.4 | Phase 24 Quality Gate Review | ✅ |
| 25.1 | ACCDB Write Contracts (H42) | ✅ |
| 25.2 | ACCDB-Port-Write-Through + Bus-E2E (H43) | ✅ |
| 25.3 | Phase 25 Quality Gate Review | ✅ |
| 26.1 | Release-Workflow: Matrix, Checksums, Notices (M13) | ✅ |
| 26.2 | Release-Smoke + Tag-Politik (M13) | ✅ |
| 26.3 | Phase 26 Quality Gate Review | ✅ |
| 27.1 | Native Query Execution (M12) | ✅ |
| 27.2 | Encrypted Save-As + Passwortwechsel (M14) | ✅ |
| 27.3 | Phase 27 Quality Gate Review | ✅ |
| 28.1 | Command-Historie: Undo/Redo (U14) | ✅ |
| 28.2 | Recent Databases im Backstage (U15) | ✅ |
| 28.3 | Phase 28 Quality Gate Review | ✅ |
| 29.1 | Create database files (Jet4 + SQLite) | ✅ |
| 29.2 | Create-from-template flow and determinism | ✅ |
| 29.3 | Sample content and generator | ✅ |
| 29.4 | Sidecar auto-load and backstage pages | ✅ |
| 29.5 | Phase 29 Quality Gate Review | ✅ |
| 30.1 | Packaging: ADR-7, backlog items M17-M19, phase plan | ✅ |
| 30.2 | Core driver-capability contracts (RED) | ✅ |
| 30.3 | Core capability plumbing and early dispatch rejection (GREEN) | ✅ |
| 30.4 | Jet4Port capability reporting + invariant contract | ✅ |
| 30.5 | SqlitePort capability reporting + invariant contract | ✅ |
| 30.6 | Driver registry in DetectingPort | ✅ |
| 30.7 | Data-driven format rendering | ✅ |
| 30.8 | UI affordances: core availability map + disabled rendering | ✅ |
| 30.9 | CSV directory driver (gated: workspace member + csv dependency) | ✅ |
| 30.10 | Documentation: README, ARCHITECTURE, ROADMAP, i18n | ✅ |
| 30.11 | Phase 30 Quality Gate Review | ✅ |
| 32.1 | Field properties persistence (H45) | ✅ |
| 32.2 | Index creation in the Jet writer (M15) | ✅ |
| 32.3 | Relationship and foreign-key creation (M16) | ✅ |
| 32.4 | Phase 32 Quality Gate Review | ✅ |
| 33.1 | Journal batches gate on the atomic_batch capability (M20) | ✅ |
| 33.2 | Phase 33 Quality Gate Review | ✅ |
| 31.1 | Retire the vestigial StorageEngine trait (M21, High gate: test suite ported) | ✅ |
| 31.2 | Crate split: datacraft-jet + datacraft-sqlite (M19) | ✅ |
| 31.3 | Standalone datacraft-mcp binary crate (M19) | ✅ |
| 31.4 | Publishing: Semver/MSRV/changelog policy, crates.io order (M19) | ✅ |
| 31.5 | Phase 31 Quality Gate Review | ✅ |
| 34.1 | Storage & drivers review: security self-review, error precision, clean-room (R1) | ✅ |
| 34.2 | Core review: dispatch/journal/capability gates, event & snapshot shapes (R1) | ✅ |
| 34.3 | UI review: paint-code discipline, i18n parity, affordance coverage (R1) | ✅ |
| 34.4 | Public API surface audit for Phase 31 (R1) | ✅ |
| 34.5 | Finding triage + fix window (R1) | ✅ |
| 34.6 | Phase 34 Quality Gate Review | ✅ |
| 35.1 | AGENTS.md + README architecture and diagrams (R2) | ✅ |
| 35.2 | ARCHITECTURE stale sections: §5 preview, §6 remaining work, §13 phase mapping (R2) | ✅ |
| 35.3 | FORMAT-JET CSV section, TESTING taxonomy, matrix re-verification (R2) | ✅ |
| 35.4 | Phase 35 Quality Gate Review | ✅ |
| 34.7 | Semantic-Conformance-Spot-Checks (T6) | ✅ |
| 36.1 | CSV behavioral edge suites (T7) | ✅ |
| 36.2 | Detecting registry edge suites + L9 (T7) | ✅ |
| 36.3 | MCP subprocess E2E + registry row (T7) — push checkpoint, green CI | ✅ |
| 36.4 | M23: registered drivers visible in the snapshot | ✅ |
| 36.5 | L7: CSV type inference | ✅ |
| 36.6 | M24: tri-state Boolean NULL decision | ✅ |
| 36.7 | Phase 36 Quality Gate Review | ✅ |
| 37.1 | Replacement fixture: object inventory and creation recipe (H50) | ✅ |
| 37.2 | Replacement fixture: create, hash-pin, port the suites, retire Northwind (H50) | ✅ |
| 37.3 | Fixture rebuild tooling: SaveAsText exports and COM builder (L21) | ✅ |
| 37.4 | Public-release readiness: licenses, fixture attribution, repo settings (H49) | ✅ |
| 37.5 | Pre-public history rewrite and verification (M34) | ✅ |
| 37.6 | Release checklist, macOS CI rerun and phase gate (L10) | ✅ |
| 38.1 | Crafting-Apps icon glyphs ported into the painter dispatcher (M40) | ✅ |
| 38.2 | Backstage restyle: full-height accent sidebar, cards, recent metadata (M41) | ✅ |
| 38.3 | Native file and message dialogs behind a UI seam (M43; rfd needs its own release) | ✅ |
| 38.4 | Window chrome polish pass: ribbon hierarchy, spacing, surfaces (M42) | ✅ |
| 38.5 | Phase 38 Quality Gate Review | ✅ |
| 39.1 | M35: named date pictures render corrupted text | ✅ |
| 39.2 | M36: SQL window dialect divergences are silent | ✅ |
| 39.3 | M37: QBE criteria silently misread unsupported Access forms | ✅ |
| 39.4 | M31: presentation cache loads swallow read errors | ✅ |
| 39.5 | Phase 39 Quality Gate Review | ✅ |
| 40.1 | Datasheet keyboard navigation: record cursor via arrow keys, Home/End, PgUp/PgDn (M38) | ✅ |
| 40.2 | Datasheet horizontal body scrolling + freeze columns (M39) | ✅ |
| 40.3 | Datasheet row deletion GUI path (M44) | ✅ |
| 40.4 | Datasheet in-place cell editing of existing rows (L23) | ✅ |
| 40.5 | Phase 40 Quality Gate Review | ✅ |
| 41.1 | Relationship mapper: create/delete affordances (M45) | ✅ |
| 41.2 | Expression window for RunExpression (M46) | ✅ |
| 41.3 | VBA static analysis viewer for AnalyzeVba (M47) | ✅ |
| 41.4 | Report designer grouping picker (M48) | ✅ |
| 41.5 | Ribbon truth pass: activate Table Design, honest Import File state (M49) | ✅ |
| 41.6 | Phase 41 Quality Gate Review | ✅ |
| 42.1 | SQLite affinity edge cases: AUTOINCREMENT scope and marker type suffixes (L13) | ✅ |
| 42.2 | Capability-gate design-time schema edits (L18) | ✅ |
| 42.3 | QBE projection resolves by name via the schema whitelist (L22) | ✅ |
| 42.4 | Phase 42 Quality Gate Review | ✅ |
| 43.1 | Coverage gap audit: contract tests for locally uncovered edge paths (T9) | ✅ |
| 43.2 | Pin cargo-llvm-cov@0.9.1 in the CI coverage job (M51, High gate: released) | ✅ |
| 43.3 | Phase 43 Quality Gate Review | ✅ |
| 44.1 | Ribbon group stretch fix + the layout/rect test seam (H52, T10) | ✅ |
| 44.2 | Status-bar panel overlap on the workspace content (H53) | ✅ |
| 44.3 | Close Database affordance on the backstage (M52) | ✅ |
| 44.4 | Contextual design view switches the open table (M53) | ✅ |
| 44.5 | Datasheet sort discoverability + first-class column filters (M54) | ✅ |
| 44.6 | Phase 44 Quality Gate Review | ✅ |
| 45.1 | Filter dialog window replaces the in-menu editor (user report: menu closes on field click) | ✅ |
| 45.2 | Record-navigation arrow order: prev left, next right (user report) | ✅ |
| 45.3 | Phase 45 Quality Gate Review (+ README datasheet row) | ✅ |
| 46.1 | Runtime paints stored control rects on the designer canvas (M57) | ✅ |
| 46.2 | Phase 46 Quality Gate Review | ✅ |
| 47.1 | Object activation surfaces the contextual ribbon tab (M58) | ✅ |
| 47.2 | Phase 47 Quality Gate Review | ✅ |
| 48.1 | Record navigation reads the cursor it writes (M60) | ✅ |
| 48.2 | Datasheet trailing elements stay above the status bar (M61) | ✅ |
| 48.3 | Filter dialog documents the criteria syntax (M62) | ✅ |
| 48.4 | Phase 48 Quality Gate Review | ✅ |
| 49.1 | Page-0 tail reverse-engineering: structure, semantics, maintenance (M63, DAO oracle) | ✅ |
| 49.2 | Writer: emit + maintain the tail and the remaining region fields (M63) | ✅ |
| 49.3 | Compatibility verification: DataCraft-created file opens in real Access (M63, user gate) | ✅ |
| 49.4 | User data recovery through the fixed writer + Phase 49 gate | ✅ |
| 50.1 | Re-admit macOS to the CI matrices (L25, High gate released 2026-11-07) and align the README matrix rows (H54) | ✅ |
| 50.2 | Phase 50 Quality Gate Review (a green CI run on main with the macOS jobs not cancelled) | ✅ |
| 51.1 | Changelog: per-phase cadence in policy and AGENTS, coarse Unreleased catch-up (L38) | ✅ |
| 51.2 | Phase 51 Quality Gate Review | ✅ |
| 52.1 | Test job: disk evidence and headroom before the build (H55, High gate released 2026-11-07) | ✅ |
| 52.2 | Phase 52 Quality Gate Review (Tests (ubuntu-latest) passes; root cause recorded) | ✅ |
| 53.1 | Jet/ACE open: a sibling .ldb forces a read-only session with the lock-file notice in the snapshot (M59) | ✅ |
| 53.2 | UI: status bar and Info backstage render the localized notice (M59) | ✅ |
| 53.3 | CLI/MCP wire surfacing + FORMAT-JET co-existence section + README matrix row (M59) | ✅ |
| 53.4 | Phase 53 Quality Gate Review | ✅ |
| 54.1 | Changelog next-version note + PUBLISHING policy wording (L39) | ✅ |
| 54.2 | Phase 54 Quality Gate Review | ✅ |
| 55.1 | Coverage-gate wording across the binding docs (M64) | ✅ |
| 55.2 | Save-as boundary + recovery vehicle in FORMAT-JET/README (M67) | ✅ |
| 55.3 | Journal batch commands documented headless-only (L26) | ✅ |
| 55.4 | Translate the German step titles in the PROGRESS permanent index (L29) | ✅ |
| 55.5 | Phase 55 Quality Gate Review | ✅ |
| 56.1 | Publish metadata: repository + readme fields on the six crates (L24, Medium gate: Cargo.toml touch — needs the human release) | ✅ |
| 56.2 | Lexer: drop the expect on the float parse path (M65) | ✅ |
| 56.3 | Phase 56 Quality Gate Review (cargo package --list clean, no warnings) | ✅ |
| 57.1 | Report designer grouping picker dispatching SetReportGrouping (M48) | ✅ |
| 57.2 | Phase 57 Quality Gate Review | ✅ |
| 58.1 | Import File flow: picker, preview, target name, write-path fill (M50) | ✅ |
| 58.2 | Capability-disabled rendering pin + source-file byte-identity (M50) | ✅ |
| 58.3 | Phase 58 Quality Gate Review | ✅ |
| 59.1 | Crate dependency-boundary guard script in CI (L32, High gate: needs the human release) | ✅ |
| 59.2 | Gate rustdoc warnings with -D warnings (L33, High gate) | ✅ |
| 59.3 | CLI smoke step on both matrix OSes (L34, High gate) | ✅ |
| 59.4 | Phase 59 Quality Gate Review | ✅ |
| 60.1 | Expression window: row-context picker dispatching RunExpression with Some(ExpressionContext) (L27) | ✅ |
| 60.2 | Oracle skip observable: #[ignore] + the CI oracle job runs --ignored (L31) | ✅ |
| 60.3 | Phase 60 Quality Gate Review | ✅ |
| 61.1 | Deterministic generator functions for canonical test databases, byte-deterministic, alongside the binary pins (L35) | ✅ |
| 61.2 | Phase 61 Quality Gate Review | ✅ |
| 62.1 | Core: stable machine-readable failure codes alongside the English text, capability rejections mapped onto the DisabledReason vocabulary (I6 step 1) | ✅ |
| 62.2 | UI: known codes render localized, unknown codes stay English (I6 step 2) | ✅ |
| 62.3 | Wire consumers switch to code assertions (command_errors, cli_contract, server_e2e, driver_caps) (I6) | ✅ |
| 62.4 | Phase 62 Quality Gate Review | ✅ |
| 63.1 | Core: SetFormControlContent/SetReportControlContent with precise validation (H56) | ✅ |
| 63.2 | UI: the selected-control content panel in both designers, kittest-pinned (H56) | ✅ |
| 63.3 | Phase 63 Quality Gate Review | ✅ |
| 64.1 | M69 decision record: Access-verified form runtime decisions in ARCHITECTURE §5 and the control matrix | ✅ |
| 64.2 | M68: bound text boxes edit through a per-record buffer (Esc field/record) with the Access ZLS rule | ✅ |
| 64.3 | M70: two-state checkbox toggle through the record edit buffer | ✅ |
| 64.4 | M70: combo row source (table/query) + display field, all rows, free entry, optional source | ✅ |
| 64.5 | M71: form-scoped WHERE filter with filtered record navigation | ✅ |
| 64.6 | M69: core ButtonAction model, control-data command and precise validation | ✅ |
| 64.7 | M69: designer action picker and runtime click dispatch (open form/report, run query, apply filter) | ✅ |
| 64.8 | Phase 64 Quality Gate Review | ✅ |
| 65.1 | Core: SourceSpec {Path, Uri} on the open/create commands — file:// resolution, precise no-server-driver boundary; carries the forced L30 command.rs split (L8) | ✅ |
| 65.2 | Core: snapshot source display; the sidecar applies to Path sources only (L8) | ✅ |
| 65.3 | UI: backstage Open accepts path or file:// URI and renders the source display, kittest pin (L8) | ✅ |
| 65.4 | Wire: URI sources flow through CLI and MCP with server_e2e pins — all four clients, file behavior byte-identical (L8) | ✅ |
| 65.5 | Phase 65 Quality Gate Review | ✅ |
| 66.1 | M72: the release draft step composes its notes from CHANGELOG's dated section (High gate released 2026-11-10) | ✅ |
| 66.2 | Phase 66 Quality Gate Review | ✅ |
| 67.1 | M72 fix 2: the publish job checks out before downloading the artifact — checkout's clean wipes dist otherwise (v0.1.1 live finding) | ✅ |
| 67.2 | Phase 67 Quality Gate Review | ✅ |
| 68.1 | Undo/Redo keyboard shortcuts: Ctrl+Z/Ctrl+Y dispatch through the bus (M73) | ✅ |
| 68.2 | Tooltip registry: spec-complete, localized tooltips for ribbon, QAT and backstage (M74) | ✅ |
| 68.3 | Notice and error history with the messages dialog (M76); carries the forced L30 app.rs/flows.rs splits | ✅ |
| 68.4 | Keyboard-shortcuts help page on F1 (M77) | ✅ |
| 68.5 | Busy overlay through the deferred-dispatch seam for blocking commands (M75); app/busy.rs split | ✅ |
| 68.6 | Window-chrome polish pass from the human's live review (M78) — blocked on the review | ⛔ |
| 68.7 | Phase 68 Quality Gate Review | ✅ |
| 69.1 | Hide MSys* system relationships from the mapper and never offer their deletion (H57) | ✅ |
| 69.2 | Ribbon toggle actions render their open state from the snapshot (M79) | ✅ |
| 69.3 | The shell follows lifecycle actions: open -> Home, create -> the new document (M80) | ✅ |
| 69.4 | A help glyph in the ribbon tab row opens the shortcuts page (M82) | ✅ |
| 69.5 | The status filter chip lists and removes individual filters (M83) | ✅ |
| 69.6 | Column headers show filter and sort state (M84) | ✅ |
| 69.7 | Tool windows open centered instead of top-left (M85) | ✅ |
| 69.8 | Phase 69 Quality Gate Review | ✅ |
| 70.1 | Wildcard patterns become an implicit Like for text filters (H58) | ✅ |
| 70.2 | Notice and error history stores messages semantically and follows a live language switch (M86) | ✅ |
| 70.3 | Tool-window contrast and the accent frame in both palettes — tokens only (M78) | ✅ |
| 70.4 | Phase 70 Quality Gate Review | ✅ |
| 71.1 | Core: ViewDef in the sidecar, snapshot entries, create/delete/set-sql commands, SELECT-only validation, ObjectKind::View opens a tab (M93) | ✅ |
| 71.2 | UI: nav-pane Views group, create/edit/delete dialog, read-only datasheet from the view SQL, kittest (M93) | ✅ |
| 71.3 | SQLite: apply the view natively (CREATE VIEW / DROP VIEW), capability-gated, with contract pins (M93) | ✅ |
| 71.4 | Phase 71 Quality Gate Review | ✅ |
| 72.1 | Core rename command, name rules and reference follow-through (M81) | ✅ |
| 72.2 | Jet/ACE table rename through the writer (M81) | ✅ |
| 72.3 | SQLite native rename and CSV boundary (M81) | ✅ |
| 72.4 | Nav-pane rename surface: context menu, F2, dialog (M81) | ✅ |
| 72.5 | Documentation and phase gate (M81) | ✅ |
| 73.1 | Native SQLite views as read-only objects: catalog, snapshot, nav-pane group, datasheet and SQL path (M90) | ✅ |
| 73.2 | Read-only index/trigger inventory per table in the snapshot and a UI surface (M90) | ✅ |
| 73.3 | File facts and a bounded quick_check at open: port channel, snapshot, Info page, precise notice (M91) | ✅ |
| 73.4 | Robust open and honesty: busy_timeout, WAL fact/notice, WITHOUT ROWID and STRICT refusals (M92) | ✅ |
| 73.5 | Documentation and phase gate (M90–M92) | ✅ |
| 74.1 | Native Access forms and reports: read-only MSysObjects inventory in snapshot and nav pane, no definition decoding (M87) | ✅ |
| 74.2 | Time-boxed feasibility verdict on decoding native form/report definitions against the committed SaveAsText exports (M89) | ✅ |
| 74.3 | Documentation and phase gate (M87, M89) | ✅ |
| 75.1 | Empty shell lands on the backstage, close returns to it (L41) | ✅ |
| 75.2 | New database: choose what, then choose where (L42) | ✅ |
| 75.3 | Blank database format choice: .mdb (Jet4) or SQLite (L43) | ✅ |
| 75.4 | Documentation and phase gate (L41–L43) | ✅ |
| 76.1 | datacraft-types: move the value vocabulary and its pure helpers below core (L36) | ✅ |
| 76.2 | datacraft-expr: extract the expression engine onto the types crate (L36) | ✅ |
| 76.3 | datacraft-query: extract the QBE model, evaluation and native execution (L37) | ✅ |
| 76.4 | sql-engine: move the shadow SQL dialect behind the query crate's feature (L37) | ✅ |
| 76.5 | Documentation and phase gate (L36, L37) | ✅ |
| 77.1 | Feasibility: jetdb against the committed fixture and the SaveAsText exports | ✅ |
| 77.2 | Route decision: own decoder with jetdb as oracle, or a jetdb-linked migration tool | ✅ |
| 77.3 | Freeze the supported control/property subset and the fixture pair | ✅ |
| 77.4 | Documentation and phase gate | ✅ |
| 78.1 | Import library: mapping, defaults, unsupported report and the contract test | ✅ |
| 78.2 | CLI and sidecar merge: arguments, atomic save, exit codes, subprocess E2E | ✅ |
| 78.3 | Six-kind and defaults fixture via the Access COM generator | ✅ |
| 78.4 | Documentation, packaging and phase gate | ✅ |
| 79.1 | Audience contract, README outline and the growth-rule amendment (PLAYBOOK §2, AGENTS §7) | ✅ |
| 79.2 | docs/FEATURES.md — move the feature inventory; README keeps a short overview | ✅ |
| 79.3 | docs/COMPATIBILITY.md — move the full evidence matrix; README keeps the dated short matrix | ✅ |
| 79.4 | User entry — current screenshot plus a release-download and install path | ✅ |
| 79.5 | CONTRIBUTING.md — quality gates and workflow; README keeps a one-line gate statement | ✅ |
| 79.6 | Retire the README roadmap table; carry M95 into the ARCHITECTURE §13 mapping | ✅ |
| 79.7 | Final pass — links, word budget, policy pointers, docs_validate and the phase gate | ✅ |
| 80.1 | Atomic write: flush, unique staging, rename retry, symlink and mode handling (H60) | ✅ |
| 80.2 | CSV import: encoding detection and delimiter sniffing through one shared core path (H59) | ✅ |
| 80.3 | Native dialogs: macOS reproduction and, if confirmed, the queued request/answer model (M97) | ⛔ |
| 81.1 | Dialog seam rework: DialogRequest/DialogAnswer queue, ScriptedDialogs through the same queue, native host on rfd's async API (M97, RED first) | ✅ |
| 81.2 | Call-site migration: parked intents for the ten sites, one-pending guard, per-frame answer poll (M97) | ✅ |
| 81.3 | Documentation and phase gate: COMPATIBILITY macOS-unverified note, kittest flows green, M97 archive | ✅ |
| 82.1 | WSL Ubuntu toolchain (rustup minimal + clippy) and scripts/unix-gate.sh with WSL-native target dir and auto-clean (M105) | ✅ |
| 82.2 | Gate run verified end-to-end, sparse VHDX, Windows-side CARGO_TARGET_DIR, documentation and phase gate (M105) | ✅ |
| 83.1 | scripts/clean-targets.ps1 with report/apply modes and the keep-only-last-build policy (M106, RED first: report run before any delete) | ✅ |
| 83.2 | Apply run against the 99 GB target, weekly scheduled task, CONTRIBUTING documentation and phase gate (M106) | ✅ |
| 84.1 | clean-targets.ps1: depth-2 cargo targets (src-tauri) plus node_modules/.venv classes with the stale rule (M107) | ✅ |
| 84.2 | Report pin of the four Tauri targets, apply run as the one-time clean, CONTRIBUTING documentation and phase gate (M107) | ✅ |
| 85.1 | Core: per-tab edit buffer vocabulary mapping datasheet updates/inserts/deletes onto the journal batch commands, RED first (H61) | ✅ |
| 85.2 | UI: pending markers, statement preview from the journal, Ctrl+S apply / Escape discard, grid lock during submission (H61) | ✅ |
| 85.3 | Capability gating without atomic_batch, read-only sources and the batch edge cases: cursor moves inside a batch, save-on-row-change decision (H61) | ✅ |
| 85.4 | Documentation and phase gate (H61) | ✅ |
| 86.1 | Core selection model: anchor rectangle, column selection, Ctrl+A, documented survival rule across scroll/sort/filter (M101) | ✅ |
| 86.2 | Clipboard TSV copy/paste with arity and size validation before any cell changes (M101) | ✅ |
| 86.3 | One-value fill as a single undo step and Delete-to-NULL with the driver NULL boundary (M101) | ✅ |
| 86.4 | Cell value viewer: read-only text/JSON/bounded hex, Shift+F2 and context menu, multiline editor through the 85 buffer (M102) | ✅ |
| 86.5 | Documentation and phase gate (M101, M102) | ✅ |
| 87.1 | FK follow on key cells: declared foreign keys, reverse keys and inferred references with composite-key pairing (M103) | ✅ |
| 87.2 | Per-tab back/forward history and capped, cancellable "referenced by" counts (M103) | ✅ |
| 87.3 | Core filter builder model: AND/OR groups with visible parentheses, ordered sort columns, criteria-string serialization (M104) | ✅ |
| 87.4 | Builder UI: draft/apply distinction, text mode as escape hatch, builder-to-criteria round-trip (M104) | ✅ |
| 87.5 | Documentation and phase gate (M103, M104) | ✅ |
| 88.1 | Ribbon overflow: group-overflow layout math in core and the overflow control with identical command dispatch (M98) | ✅ |
| 88.2 | Tab strip scroll-to-selected on activation and reachability by pointer and keyboard (L45) | ✅ |
| 88.3 | Close-to-side bus commands and the idle-tab reuse policy in the open-object path (L49) | ✅ |
| 88.4 | Glyph coverage test over both catalogs and hostile names with an explicit painted-glyph allowlist (T11) | ✅ |
| 88.5 | Documentation and phase gate (M98, L45, L49, T11) | ✅ |
| 89.1 | Tool windows as task panes — decision record in docs/ROADMAP.md with go/no-go and a scoped follow-up if yes (M100) | ✅ |
| 89.2 | Native menu bar on macOS — decision record; if yes, existing commands only and the M99 shortcut model (L46) | ✅ |
| 89.3 | Low-power/RDP rendering mode — detection decision and the mode's measurable effects (L50) | ✅ |
| 89.4 | Design pass: the human's live density/typography/spacing review with per-finding items (L47) | ✅ |
| 89.5 | Documentation and phase gate (M100, L46, L50, L47) | ✅ |
| 90.1 | Per-crate READMEs for the ten published crates with cargo package --list verification and the PUBLISHING checklist (L44, Medium gate) | ✅ |
| 90.2 | Ubuntu oracle job: bounded apt-get retry with backoff against transient DNS failures, persistent failures stay red (L40, High gate) | ✅ |
| 90.3 | Documentation and phase gate (L40, L44) | ✅ |
| 91.1 | Version display in UI, CLI and MCP (M109) | ✅ |
| 91.2 | SQL window height budget (M110) | ✅ |
| 91.3 | Import File honesty: wrong-surface rejection, single path, path display (M111, M112, L53) | ✅ |
| 91.4 | Status-line truncation contract (L52) | ✅ |
| 91.5 | Datasheet sort: total order with defined NULL placement and honest indicator (M113) | ✅ |
| 91.6 | Datasheet affordance clarity: AutoNumber hint, freeze tooltip and visible frozen state (M114, L54) | ✅ |
| 91.7 | Nav pane: sort-state honesty, whole-row click target, native-object chrome, category dropdown, New View placement (M115, L55–L58) | ✅ |
| 91.8 | Backstage: Save-page section hierarchy, Info fact-value column (M116, L59) | ✅ |
| 91.9 | Tool windows: Relationships scroll, Expression Close placement, VBA token errors, view-editor field labels, notices density, macro numbering (M117–M119, L60–L62) | ✅ |
| 91.10 | Form designer/runtime: visible selection ring, runtime combo caret, canvas scroll at the pinned minimum (M120–M122) | ✅ |
| 91.11 | Report preview: page geometry and text scale (M123, L63) | ✅ |
| 91.12 | Form record navigation: first/last/new-record (M124) | ✅ |
| 91.13 | Documentation and phase gate (M109–M124, L52–L63) | ✅ |
| 92.1 | Platform-aware shortcut matcher and labels — macOS ⌘, redo ⌘⇧Z, help/tooltip labels (M99) | ✅ |
| 92.2 | Docked task-pane host — dock right/bottom/float with the SQL window as pilot, per-surface geometry persistence (M108) | ✅ |
| 92.3 | Nav pane: search/filter across categories with a clear empty state (L48) | ✅ |
| 92.4 | Nav pane: multi-select with Shift/Ctrl, context menu acts on the selection (L48) | ✅ |
| 92.5 | Nav pane: list/grid overview, local groups and colours persisted per database in ui.json (L48) | ✅ |
| 92.6 | Low-power/RDP rendering mode — detection, override>setting>detection precedence, measurable effects, overlay (L51) | ✅ |
| 92.7 | Documentation and phase gate (M99, M108, L48, L51) | ✅ |
| 93.1 | The Relationships mapper docks through the pane host (M125, M108 follow-up) | ✅ |
| 93.2 | The Expression window docks through the pane host (M125) | ✅ |
| 93.3 | Object Details gains its Type column — the grid overview (L64, L48 follow-up) | ✅ |
| 93.4 | Documentation and phase gate (M125, L64) | ✅ |
| 94.1 | UI paint-site scan: t()-bypass guard over the UI crate, RED first (T12) | ✅ |
| 94.2 | CI hygiene job wiring for the guard (T12, High gate: needs the human release) | ✅ |
| 94.3 | Documentation and phase gate (T12) | ✅ |
| 95.1 | Core: pre-open health check — header/region/page sanity at open, read-only, health facts in the snapshot (roadmap §10 row 3) | ✅ |
| 95.2 | Core: compact — read everything, write a fresh verified Jet4/ACE file, atomic swap, the sidecar follows the path; Jet/ACE-scoped (roadmap §10 row 1) | ✅ |
| 95.3 | UI: health facts and the Compact affordance on the backstage/Info, capability-gated, localized | ✅ |
| 95.4 | Wire: compact and health through CLI and MCP with contract pins — all four clients | ✅ |
| 95.5 | Documentation and phase gate (roadmap §10 — README matrix, COMPATIBILITY evidence, CHANGELOG) | ✅ |
| 96.1 | Driver crate skeleton: datacraft-xlsx (ADR-7, core-only dependency), calamine pin in workspace deps, sniff, DatabaseFormat variant (Medium gate: new crate + dependency — needs the human release per occurrence) | ✅ |
| 96.2 | Read path: sheets as tables, typed cell values, the record's format-matrix boundaries (.xlsm values, .xls values, .xlsb values), fixtures pinned (Medium gate: fixtures) | ✅ |
| 96.3 | Composition + UI: registry registration, nav pane and datasheet through the snapshot, read-only capability honesty | ✅ |
| 96.4 | Import File: xlsx as an import source over the M50 chain (roadmap §7 downstream) | ✅ |
| 96.5 | Documentation and phase gate (README matrix, COMPATIBILITY evidence, FIXTURE-SOURCES provenance) | ✅ |
| 97.1 | Renderer decision + core feature: the pure-Rust PDF writer behind an optional core feature, the sql-engine precedent (Medium gate: dependency — needs the human release) | ✅ |
| 97.2 | Core: the export command over the paginated page model — deterministic (fixed metadata, stable order), text stays text, fonts reach the writer without depending on datacraft-ui | ✅ |
| 97.3 | UI + wire: the export affordance at the report preview, capability-gated, CLI and MCP parity | ✅ |
| 97.4 | Acceptance + documentation and phase gate: page count and breaks in a standard viewer, text extracts, byte-identical re-export, unwritable target answers precisely | ✅ |
| 98.1 | Core: read-only native SQL execution against a SQLite source — full dialect, index use, views, EXPLAIN QUERY PLAN; writes stay refused on the bus path | ✅ |
| 98.2 | UI: the SQL window names the dialect per source (SQLite native vs the shadow engine), EXPLAIN QUERY PLAN rendering | ✅ |
| 98.3 | Wire: CLI and MCP parity with contract pins; Jet/CSV sources stay on the shadow engine unchanged | ✅ |
| 98.4 | Documentation and phase gate (README SQL row, COMPATIBILITY evidence); the phase-98 method retrospective checkpoint (PLAYBOOK §8) rides the gate | ✅ |
| 99.1 | Core: ExportTable command — CSV and the JSON schema+rows document, deterministic new-file write, source untouched (roadmap §11) | ✅ |
| 99.2 | UI: export affordance at the datasheet/nav pane with format and path dialog | ✅ |
| 99.3 | Wire: CLI and MCP parity with contract pins and precise refusals | ✅ |
| 99.4 | Documentation and phase gate (README export row, COMPATIBILITY evidence, CHANGELOG) | ✅ |
| 100.1 | Jet: linked-table record decode — MSysObjects types 4/6, CONNECT/DATABASE, foreign name and flags, read-only (roadmap §9 stage 0) | ✅ |
| 100.2 | Core: link inventory in the snapshot, capability honesty — no execution, no network, never written | ✅ |
| 100.3 | UI: read-only linked-table surface with localized labels | ✅ |
| 100.4 | Wire pins, documentation and phase gate (README, COMPATIBILITY, FORMAT-JET, CHANGELOG) | ✅ |
| 101.1 | Core: filter-by-value model — bounded distinct values, deterministic order, applied through the existing criteria path | ✅ |
| 101.2 | UI: the filter-by-value dialog at the column header | ✅ |
| 101.3 | UI/core: find/replace over the visible rows, replacements staged through the edit buffer | ✅ |
| 101.4 | UI: column drag-reorder and the visual Null marker in both palettes | ✅ |
| 101.5 | Core/port: windowed row reads with bounded caches and scroll-follow — the invasive stage-1 item | ✅ |
| 101.6 | Documentation and phase gate (README datasheet row, COMPATIBILITY evidence, CHANGELOG) | ✅ |
| 102.1 | Core: control property model with sidecar persistence and one command per edit, precise validation | ⬜ |
| 102.2 | Core/UI: resize handles through the shared geometry rules | ⬜ |
| 102.3 | UI/core: z-order and tab-order surface in both designers | ⬜ |
| 102.4 | UI: preview zoom control and drag-and-drop field binding | ⬜ |
| 102.5 | Documentation and phase gate (README designer row, COMPATIBILITY evidence, CHANGELOG) | ⬜ |
| 103.1 | Core: AnalyzeApp readiness report — ring coverage per object, VBA estimate, link inventory, unsupported list, effort summary | ⬜ |
| 103.2 | UI and wire: read-only report surface with CLI/MCP parity | ⬜ |
| 103.3 | Benchmarks: harness in tools/dao-oracle plus docs/BENCHMARKS.md (method, measurements, honest gaps) | ⬜ |
| 103.4 | Documentation and phase gate; the phase-103 method retrospective checkpoint (PLAYBOOK §8) rides the gate | ⬜ |
