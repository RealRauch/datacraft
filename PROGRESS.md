# Project tracking: DataCraft

> Source method: `docs/PLAYBOOK.md` (project-adapted)
> Progress file — updated after every step.
> Legend: ⬜ open · 🔄 in progress · ✅ done · ⛔ blocked

> **Structure (archive pattern):** This file contains the **full progress table**
> and detail blocks **only for active/open phases**. Detail blocks of completed phases
> are moved to `docs/archive/PROGRESS_ARCHIVE.md` unchanged after completion.

## Active Phases
### Phase 44 — UI Layout & Affordance Audit

**Scope (Steps):**

- **44.1 Ribbon group stretch fix + the layout/rect test seam (H52, T10)**
- **44.2 Status-bar panel overlap on the workspace content (H53)**
- **44.3 Close Database affordance on the backstage (M52)**
- **44.4 Contextual design view switches the open table (M53)**
- **44.5 Datasheet sort discoverability + first-class column filters (M54)**
- **44.6 Phase 44 Quality Gate Review**
### Phase 45 — Walkthrough Fixes

**Scope (Steps):**

- **45.1 Filter dialog window replaces the in-menu editor (user report: menu closes on field click)**
- **45.2 Record-navigation arrow order: prev left, next right (user report)**
- **45.3 Phase 45 Quality Gate Review (+ README datasheet row)**
### Phase 46 — Form Runtime Layout

**Scope (Steps):**

- **46.1 Runtime paints stored control rects on the designer canvas (M57)**
- **46.2 Phase 46 Quality Gate Review**
















































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
| 11.3 | Ribbon + Neue-Tabelle-Flow | ✅ |
| 11.4 | Nav-Pane-Löschflüsse | ✅ |
| 11.5 | QBE: Run + Criteria-Clear | ✅ |
| 11.6 | Designer-Paletten + Quellen-Wahl | ✅ |
| 11.7 | Datasheet: Neue-Zeile | ✅ |
| 11.8 | Sidecar-UI + Aufräumen + access_gate-Beispiel | ✅ |
| 11.9 | Phase 11 Quality Gate Review | ✅ |
| 12.1 | Theme-Modul: ArtCraft-Palette + egui Visuals | ✅ |
| 12.2 | Ribbon: Access-Struktur (Tab-Zeile, Gruppen-Captions, Trenner) | ✅ |
| 12.3 | Nav-Pane: Collapsing-Gruppen im Access-Stil | ✅ |
| 12.4 | Status-Leiste: Kontext + Record-Navigation (core-seitig) | ✅ |
| 12.5 | Datasheet im Icon-Look (Teal-Header, Zeilenselektor) | ✅ |
| 12.6 | Phase 12 Quality Gate Review | ✅ |
| 11.10 | Jet4 Update/Delete-Writes (H40) | ✅ |
| 11.11 | Journaled Write-Profile (M2) | ✅ |
| 13.1 | SQLite-Datenquelle & Detection (H35) | ✅ |
| 13.2 | SQLite-Writes & InsertRow-Command (H36) | ✅ |
| 13.3 | SQL-Engine & UI-Fenster (H37) | ✅ |
| 13.4 | Phase 13 Quality Gate Review | ✅ |
| 14.1 | ACCDB-Fixtures pinnen (H38) | ✅ |
| 14.2 | ACCDB-Read-Pfad (H39) | ✅ |
| 14.3 | Phase 14 Quality Gate Review | ✅ |
| 15.1 | Expression-Kern: Evaluator & Funktionen (H41) | ✅ |
| 15.2 | Anschlüsse: Default Values, Validation Rules, Calculated Fields (H41) | ✅ |
| 15.3 | Native Saved Queries: MSysQueries (M3) | ✅ |
| 15.4 | Phase 15 Quality Gate Review | ✅ |
| 16.1 | Complex Values/Multivalue (M4) | ✅ |
| 16.2 | Verschlüsselung/Passwort (M5) | ✅ |
| 16.3 | Jet3-Reste: Jump-Table & TDEF-Ketten (L3) | ✅ |
| 16.4 | Phase 16 Quality Gate Review | ✅ |
| 17.1 | Makro-Inventur (L4) | ✅ |
| 17.2 | VBA-Extraction & Viewer (L5) | ✅ |
| 17.3 | Statische Analyse headless (L5) | ✅ |
| 17.4 | Doku-Genauigkeit: Kompatibilitätsmatrix (M6) | ✅ |
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
| 25.1 | ACCDB-Write-Verträge (H42) | ✅ |
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
| 44.2 | Status-bar panel overlap on the workspace content (H53) | 🔄 |
| 44.3 | Close Database affordance on the backstage (M52) | ✅ |
| 44.4 | Contextual design view switches the open table (M53) | ✅ |
| 44.5 | Datasheet sort discoverability + first-class column filters (M54) | ✅ |
| 44.6 | Phase 44 Quality Gate Review | ✅ |
| 45.1 | Filter dialog window replaces the in-menu editor (user report: menu closes on field click) | ✅ |
| 45.2 | Record-navigation arrow order: prev left, next right (user report) | 🔄 |
| 45.3 | Phase 45 Quality Gate Review (+ README datasheet row) | ✅ |
| 46.1 | Runtime paints stored control rects on the designer canvas (M57) | 🔄 |
| 46.2 | Phase 46 Quality Gate Review | ⬜ |
