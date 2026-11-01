# Project tracking: DataCraft

> Source method: `docs/PLAYBOOK.md` (project-adapted)
> Progress file — updated after every step.
> Legend: ⬜ open · 🔄 in progress · ✅ done · ⛔ blocked

> **Structure (archive pattern):** This file contains the **full progress table**
> and detail blocks **only for active/open phases**. Detail blocks of completed phases
> are moved to `docs/archive/PROGRESS_ARCHIVE.md` unchanged after completion.

## Active Phases
### Phase 21 — Access UI Alignment *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **21.1 Contextual ribbon tabs + Views split-button (U5)**
- **21.2 Quick Access Toolbar (U6)**
- **21.3 File backstage page (U7)**
- **21.4 Datasheet UX: column resize, header sort, row shading (U8)**
- **21.5 Nav pane: category dropdown, context menu, group sort (U9)**
- **21.6 Tabs: middle-click close, context menu (U10)**
- **21.7 Window title & status-bar conventions (U11)**
- **21.8 Keyboard: shortcuts & ribbon key tips (U12)**
- **21.9 Phase 21 Quality Gate Review**
### Phase 23 — Hardening & Test Gaps *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **23.1 CLI binary E2E over a real subprocess (T4)**
- **23.2 egui headless UI tests with egui_kittest (T5)**
- **23.3 Native-query decode errors surface at open (M10)**
- **23.4 Phase 23 Quality Gate Review**
### Phase 24 — Jet Writer Remainder *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **24.1 Jet AutoNumber counter maintenance on insert (M8)**
- **24.2 Jet table drops through the writer (M7)**
- **24.3 Design-time schema editing, ALTER TABLE (M9)**
- **24.4 Phase 24 Quality Gate Review**


























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
| 21.1 | Contextual ribbon tabs + Views split-button (U5) | 🔄 |
| 21.2 | Quick Access Toolbar (U6) | ⬜ |
| 21.3 | File backstage page (U7) | ⬜ |
| 21.4 | Datasheet UX: column resize, header sort, row shading (U8) | ⬜ |
| 21.5 | Nav pane: category dropdown, context menu, group sort (U9) | ⬜ |
| 21.6 | Tabs: middle-click close, context menu (U10) | ⬜ |
| 21.7 | Window title & status-bar conventions (U11) | ⬜ |
| 21.8 | Keyboard: shortcuts & ribbon key tips (U12) | ⬜ |
| 21.9 | Phase 21 Quality Gate Review | ⬜ |
| 22.1 | Atomic ui.json save (temp + rename) | ✅ |
| 22.2 | Window position, maximized state and off-screen rescue | ✅ |
| 22.3 | Phase 22 Quality Gate Review | ✅ |
| 23.1 | CLI binary E2E over a real subprocess (T4) | ⬜ |
| 23.2 | egui headless UI tests with egui_kittest (T5) | ⬜ |
| 23.3 | Native-query decode errors surface at open (M10) | ⬜ |
| 23.4 | Phase 23 Quality Gate Review | ⬜ |
| 24.1 | Jet AutoNumber counter maintenance on insert (M8) | ⬜ |
| 24.2 | Jet table drops through the writer (M7) | ⬜ |
| 24.3 | Design-time schema editing, ALTER TABLE (M9) | ⬜ |
| 24.4 | Phase 24 Quality Gate Review | ⬜ |
