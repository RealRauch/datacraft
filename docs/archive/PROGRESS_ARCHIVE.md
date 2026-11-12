# PROGRESS_ARCHIVE.md — Archive of completed phases (append-only)

> Detail blocks move here **verbatim**; the progress table stays complete
> in `PROGRESS.md` (permanent index).

---

*(empty)*

---

---

### Phase 0 — Bootstrap

**Scope (Steps):**

- **0.1 Repository Bootstrap**
**Verification:** Repository bootstrap complete: git on main, GitHub Actions quality gates (coverage >= 95%, file-size < 500 lines, cross-platform matrix), stepwell docs, README with storage decision record. (checkpoint: db6e6d5)

---

### Phase 1 — Blueprint & Skeleton

**Scope (Steps):**

- **1.1 Architecture Blueprint**
- **1.2 Cargo Workspace & Dependencies**
- **1.3 CI Pipeline & Quality Gates**
- **1.4 Phase 1 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured directly): fmt 0, clippy -D warnings 0, cargo test 5 sections ok, cargo llvm-cov --exclude datacraft-ui --fail-under-lines 95 exit 0, file-size gate OK (max file 89 lines). Items K1/K2/H1/H2/H3 archived; M1 (design-layer persistence) added as open decision. (checkpoint: 930a289)

---

### Phase 2 — Storage Engine Core (Pure Rust)

**Scope (Steps):**

- **2.1 StorageEngine Trait Boundary**
- **2.2 Jet Page Layer & File Header Decoder**
- **2.3 Table Definition & Data Decoder**
- **2.4 libmdb Test Oracle & Golden-File Tests**
- **2.5 Phase 2 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace all sections ok, cargo llvm-cov --exclude datacraft-ui --fail-under-lines 95 exit 0, file-size gate OK (max .rs file 456 lines). LESSONS self-review clean: no unsafe, no bare unwrap on parse paths (single invariant expect at catalog.rs:108 guarded by the filter above), evil-input suite present, dependency direction ui⊥storage holds, CI fully green on main incl. oracle job (run 41257090455). Backlog H4-H7 archived; open survivors: T3 (fixture pin), H9 (writer workstream), L1/L2, H8. (checkpoint: e81a0cd)

---

### Phase 3 — Data Model & Schema

**Scope (Steps):**

- **3.1 Access Data Types & Field Properties**
- **3.2 Schema Persistence with Serde**
- **3.3 Phase 3 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 13 sections ok, llvm-cov --fail-under-lines 95 exit 0 (run after step 3.2 with no code changes since). LESSONS review: error taxonomy precise (one documented-infailable expect in to_json, disclosed); no unsafe; validation boundaries table-tested; JSON round-trips proptested. Items H10/H11 archived; H12 (GUID field kind) filed as the Phase 4 prerequisite. Coverage lesson re-applied: gate re-run per step caught the 94.08% dip before commit. (checkpoint: a72c5e6)

---

### Phase 4 — Headless Core & Command API

**Scope (Steps):**

- **4.1 Command Dispatch Core**
- **4.2 State Query API & Test IDs**
- **4.3 JSON-RPC Transport Stub**
- **4.4 Phase 4 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 16 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK (max .rs file 467 lines; the ad-hoc wc check that flagged one file was counting the wc total row — CI's per-file -exec form is correct and clean), zero debt markers. LESSONS #2 seam status: command bus tested against MockPort; real-side adapter tracked as H16 (Jet4 backend implements StoragePort) before Phase 5 opens files. Work ran sequentially on main — no phase-end merge needed. H13/H14/H15 archived. (checkpoint: a0c5137)

---

### Phase 5 — UI Skeleton (egui)

**Scope (Steps):**

- **5.1 App Shell & Layout**
- **5.2 Ribbon Bar**
- **5.3 Navigation Pane**
- **5.4 Tabbed Document Workspace**
- **5.5 Phase 5 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 19 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK (no .rs file >= 500), zero debt markers. Work ran sequentially on main — no phase-end merge needed. LESSONS: ui->core layering holds (Jet4Port composition deferred to 6.2 via H21 — the mock stays the documented Phase 5 backend); egui 0.37 API notes recorded (unified Panel, show rename, default_size). H17-H20 archived; H21 filed. (checkpoint: 015459d)

---

### Phase 6 — Datasheet & Design View

**Scope (Steps):**

- **6.1 Shared Grid Core**
- **6.2 Datasheet View & Record Navigation**
- **6.3 Table Design View**
- **6.4 Phase 6 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 21 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK, zero debt markers. Work sequential on main — no merge needed. LESSONS review: row window semantics mirrored on both seam sides (mock + Jet4 adapter); full-scan reader cached per tab (windowed reads = usage-map follow-up, tracked via the datasheet note); no silent skips. H22-H24 archived, H21 resolved. (checkpoint: 06c2982)

---

### Phase 7 — QBE Query Builder

**Scope (Steps):**

- **7.1 Relationship Mapper Pane**
- **7.2 QBE Grid**
- **7.3 Query Engine Integration**
- **7.4 Phase 7 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 24 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK (the 567-line query_engine was split into query_criteria + query_engine during 7.3), zero debt markers. Work sequential on main — no merge needed. LESSONS #5: every identifier reaching the engine passes the schema whitelist (table against database, fields against source table, criteria columns re-validated in the engine). H25-H27 archived. (checkpoint: ec62b7b)

---

### Phase 8 — Form & Report Canvas

**Scope (Steps):**

- **8.1 WYSIWYG Canvas & Snap-to-Grid**
- **8.2 Access Control Types**
- **8.3 Form Runtime & Data Binding**
- **8.4 Phase 8 Quality Gate Review**
**Verification:** Gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 27 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK (core.rs/forms.rs split during 8.2), zero debt markers. Work sequential on main — no merge needed. Scope decision: the report canvas is deferred to BACKLOG H31 (canvas math, palette and sidecar persistence are shared and proven; print layout + grouped/aggregate printing wait for the expression service M1). H28-H30 archived. (checkpoint: 7a189b8)

---

### Phase 9 — MCP Endpoint & Automation

**Scope (Steps):**

- **9.1 MCP/JSON-RPC Server Endpoint**
- **9.2 Headless E2E Test Harness**
- **9.3 Final Documentation & Release Prep**
- **9.4 CLI Client (Headless Transport)**
**Verification:** Final gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 34 sections ok, llvm-cov --fail-under-lines 95 exit 0, file-size gate OK (max file 475 lines), zero debt markers. README/architecture updated for the four-client architecture; roadmap table closed. H34 archived with the step. The Phase 1-9 plan is complete. (checkpoint: efa8b71)

---

### Phase 10 — Jet4 Writer, Report Canvas & Release

**Scope (Steps):**

- **10.1 Jet4 Writer Core & Usage-Map Allocation (h9)**
- **10.2 Jet4 Create Tables & Write Rows (h9)**
- **10.3 Jet4 Writer Round-Trip Acceptance (h9)**
- **10.4 Report Canvas — Print Layout & Design (h31)**
- **10.5 Report Print Preview Runtime (h31)**
- **10.6 Jet 3 (Access 97) Read Support (l1)**
- **10.7 Jet4 Column Type 0x11 — Study & Mapping Decision (l2)**
- **10.8 Fixture Pin Integrity — Manifest & CI Hash Gate (t3)**
- **10.9 Final Documentation & Release Prep (h34)**
- **10.10 Phase 10 Quality Gate Review**
**Verification:** Final gate review passed locally (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 41 sections ok, llvm-cov --fail-under-lines 95 exit 0 (95.11% lines), file-size gate 0 violations, zero debt markers (CI word-boundary pattern), fixture manifest verified. Phase 1-10 plan is complete; the manual Access-repair check on written files remains a documented follow-up. (checkpoint: 7f9f9fc)

---

### Phase 12 — Access-Fidelity: Struktur & Icon-Thema

**Scope (Steps):**

- **12.1 Theme-Modul: ArtCraft-Palette + egui Visuals**
- **12.2 Ribbon: Access-Struktur (Tab-Zeile, Gruppen-Captions, Trenner)**
- **12.3 Nav-Pane: Collapsing-Gruppen im Access-Stil**
- **12.4 Status-Leiste: Kontext + Record-Navigation (core-seitig)**
- **12.5 Datasheet im Icon-Look (Teal-Header, Zeilenselektor)**
- **12.6 Phase 12 Quality Gate Review**
**Verification:** Final gate review passed (exit codes measured): fmt 0, clippy -D warnings 0, cargo test --workspace 41 sections ok, llvm-cov >=95 lines exit 0, file-size clean, fixture manifest verified. App wears the icon palette (teal tile field as theme colour, paper chrome, ink text); ribbon/nav/status/datasheet follow Access structure. Visual acceptance (optically appealing) is the user's look at the running app. (checkpoint: 2400d58)

---

### Phase 13 — SQL- & SQLite-Integration

**Scope (Steps):**

- **13.1 SQLite-Datenquelle & Detection (H35)**
- **13.2 SQLite-Writes & InsertRow-Command (H36)**
- **13.3 SQL-Engine & UI-Fenster (H37)**
- **13.4 Phase 13 Quality Gate Review**
**Verification:** Quality gate review passed: fmt, clippy -D warnings, cargo test --workspace, coverage 95.05% (ui excluded), file-size gate (<500 lines), docs_validate clean, no TODO/FIXME/HACK/XXX. Review findings fixed in c9cdf67 (read-only SQLite files read and reject writes precisely; mapping tests split). Phase commits: ca61638/5a04e94 (13.1), 99ddb0b/390c79c (13.2), 1a58292/0400f63 (13.3), c9cdf67 (gate).

---

### Phase 14 — ACCDB/ACE-Read

**Scope (Steps):**

- **14.1 ACCDB-Fixtures pinnen (H38)**
- **14.2 ACCDB-Read-Pfad (H39)**
- **14.3 Phase 14 Quality Gate Review**
**Verification:** Quality gate passed: fmt, clippy -D warnings, 49 suites green, coverage 95.13%, file-size gate, fixture manifest 5/5 OK, docs_validate clean. Phase commits: aef9b8f (14.1), d6a3a66 (14.2), 976c062 (gate); ROADMAP M0 updated; complex f_ subtable interim documented on M4.

---

### Phase 11 — UI ausbauen: Write-Pfad & Objektflüsse

**Scope (Steps):**

- **11.1 Port Write-Through (insert_row, Jet4Port, port_write.rs)**
- **11.2 Core: InsertRow-Command**
- **11.3 Ribbon + Neue-Tabelle-Flow**
- **11.4 Nav-Pane-Löschflüsse**
- **11.5 QBE: Run + Criteria-Clear**
- **11.6 Designer-Paletten + Quellen-Wahl**
- **11.7 Datasheet: Neue-Zeile**
- **11.8 Sidecar-UI + Aufräumen + access_gate-Beispiel**
- **11.9 Phase 11 Quality Gate Review**
- **11.10 Jet4 Update/Delete-Writes (H40)**
- **11.11 Journaled Write-Profile (M2)**
**Verification:** Journal in core: PreviewWrite diff + token, ApplyWrite with stale-preview re-validation and atomic StoragePort::write_row_ops (Jet: one writer session), RollbackWrite (inverse ops, deletes re-appended), DiscardWrite; write profile refuses attachment/GUID-AutoNumber precisely (M4/M8). 14 contract tests incl. JSON-RPC round trip. Verification: fmt, clippy -D warnings, cargo test --workspace (55 suites), llvm-cov 95.20% lines, file-size clean. (checkpoint dcb6852)

---

### Phase 15 — Expression-Service & Saved Queries

**Scope (Steps):**

- **15.1 Expression-Kern: Evaluator & Funktionen (H41)**
- **15.2 Anschlüsse: Default Values, Validation Rules, Calculated Fields (H41)**
- **15.3 Native Saved Queries: MSysQueries (M3)**
- **15.4 Phase 15 Quality Gate Review**
**Verification:** Quality gate passed: fmt, clippy -D warnings, 60 suites green, coverage ≥95%, file-size gate, fixture manifest + docs_validate clean. Commits: 15.1 b6c7c73/a657762, 15.2 7eb3bbc/6cb634d, 15.3 78203c3/d914484, fixture 51bd3c5; H41 + M3 archived; ROADMAP M1 closed, M4 bands note updated.

---

### Phase 16 — Complex Values, Verschlüsselung & Jet3-Reste

**Scope (Steps):**

- **16.1 Complex Values/Multivalue (M4)**
- **16.2 Verschlüsselung/Passwort (M5)**
- **16.3 Jet3-Reste: Jump-Table & TDEF-Ketten (L3)**
- **16.4 Phase 16 Quality Gate Review**
**Verification:** Quality gate passed: fmt 0, clippy -D warnings 0, workspace tests green, file-size 0 violations, fixture manifest 13/13 OK, docs_validate clean, no debt markers. Coverage 92.81% local (baseline 4425bc2 measured 92.81% on this machine — the 95% gate is CI-enforced; no regression). M4/M5/L3 archived with their commits; CI oracle auto-includes complexDataV2007/testV1997/test2V1997.

---

### Phase 17 — Makros, VBA & Doku-Genauigkeit

**Scope (Steps):**

- **17.1 Makro-Inventur (L4)**
- **17.2 VBA-Extraction & Viewer (L5)**
- **17.3 Statische Analyse headless (L5)**
- **17.4 Doku-Genauigkeit: Kompatibilitätsmatrix (M6)**
- **17.5 Phase 17 Quality Gate Review**
**Verification:** Gate review passed: fmt, clippy -D warnings, cargo test --workspace, file-size <500, fixture manifest hashes, docs_validate (4 accepted PLAN_WITHOUT_WIP for planned phases), no debt markers. Coverage: Windows line coverage 93.45% vs 92.81% Phase-16 baseline on the same host; new modules 95-100%; Linux CI remains the binding >=95% gate (TESTING §2 platform note). Archive debt-scan exclusion fixed in 581b28d. Phase commits: cf63492/0bc2176/8da3acb (17.1), d155410/ccf2107 (17.2), f56ca82/63ec95c (17.3), 19854e6/d532c45 (17.4), 5e1bd88/581b28d (gate). (checkpoint: 581b28d)

---

### Phase 18 — UI Localization (i18n) *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **18.1 i18n kernel in the UI crate (I1)**
- **18.2 All UI literals through t() (I2)**
- **18.3 German catalog complete (I3)**
- **18.4 Phase 18 Quality Gate Review**
**Verification:** Gate passed: fmt 0, clippy -D warnings 0, workspace tests green, file-size gate 0, fixture manifest 15/15 OK, no debt markers, docs_validate clean. Coverage 93.46% local vs Phase-17 93.45% on the same host; Linux CI remains the binding >=95% gate (TESTING §2). I2/I3 verified: no bare user-facing literals left (grep allow-list: glyphs/numbers only), catalog parity both directions, German layout 58.2/112 px and 164.9/640 px, locale mapping + English fallback unit-tested. sys-locale content gate approved by the user before landing (crate already transitive). Commits: 1780c1e/dbc513a (18.1), 57af757/b616e9c (18.2), 44e2251/6486786/5c9d69e/1926448 (18.3). (checkpoint: 1926448)

---

### Phase 19 — App Settings & Language Switcher *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **19.1 App settings store ui.json (I4)**
- **19.2 Language switcher in the UI (I5)**
- **19.3 Phase 19 Quality Gate Review**
**Verification:** Gate passed: fmt 0, clippy -D warnings 0, workspace tests green (UI 39 unit + 10 e2e), file-size gate 0 after the app.rs split (195df3c), fixture manifest 14/14 OK (the Phase-18 note said 15/15 — the manifest has 14 entries, corrected here), CI debt scan clean, docs_validate clean. Coverage 93.46% local, unchanged from Phase 18 (UI crate excluded); Linux CI remains the binding >=95% gate (TESTING §2). I5 verified: live switch, immediate persistence, Auto→detection and the full precedence chain headless-tested; settings live only in ui.json and the sidecar is untouched. Commits: 6af3622/d898120 (19.1), d765cec/745f84f (19.2), 195df3c (gate split). (checkpoint: 195df3c)

---

### Phase 20 — Look & Typography (WordCraft neutrals, Teal accent) *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **20.1 Theme rebase: neutral tokens, Teal accent (U1)**
- **20.2 Typography: Inter + type rungs (U2)**
- **20.3 Original vector iconography (U3)**
- **20.4 Dark mode: Appearance Light/Dark/System (U4)**
- **20.5 Phase 20 Quality Gate Review**
**Verification:** fmt --check and clippy -D warnings green; cargo test --workspace green; file-size gate green; local Windows coverage 91.98 % lines unchanged (datacraft-ui excluded by registry #1, platform gap documented in docs/TESTING.md §3); before/after light and dark screenshots under docs/phase-20 reviewed at the gate; the empty-workspace canvas defect found by the screenshots was fixed inline (F3). (checkpoint: 82e456a)

---

### Phase 22 — Settings hardening (U13)

**Scope (Steps):**

- **22.1 Atomic ui.json save (temp + rename)**
- **22.2 Window position, maximized state and off-screen rescue**
- **22.3 Phase 22 Quality Gate Review**
**Verification:** fmt --check and clippy -D warnings green; cargo test --workspace green (69 UI tests incl. atomic-save and geometry suites); file-size gate green (settings split into `settings/tests.rs`); local Windows coverage 91.98 % lines unchanged (datacraft-ui excluded by registry #1, platform gap documented in docs/TESTING.md §3); runtime smoke test confirmed position round-trip and off-screen rescue (9000,9000 → 820,380). (checkpoint: 21044b2)

---

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
**Verification:** Phase 21 gate: fmt --check, clippy -D warnings, workspace tests and the file-size gate are green; no banned markers; local Windows llvm-cov reports 93.50% (documented platform note — CI ubuntu is the binding gate); README phase table and intro updated; all U5–U12 items archived, follow-ups U14/U15 recorded. (checkpoint: dd8fe9e)

---

### Phase 23 — Hardening & Test Gaps *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **23.1 CLI binary E2E over a real subprocess (T4)**
- **23.2 egui headless UI tests with egui_kittest (T5)**
- **23.3 Native-query decode errors surface at open (M10)**
- **23.4 Phase 23 Quality Gate Review**
**Verification:** Phase 23 gate: fmt --check, clippy -D warnings, workspace tests (incl. CLI process E2E and kittest) and the file-size gate are green; no banned markers; local Windows llvm-cov 93.67% under the documented platform note (CI ubuntu is binding); README phase table extended; T4/T5/M10 archived, F4 recorded and archived. (checkpoint: c1b28b2)

---

### Phase 24 — Jet Writer Remainder *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **24.1 Jet AutoNumber counter maintenance on insert (M8)**
- **24.2 Jet table drops through the writer (M7)**
- **24.3 Design-time schema editing, ALTER TABLE (M9)**
- **24.4 Phase 24 Quality Gate Review**
**Verification:** Phase 24 gate: fmt --check, clippy -D warnings, workspace tests (incl. oracle-gated writer tests), file-size gate and marker scan are green; local Windows llvm-cov 93.32% under the documented platform note (CI ubuntu is binding); README phase table extended; M7/M8/M9 archived. (checkpoint: 4fecca9)

---

### Phase 25 — ACCDB-Write

**Scope (Steps):**

- **25.1 ACCDB-Write-Verträge (H42)**
- **25.2 ACCDB-Port-Write-Through + Bus-E2E (H43)**
- **25.3 Phase 25 Quality Gate Review**
**Verification:** canonical verification set green (fmt, clippy -D warnings, workspace tests, llvm-cov >= 95% lines, file-size gate); README matrix + ROADMAP + FORMAT-JET now carry the verified ACE write claims (checkpoint: b7afaef)

---

### Phase 29 — Sample Databases & Templates

**Scope (Steps):**

- **29.1 Create database files (Jet4 + SQLite)**
- **29.2 Create-from-template flow and determinism**
- **29.3 Sample content and generator**
- **29.4 Sidecar auto-load and backstage pages**
- **29.5 Phase 29 Quality Gate Review**
**Verification:** canonical verification set green (fmt, clippy -D warnings, workspace tests, llvm-cov >= 95% lines, file-size gate, make_samples --check, sha256sum -c samples/manifest); README matrix + ROADMAP carry the verified sample/template claims; manual Access-repair gate documented as follow-up (checkpoint: 7d38ac6)

---

### Phase 30 — Driver Model: Capabilities, Registry, CSV & UI Affordances

**Scope (Steps):**

- **30.1 Packaging: ADR-7, backlog items M17-M19, phase plan**
- **30.2 Core driver-capability contracts (RED)**
- **30.3 Core capability plumbing and early dispatch rejection (GREEN)**
- **30.4 Jet4Port capability reporting + invariant contract**
- **30.5 SqlitePort capability reporting + invariant contract**
- **30.6 Driver registry in DetectingPort**
- **30.7 Data-driven format rendering**
- **30.8 UI affordances: core availability map + disabled rendering**
- **30.9 CSV directory driver (gated: workspace member + csv dependency)**
- **30.10 Documentation: README, ARCHITECTURE, ROADMAP, i18n**
- **30.11 Phase 30 Quality Gate Review**
**Verification:** Canonical verification set green at the gate: cargo fmt --check ok, clippy -D warnings exit 0, cargo test --workspace exit 0 (112 suites ok), file-size gate clean (no .rs >= 500 lines), no new technical-debt markers, no new coverage exclusions (TESTING.md registry unchanged). CLI smoke over the real binary: a CSV directory opens with driver "csv" + full capabilities + availability map in the snapshot, InsertRow persists to disk. CI (ubuntu) remains the binding coverage gate on push. Review against LESSONS: seam contracts tested from both sides (core driver_caps/jet/sqlite/registry/csv + wire server_e2e_csv), no silent error swallowing (precise Unsupported/OpenFailed everywhere), real read/write paths exercised against real files, no mass replacements. Two size-gate slips were fixed forward (19f23f8, server_e2e_csv split); one fmt slip fixed forward (abb5869). (checkpoint: 403e273)

---

### Phase 27 — Native Queries & Encrypted Save-As

**Scope (Steps):**

- **27.1 Native Query Execution (M12)**
- **27.2 Encrypted Save-As + Passwortwechsel (M14)**
- **27.3 Phase 27 Quality Gate Review**
**Verification:** Phase 27 gate passed: fmt, clippy -D warnings, cargo test --workspace green; file-size gate clean; local Windows coverage 94.22% lines vs the 94.00% pre-phase baseline (new code ~98%; CI ubuntu remains the binding gate); docs_validate has no Phase-27 finding (pre-existing PLAN_WITHOUT_WIP for planned phases 26/28/31-35); no debt markers.

---

### Phase 28 — UI-Reste (Undo & Backstage)

**Scope (Steps):**

- **28.1 Command-Historie: Undo/Redo (U14)**
- **28.2 Recent Databases im Backstage (U15)**
- **28.3 Phase 28 Quality Gate Review**
**Verification:** Phase 28 gate passed: fmt, clippy -D warnings, cargo test --workspace green; file-size gate clean; local Windows coverage 94.05% lines vs the 94.00% pre-phase baseline (CI ubuntu remains binding); docs_validate has no Phase-28 finding; no debt markers.

---

### Phase 33 — Journal Batch Integrity

**Scope (Steps):**

- **33.1 Journal batches gate on the atomic_batch capability (M20)**
- **33.2 Phase 33 Quality Gate Review**
**Verification:** Phase 33 gate passed: fmt, clippy -D warnings, cargo test --workspace green; file-size gate clean; local Windows coverage 94.04% lines (above the 94.00% baseline; CI ubuntu binding); docs_validate has no Phase-33 finding; no debt markers.

---

### Phase 32 — Writer Catalog Fidelity

**Scope (Steps):**

- **32.1 Field properties persistence (H45)**
- **32.2 Index creation in the Jet writer (M15)**
- **32.3 Relationship and foreign-key creation (M16)**
- **32.4 Phase 32 Quality Gate Review**
**Verification:** Phase 32 gate passed: fmt, clippy -D warnings, cargo test --workspace green; file-size gate clean; local Windows coverage 94.05% lines (above the 94.00% pre-phase baseline; CI ubuntu binding); docs_validate has no Phase-32 finding; no debt markers.

---

### Phase 26 — Distribution & Release *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **26.1 Release-Workflow: Matrix, Checksums, Notices (M13)**
- **26.2 Release-Smoke + Tag-Politik (M13)**
- **26.3 Phase 26 Quality Gate Review**
**Verification:** canonical gates green (fmt/clippy/tests 130 suites ok, local coverage >= 95, file-size, hygiene tokens, fixture pins, no binaries in repo); release.yml YAML-validated; packaged smoke green in three runs (jet fixture, samples demo, csv directory); M13 acceptance verified and archived; security self-review: least-privilege token scopes, draft-only publication, no third-party actions; the first real release run stays pending the first v* tag push (checkpoint: d3a008c)

---

### Phase 31 — Engine Publishing *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **31.1 Retire the vestigial StorageEngine trait (M21, High gate: test suite ported)**
- **31.2 Crate split: datacraft-jet + datacraft-sqlite (M19)**
- **31.3 Standalone datacraft-mcp binary crate (M19)**
- **31.4 Publishing: Semver/MSRV/changelog policy, crates.io order (M19)**
- **31.5 Phase 31 Quality Gate Review**
**Verification:** canonical gates green (fmt/clippy/tests 134 suites, local coverage >= 95, file-size, hygiene, fixture pins 14/14); acceptance walked: one seam (M21), C-free datacraft-jet via the core sql-engine gate + standalone datacraft-mcp crate (M19), publishing policy in docs/PUBLISHING.md with package dry-run evidence (31.4); M19/M21/M22 archived; pending honestly: first CI run against the seven-crate layout, first v* release and first crates.io publish all wait on the next push/tag (checkpoint: 10689e6)

---

### Phase 36 — Coverage repair & backlog clearing

**Scope (Steps):**

- **36.1 CSV behavioral edge suites (T7)**
- **36.2 Detecting registry edge suites + L9 (T7)**
- **36.3 MCP subprocess E2E + registry row (T7) — push checkpoint, green CI**
- **36.4 M23: registered drivers visible in the snapshot**
- **36.5 L7: CSV type inference**
- **36.6 M24: tri-state Boolean NULL decision**
- **36.7 Phase 36 Quality Gate Review**
**Verification:** canonical gates green locally (fmt/clippy/tests 138 suites/coverage >= 95/file-size after the port+csv root splits/hygiene/fixtures 14); CI green on ubuntu tests+oracle, windows tests, coverage, clippy, fmt, hygiene, fixtures; the macOS jobs were cancelled by runner infrastructure in five consecutive attempts (queue cancellation at 15m01s, no logs — the same suite ran green on macOS that morning); tracked as BACKLOG L10 for the rerun, gate config unchanged; phase acceptance: every open item archived (T7, L9, M23, L7, M24), the coverage gate green on CI, the backlog's unassigned items cleared (checkpoint: 4ca02ac)

---

### Phase 34 — Cross-Cutting Code Review *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **34.1 Storage & drivers review: security self-review, error precision, clean-room (R1)**
- **34.2 Core review: dispatch/journal/capability gates, event & snapshot shapes (R1)**
- **34.3 UI review: paint-code discipline, i18n parity, affordance coverage (R1)**
- **34.4 Public API surface audit for Phase 31 (R1)**
- **34.5 Finding triage + fix window (R1)**
- **34.6 Phase 34 Quality Gate Review**
- **34.7 Semantic-Conformance-Spot-Checks (T6)**
**Verification:** canonical gates green locally: fmt 0, clippy -D warnings 0, 143 test suites 0, coverage 95.03% lines (datacraft-ui excluded), file-size gate clean after the row/database/app splits, docs_validate clean, RUSTDOCFLAGS="-D warnings" cargo doc 0. Review: 34 findings filed across storage/core/UI/API; all three HIGH (H46 crafted-structure panics, H47 non-atomic persistence, H48 stale journals) fixed in-phase with RED/GREEN commits, plus M25/M26/M27/M28/M29/M32/M33, L11-L20 and F5 — 19 items archived with hashes, the rest packaged (M31, M35-M37, L13, L18, L22, I6, L8, L10; H49/H50 and M34 come from the parallel release-prep work). LESSONS checklist walked per area in the phase review notes; T6 spot checks ran 7 clusters against expression/QBE/SQL: NULL semantics conformant, Format named pictures broken (M35), SQL and QBE dialect divergences fenced as M36/M37, QBE projection contract as L22. Correction: the L17 and M29 archive notes carry typo'd RED hashes — the actual commits are c96d117 and ff678c7.

---

### Phase 37 — Release Provenance & Licensing *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **37.1 Replacement fixture: object inventory and creation recipe (H50)**
- **37.2 Replacement fixture: create, hash-pin, port the suites, retire Northwind (H50)**
- **37.3 Fixture rebuild tooling: SaveAsText exports and COM builder (L21)**
- **37.4 Public-release readiness: licenses, fixture attribution, repo settings (H49)**
- **37.5 Pre-public history rewrite and verification (M34)**
- **37.6 Release checklist, macOS CI rerun and phase gate (L10)**

**Goal:** Make the repository publishable: no unlicensed fixture, tracked
license texts, rewritten private history, green CI. Replace the unlicensed
Northwind fixture with a self-created, hash-pinned one, record the fixture
attribution, and rewrite the private history before the visibility switch.
Risk gates apply per step: H50 is Medium (new fixture) plus High (evidence
change), H49 (license files) and M34 (history rewrite) are High — each needs
an explicit human release; 37.5 runs only after 37.2.

**Acceptance:** No committed fixture traces to an unlicensed source; license
texts, attribution and history are release-clean. `LICENSE-MIT`,
`LICENSE-APACHE` and `NOTICE` are tracked and referenced by Cargo/README;
every fixture has an attribution row; `git log --all --name-only` lists
neither the Northwind fixture nor `.zcode` plans; the rewritten `main` passes
the canonical set and the fixture hash gate; CI including macOS is green
before the tag.

---

### Phase 38 — UI restyle

**Scope (Steps):**

- **38.1 Crafting-Apps icon glyphs ported into the painter dispatcher (M40)**
- **38.2 Backstage restyle: full-height accent sidebar, cards, recent metadata (M41)**
- **38.3 Native file and message dialogs behind a UI seam (M43; rfd needs its own release)**
- **38.4 Window chrome polish pass: ribbon hierarchy, spacing, surfaces (M42)**
- **38.5 Phase 38 Quality Gate Review**
**Verification:** Canonical set on the final tree: fmt green, clippy -D warnings green, 143 workspace test suites green, file-size gate green, docs_validate clean. llvm-cov exits 1 locally at 94.82% lines — the documented Windows skew (runner-conditional skips landed in phase 37; all phase-38 code sits in the coverage-excluded UI crate and moves the number not at all); CI ubuntu-latest is the binding gate per AGENTS §4 and runs on the next push. README/NOTICES updated per step (icon attribution, backstage grammar, dialog seam).

---

### Phase 39 — Release correctness bugs

**Scope (Steps):**

- **39.1 M35: named date pictures render corrupted text**
- **39.2 M36: SQL window dialect divergences are silent**
- **39.3 M37: QBE criteria silently misread unsupported Access forms**
- **39.4 M31: presentation cache loads swallow read errors**
- **39.5 Phase 39 Quality Gate Review**
**Verification:** Canonical set on the final tree: fmt green, clippy -D warnings green, 144 workspace test suites green, file-size gate green, docs_validate clean. llvm-cov exits 1 locally at the documented 94.82% Windows baseline (all phase-39 code sits in the covered core crate and the number did not move from the phase-37 baseline; runner-conditional skips skew the local run) — CI ubuntu is the binding gate per AGENTS §4. Commit hygiene note: a4b3fc9 captured only a formatting fix; 33a82b8 landed the M31 implementation files as an immediate fix-forward (no amend).

---

### Phase 40 — Datasheet Interaction

**Scope (Steps):**

- **40.1 Datasheet keyboard navigation: record cursor via arrow keys, Home/End, PgUp/PgDn (M38)**
- **40.2 Datasheet horizontal body scrolling + freeze columns (M39)**
- **40.3 Datasheet row deletion GUI path (M44)**
- **40.4 Datasheet in-place cell editing of existing rows (L23)**
- **40.5 Phase 40 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 148 workspace test suites, file-size gate, no hygiene markers. Local Windows llvm-cov is 94.88% (informational only; CI ubuntu-latest is the binding gate per AGENTS §4) — the sole uncovered core line is the pre-existing column_span_x out-of-range fallback (grid.rs:113), all step 40.1-40.4 code is fully covered. Phase deliverables: M38 keyboard record navigation with scroll-follow core math (46369e8), M39 freeze columns + horizontal body scrolling (8f3d433), M44 confirmed DeleteRow via context menu and Del key (88df660), L23 in-place cell editor committing exactly one UpdateRow (8e28ef9).

---

### Phase 43 — Coverage Parity

**Scope (Steps):**

- **43.1 Coverage gap audit: contract tests for locally uncovered edge paths (T9)**
- **43.2 Pin cargo-llvm-cov@0.9.1 in the CI coverage job (M51, High gate: released)**
- **43.3 Phase 43 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 148 workspace test suites, file-size gate, hygiene clean, and the local Windows coverage gate (95.0032%, MSVC attribution) now passes with --fail-under-lines 95. Deliverables: T9 edge-path pins across expr/undo/port/qbe/present/nav/schema/alter/csv with TESTING.md exception #4 documenting the defensive/unreachable arms and the MSVC-vs-linux attribution residue (b83ffe1); M51 pinned cargo-llvm-cov@0.9.1 in the CI coverage job with the deliberate-bump policy (1a769e2, High gate released in-session).

---

### Phase 41 — GUI Parity: Surface the Headless Commands *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **41.1 Relationship mapper: create/delete affordances (M45)**
- **41.2 Expression window for RunExpression (M46)**
- **41.3 VBA static analysis viewer for AnalyzeVba (M47)**
- **41.4 Report designer grouping picker (M48)**
- **41.5 Ribbon truth pass: activate Table Design, honest Import File state (M49)**
- **41.6 Phase 41 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 153 workspace suites (1296 tests, 0 failures), file-size gate, hygiene clean (CI word-boundary scan), coverage gate green (llvm-cov --fail-under-lines 95 exit 0). Deliverables: M45 mapper create form + per-row delete with relationships.create/delete affordance ids and the CSV wire pin (2aaa13a/f331afe); M46 Expression window over RunExpression, row-context follow-up filed as L27 (3e1750c/0fdb340); M47 analysis pane rendering all five VbaAnalysis sections (af03bed/8c6c689); M48 grouping picker in both directions plus the F10 design-view palette fix (681e0f9/76ab83c); M49 Table Design active and Import File honest Planned (63fe820/78b19d2). Deviations documented: M49's command_for wording refined to the shared flows seam (the same input-bound exclusion the seam grants the Table action); M47's kittest pins fixture-shaped MockPort content because datacraft-ui cannot decode the accdb fixture (dependency direction). Process note: a popup-bisect scratch suite slipped into 76ab83c and was removed forward in cf84051.

---

### Phase 42 — Core Correctness *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **42.1 SQLite affinity edge cases: AUTOINCREMENT scope and marker type suffixes (L13)**
- **42.2 Capability-gate design-time schema edits (L18)**
- **42.3 QBE projection resolves by name via the schema whitelist (L22)**
- **42.4 Phase 42 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 154 workspace suites (1310 tests, 0 failures), file-size gate, hygiene clean (no banned tokens in phase-42 files), coverage gate green (llvm-cov --fail-under-lines 95 exit 0), docs_validate ok. Deliverables: L13 token-scoped AUTOINCREMENT detection plus suffix-tolerant marker names via ddl::pk_is_autoincrement/marker_base, FORMAT-JET updated (e900ea6/4d5ebba); L18 AlterTables/CreateDatabase availability vocabulary gating the design-view edit block and the backstage Create Database button with localized reasons, CSV wire pin honestly extended (890bf07/ea7b277); L22 by-name QBE projection through the source whitelist with DuplicateField/UnknownField errors and the contract documented on QueryDef/QbeField (8b6ac9b/69e0797). One unrelated flake observed during 42.3 filed as L28 (stale scratch files collide with recycled pids). Mechanical normalization inside the L13 RED commit: the corrupt-file test's raw NUL byte became the identical \\x00 escape. (checkpoint: 69e0797)

---

### Phase 35 — Documentation Sweep *(accepted: PLAN_WITHOUT_WIP)*

**Scope (Steps):**

- **35.1 AGENTS.md + README architecture and diagrams (R2)**
- **35.2 ARCHITECTURE stale sections: §5 preview, §6 remaining work, §13 phase mapping (R2)**
- **35.3 FORMAT-JET CSV section, TESTING taxonomy, matrix re-verification (R2)**
- **35.4 Phase 35 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 154 workspace suites (1310 tests, 0 failures), file-size gate, hygiene clean (no banned tokens in code files), coverage gate green (llvm-cov --fail-under-lines 95 exit 0), docs_validate ok. Deliverables: 35.1 AGENTS.md/README crate graphs, roadmap 26-43, CI-matrix and read/write claims match the manifests and workflows (8b8c686); 35.2 ARCHITECTURE command surface points at command.rs, crate tree/edges corrected (cli listed, ui/cli carry storage+csv), format milestones through Phase 32, honest remaining-work list, full 1-43 phase mapping (8af10c0); 35.3 FORMAT-JET CSV driver section, TESTING taxonomy repaired with the live suite families, compatibility matrix re-verified and stamped 2026-11-05 (ab86d4d); ROADMAP milestone rows (M12/M3) and ordering notes updated with the executed order through Phase 43; R2 archived. Acceptance evidence: the fresh-contributor walkthrough (read AGENTS/README -> build -> open a CSV directory) runs green over the real CLI binary. (checkpoint: ab86d4d)

---

### Phase 46 — Form Runtime Layout

**Scope (Steps):**

- **46.1 Runtime paints stored control rects on the designer canvas (M57)**
- **46.2 Phase 46 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 158 workspace suites (0 failures), file-size gate, hygiene clean, coverage gate green (llvm-cov --fail-under-lines 95 exit 0), docs_validate ok. Deliverable: 46.1 the form runtime renders the designed layout — the shared 640x400 designer canvas, controls at their stored rects and sizes (c163a49, M57 archived); the new ui_kittest_form suite pins runtime-rect == stored-rect through the layout_probe seam. README form row unchanged (the compatibility matrix's design-layer row already claims sidecar forms; the runtime now honours it). (checkpoint: c163a49)

---

### Phase 47 — Walkthrough Fixes II

**Scope (Steps):**

- **47.1 Object activation surfaces the contextual ribbon tab (M58)**
- **47.2 Phase 47 Quality Gate Review**
**Verification:** Canonical verification set green on the final state: fmt, clippy -D warnings, 158 workspace suites (0 failures), file-size gate, hygiene clean, coverage gate green (llvm-cov --fail-under-lines 95 exit 0), docs_validate ok. Deliverable: 47.1 object activation surfaces the contextual ribbon tab (0154080, M58 archived); README/ARCHITECTURE carry the phase-47 rows. (checkpoint: 0154080)

---

### Phase 44 — UI Layout & Affordance Audit

**Scope (Steps):**

- **44.1 Ribbon group stretch fix + the layout/rect test seam (H52, T10)**
- **44.2 Status-bar panel overlap on the workspace content (H53)**
- **44.3 Close Database affordance on the backstage (M52)**
- **44.4 Contextual design view switches the open table (M53)**
- **44.5 Datasheet sort discoverability + first-class column filters (M54)**
- **44.6 Phase 44 Quality Gate Review**

---

### Phase 45 — Walkthrough Fixes

**Scope (Steps):**

- **45.1 Filter dialog window replaces the in-menu editor (user report: menu closes on field click)**
- **45.2 Record-navigation arrow order: prev left, next right (user report)**
- **45.3 Phase 45 Quality Gate Review (+ README datasheet row)**

---

### Phase 49 — Create-Path Access Compatibility (M63, Critical)

**Scope (Steps):**

- **49.1 Page-0 tail reverse-engineering: structure, semantics, maintenance (M63, DAO oracle)**
- **49.2 Writer: emit + maintain the tail and the remaining region fields (M63)**
- **49.3 Compatibility verification: DataCraft-created file opens in real Access (M63, user gate)**
- **49.4 User data recovery through the fixed writer + Phase 49 gate**
**Verification:** Verification: canonical verification set green on the final tree (fmt, clippy -D warnings, workspace tests 160/160 suites, llvm-cov >= 95 % lines, file-size gate). Step 49.3: the ten-step real-Access checklist passed end to end (user-verified 2026-11-07); the pre-flight found and fixed K4 (seed semantics). Step 49.4: the incident database (database.mdb, byte-identical backup) was identified, the engine refusal re-proven without the lock confound, SaveDatabaseAs proven unable to repair (M67), and the data recovered through the create path via examples/recover_mdb.rs — engine-accepted, value-parity per table, autonumber continues across engines; user confirmed the data in Access. K5 (restore writes left the seed behind) fixed test-first on the way. (checkpoint: 5088f6e)

---

### Phase 48 — Walkthrough Fixes III

**Scope (Steps):**

- **48.1 Record navigation reads the cursor it writes (M60)**
- **48.2 Datasheet trailing elements stay above the status bar (M61)**
- **48.3 Filter dialog documents the criteria syntax (M62)**
- **48.4 Phase 48 Quality Gate Review**
**Verification:** Canonical verification set green on the final tree: fmt clean, clippy -D warnings clean, 160 workspace test suites ok, file-size gate clean. Coverage: ui excluded (registered exception #1); the local Windows lcov parse reads 94.94% — unchanged by the phase (0 ui entries in the export) and not comparable to the binding ubuntu CI gate, which has not run since 2026-11-03 (152 unpushed commits at gate time; push pending the human's call). Steps: 48.1 M60 (red 0bc35e6, feat 7496d5b), 48.2 M61 (red 2f4b86c, feat b0a41b6 — the add-record reachability fix), 48.3 M62 (red 2f3e1f3, feat d4353fe); executed 48.2 first per the human's add-record priority. (checkpoint: 1061922)

---

### Phase 51 — Changelog cadence

**Scope (Steps):**

- **51.1 Changelog: per-phase cadence in policy and AGENTS, coarse Unreleased catch-up (L38)**
- **51.2 Phase 51 Quality Gate Review**
**Verification:** Docs-only phase: the Rust tree is unchanged since the green canonical sweep at the phase 48 gate (01c261f); hygiene scan clean on all touched files; docs_validate green. The binding ubuntu CI gate is currently blocked by an infra-level linker failure (H55) — unrelated to this phase's docs. (checkpoint: 482ec1c)

---

### Phase 52 — CI ubuntu diagnosis

**Scope (Steps):**

- **52.1 Test job: disk evidence and headroom before the build (H55, High gate released 2026-11-07)**
- **52.2 Phase 52 Quality Gate Review (Tests (ubuntu-latest) passes; root cause recorded)**
**Verification:** Canonical gates green on CI run 42411956901: all 10 jobs success including Tests (ubuntu-latest) with the H55 headroom step (15 GB -> 37 GB free; root cause disk exhaustion), macOS tests + oracle green (not cancelled). (checkpoint: ad5a1a5)

---

### Phase 50 — CI macOS re-admission

**Scope (Steps):**

- **50.1 Re-admit macOS to the CI matrices (L25, High gate released 2026-11-07) and align the README matrix rows (H54)**
- **50.2 Phase 50 Quality Gate Review (a green CI run on main with the macOS jobs not cancelled)**
**Verification:** Canonical verification green on CI main run 42411956901 (all 10 jobs success; the ubuntu coverage gate reads >= 95 % from the exported lcov, confirming the local Windows 94.94 % reading as the known non-comparable local number). macOS matrix re-admission (L25/H54) accepted on this run; the ubuntu diagnosis (H55) closed in phase 52. (checkpoint: ad5a1a5)

---

### Phase 53 — Access co-existence

**Scope (Steps):**

- **53.1 Jet/ACE open: a sibling .ldb forces a read-only session with the lock-file notice in the snapshot (M59)**
- **53.2 UI: status bar and Info backstage render the localized notice (M59)**
- **53.3 CLI/MCP wire surfacing + FORMAT-JET co-existence section + README matrix row (M59)**
- **53.4 Phase 53 Quality Gate Review**
**Verification:** Canonical set green on the final tree: fmt, clippy -D warnings, 160 workspace test suites, file-size and hygiene gates clean. Local coverage parse 94.93% (ui excluded; the binding gate is the ubuntu CI run this push triggers). CHANGELOG carries the phase entry per the L38 cadence. Steps: 53.1 (red a8c8fe1, feat fddc268), 53.2 (red 2c50188, feat 828ab0c), 53.3 (0dba93c). (checkpoint: 0dba93c)

---

### Phase 54 — Next-version note

**Scope (Steps):**

- **54.1 Changelog next-version note + PUBLISHING policy wording (L39)**
- **54.2 Phase 54 Quality Gate Review**
**Verification:** Docs-only phase: hygiene scan clean, fmt/clippy/tests green (160 suites); the Rust tree is unchanged since the phase 53 gate. (checkpoint: cfd8b1f)

---

### Phase 56 — Release readiness

**Scope (Steps):**

- **56.1 Publish metadata: repository + readme fields on the six crates (L24, Medium gate: Cargo.toml touch — needs the human release)**
- **56.2 Lexer: drop the expect on the float parse path (M65)**
- **56.3 Phase 56 Quality Gate Review (cargo package --list clean, no warnings)**
**Verification:** Canonical set green on the final tree (fmt, clippy -D warnings, 160 suites, size gate; tree clean — LESSONS #14 sequence followed). No changelog entry per policy: crate metadata and an unreachable-path refactor carry no observable surface. The publish-relevant facts live in the README/PUBLISHING docs and the L24 archive note. (checkpoint: 31190fb)

---

### Phase 55 — Docs truth pass

**Scope (Steps):**

- **55.1 Coverage-gate wording across the binding docs (M64)**
- **55.2 Save-as boundary + recovery vehicle in FORMAT-JET/README (M67)**
- **55.3 Journal batch commands documented headless-only (L26)**
- **55.4 Translate the German step titles in the PROGRESS permanent index (L29)**
- **55.5 Phase 55 Quality Gate Review**
**Verification:** Canonical set green on the final tree: fmt, clippy -D warnings, 160 workspace suites, file-size gate; the local coverage parse reads 94.91% (the only Rust change is command.rs doc comments — the binding gate is the ubuntu CI run this push triggers). No changelog entry per policy (no observable surface). Steps: 55.1 M64 (42d6862), 55.2 M67 (97b180b), 55.3 L26 (3d0978d), 55.4 L29 (1a586a2). (checkpoint: 1a586a2)

---

### Phase 57 — Report grouping picker

**Scope (Steps):**

- **57.1 Report designer grouping picker dispatching SetReportGrouping (M48)**
- **57.2 Phase 57 Quality Gate Review**
**Verification:** No Rust change in this phase (ROADMAP §16 hygiene note + stepwell files only); the canonical set stands from the 55.5 gate on the identical Rust tree (fmt, clippy, 160 suites, size gate) and the grouping suite re-ran green (2/2) as the M48 acceptance evidence; the phase 55 CI run is the binding arbiter. (checkpoint: 1a586a2)

---

### Phase 58 — Import File

**Scope (Steps):**

- **58.1 Import File flow: picker, preview, target name, write-path fill (M50)**
- **58.2 Capability-disabled rendering pin + source-file byte-identity (M50)**
- **58.3 Phase 58 Quality Gate Review**
**Verification:** Canonical set green on the final tree: fmt, clippy -D warnings, 162 workspace suites (the two import kittests included), file-size gate clean after the L30 splits; local coverage parse 94.72% (the binding gate is the ubuntu CI run this push triggers). Changelog entry per the L38 cadence; README feature row added. (checkpoint: f9d8d51)

---

### Phase 59 — CI hardening

**Scope (Steps):**

- **59.1 Crate dependency-boundary guard script in CI (L32, High gate: needs the human release)**
- **59.2 Gate rustdoc warnings with -D warnings (L33, High gate)**
- **59.3 CLI smoke step on both matrix OSes (L34, High gate)**
- **59.4 Phase 59 Quality Gate Review**
**Verification:** Every gate green on the final tree — including the three new ones run locally (guard self-test + tree check, rustdoc -D warnings, CLI smoke exit 0) — plus fmt, clippy, 162 suites, file-size gate. AGENTS §1 cross-references the guard; the README CI row and the PUBLISHING required-checks list carry the new gates. No changelog entry per policy (no crate surface). (checkpoint: 68de28f)

---

### Phase 60 — Small closes

**Scope (Steps):**

- **60.1 Expression window: row-context picker dispatching RunExpression with Some(ExpressionContext) (L27)**
- **60.2 Oracle skip observable: #[ignore] + the CI oracle job runs --ignored (L31)**
- **60.3 Phase 60 Quality Gate Review**
**Verification:** Every gate green with explicit exit codes (fmt, clippy -D warnings, 162 suites, file-size gate). Coverage unchanged by construction: the only Rust changes live in the excluded ui crate and a test file (oracle.rs) — the binding gate is the ubuntu CI run this push triggers. No changelog entry per policy (no observable crate surface). (checkpoint: 3e6f808)

---

### Phase 61 — Fixture generators

**Scope (Steps):**

- **61.1 Deterministic generator functions for canonical test databases, byte-deterministic, alongside the binary pins (L35)**
- **61.2 Phase 61 Quality Gate Review**
**Verification:** Every gate green with explicit exit codes: fmt, clippy -D warnings, 163 suites (the new generated_databases suite included), file-size gate, and the fixture manifest hash check — the pins and the oracle suite are untouched. No changelog entry per policy (test infrastructure). (checkpoint: c62ff1a)

---

### Phase 62 — Failure codes

**Scope (Steps):**

- **62.1 Core: stable machine-readable failure codes alongside the English text, capability rejections mapped onto the DisabledReason vocabulary (I6 step 1)**
- **62.2 UI: known codes render localized, unknown codes stay English (I6 step 2)**
- **62.3 Wire consumers switch to code assertions (command_errors, cli_contract, server_e2e, driver_caps) (I6)**
- **62.4 Phase 62 Quality Gate Review**
**Verification:** Every gate green on the final tree: fmt, clippy -D warnings, 163 suites, file-size gate (database/rows.rs is the L30 split forced by the sweep's code arguments); the local coverage parse reads 94.51% — the binding gate is the ubuntu CI run this push triggers. Changelog entry per the L38 cadence (the wire change is observable). Steps: 62.1 f00dbbe, 62.2 b5631cf, 62.3 4ee7572. (checkpoint: 494a47f)

---

### Phase 63 — Control content editing

**Scope (Steps):**

- **63.1 Core: SetFormControlContent/SetReportControlContent with precise validation (H56)**
- **63.2 UI: the selected-control content panel in both designers, kittest-pinned (H56)**
- **63.3 Phase 63 Quality Gate Review**
**Verification:** Canonical set green on the final tree (fmt, clippy -D warnings, 165 suites, file-size gate after the two splits); changelog entry per the L38 cadence (user-visible designer feature); the GUI relaunches from this build. (checkpoint: f3615dd)

---

### Phase 64 — Form runtime completion

**Scope (Steps):**

- **64.1 M69 decision record: Access-verified form runtime decisions in ARCHITECTURE §5 and the control matrix**
- **64.2 M68: bound text boxes edit through a per-record buffer (Esc field/record) with the Access ZLS rule**
- **64.3 M70: two-state checkbox toggle through the record edit buffer**
- **64.4 M70: combo row source (table/query) + display field, all rows, free entry, optional source**
- **64.5 M71: form-scoped WHERE filter with filtered record navigation**
- **64.6 M69: core ButtonAction model, control-data command and precise validation**
- **64.7 M69: designer action picker and runtime click dispatch (open form/report, run query, apply filter)**
- **64.8 Phase 64 Quality Gate Review**
**Verification:** Canonical set green: fmt, clippy -D warnings, full workspace tests; lcov line coverage 17525/18337 = 95.572 % (gate >= 95); file-size gate clean (no .rs >= 500); docs_validate clean. README/CHANGELOG/ROADMAP updated; M68/M69/M70/M71 archived.

---

### Phase 65 — Server sources (SourceSpec)

**Scope (Steps):**

- **65.1 Core: SourceSpec {Path, Uri} on the open/create commands — file:// resolution, precise no-server-driver boundary; carries the forced L30 command.rs split (L8)**
- **65.2 Core: snapshot source display; the sidecar applies to Path sources only (L8)**
- **65.3 UI: backstage Open accepts path or file:// URI and renders the source display, kittest pin (L8)**
- **65.4 Wire: URI sources flow through CLI and MCP with server_e2e pins — all four clients, file behavior byte-identical (L8)**
- **65.5 Phase 65 Quality Gate Review**

**Goal:** Remove the filesystem-path assumption from the open surface (L8,
ADR-7's documented future server-driver path): an additive `SourceSpec
{Path, Uri}` on the open/create commands, a source display in the snapshot,
and the sidecar derivation (ADR-1) scoped to `Path` sources. Non-file URI
schemes answer the precise no-server-driver boundary — the first actual
server driver (credentials/SSL, synchronous behind the no-async rule) stays
out of scope, and the ADR-7 extension note is deferred to its planning
(per the item's acceptance). The standing L30 rule rides the step that
touches a band file: `command.rs` (499), `app.rs` (496) and
`server_e2e.rs` (484) sit at the limit and carry their forced splits.
Risk classes: all steps are Low (source/test edits within step scope); no
new dependencies, fixtures, or CI changes — L36/L37 stay parked per
ROADMAP §17 (no dedicated split phases; no released step touches
`expr`/`schema`/`native_exec` here beyond the L30 module splits).

**Acceptance:** A URI source opens through all four clients (GUI backstage,
CLI, MCP, direct bus) — `file://` URIs resolve to the existing file path
with byte-identical behavior; other schemes answer the precise documented
boundary. File-source behavior is unchanged everywhere (open, sidecar
location, recent list). The snapshot carries a source display. The L30
splits keep every `.rs` file below 500 lines. Canonical verification set
green; docs stay honest (no compatibility claims beyond the verified
matrix; no server-driver claims).
**Verification:** Canonical set green with explicit exit codes (fmt, clippy -D warnings, full workspace tests, file-size gate) and the binding CI run 42888059548 fully green on main — 13/13 jobs including the coverage gate reading 17671/18466 = 95.695% (the local Windows parses read ~94.1-94.2%, the documented MSVC attribution drift; ubuntu is the binding arbiter per PLAYBOOK §5). LESSONS walk with verdicts: evil-input contracts pin truncated/invalid percent escapes, remote file hosts, query/fragment parts and scheme misuse (A1/A7); the source seam is pinned from both sides — core MockPort contract suites plus the real detecting stack over the wire, the real CLI subprocess and the real MCP binary (A2/A3); every malformed URI answers a precise, locatable error, no unwrap on parse paths (A4); the crafted-URI surface allocates nothing beyond the input string and touches no filesystem before resolution (C7); no async, no unsafe, no dependency change (D11-D13 N/A). Changelog entry per the L38 cadence; README feature row and ARCHITECTURE §5 source paragraph updated. L8 archived with the commit chain; L30 stays open as the live band list (its rule is codified in PLAYBOOK §5 / LESSONS #15); L36/L37 stay parked per ROADMAP §17. Five forced L30 splits this phase: command.rs (498 via source.rs extraction), command_bus/tabs.rs (333+176), journal/wire_and_errors.rs (314+200), backstage/info.rs (438+91), server_e2e/vba.rs (452+88). (checkpoint: aa64402)

---

### Phase 66 — Release draft step

**Scope (Steps):**

- **66.1 M72: the release draft step composes its notes from CHANGELOG's dated section (High gate released 2026-11-10)**
- **66.2 Phase 66 Quality Gate Review**
**Verification:** Gate review for the workflow-only phase: the Rust tree is byte-identical to the fully green CI run 42888059548 (canonical set stands per the phase-57 precedent, and the push re-runs the full matrix); the extraction logic was simulated locally in both branches (0.1.0 -> 82 lines verbatim from "### Added"; 0.1.1 -> the loud L39 refusal), which also caught the broken awk -v argument form pre-commit; the workflow step itself runs with set -euo pipefail and an explicit non-empty guard. M72 stays open by design: its acceptance (a tag-triggered run fully green including the draft step) is proven live by the next release — the natural vehicle is the 0.1.1 cut, which also carries the changelog heading the new step will extract. (checkpoint: d683572)

---

### Phase 67 — Release draft step, fix forward

**Scope (Steps):**

- **67.1 M72 fix 2: the publish job checks out before downloading the artifact — checkout's clean wipes dist otherwise (v0.1.1 live finding)**
- **67.2 Phase 67 Quality Gate Review**
**Verification:** Gate review for the workflow-only fix-forward phase: step order pinned by inspection, the create step unchanged from the v0.1.1-proven extraction, M72 carries both root causes (generate-notes without checkout; checkout clean after the download), the v0.1.1 draft was salvaged from the verified bundle, and the canonical set stands on the identical Rust tree (CI run 42895207902) — the push re-runs the matrix. (checkpoint: c8000a3)

---

### Phase 68 — Feedback & honesty sweep (GUI stage 0)

**Scope (Steps):**

- **68.1 Undo/Redo keyboard shortcuts: Ctrl+Z/Ctrl+Y dispatch through the bus (M73)**
- **68.2 Tooltip registry: spec-complete, localized tooltips for ribbon, QAT and backstage (M74)**
- **68.3 Notice and error history with the messages dialog (M76); carries the forced L30 app.rs/flows.rs splits**
- **68.4 Keyboard-shortcuts help page on F1 (M77)**
- **68.5 Busy overlay through the deferred-dispatch seam for blocking commands (M75); app/busy.rs split**
- **68.6 Window-chrome polish pass from the human's live review (M78) — blocked on the review**
- **68.7 Phase 68 Quality Gate Review**

**Goal:** Close ROADMAP §16 stage 0 — the feedback & honesty sweep — as a
class instead of per instance: the keyboard reaches the shipped Undo/Redo
pair (Ctrl+Z/Ctrl+Y, M73), every ribbon/QAT/backstage control explains
itself in a localized tooltip (M74), the status bar keeps a retrievable,
bounded notice and error history (M76), F1 opens a shortcuts help page
(M77), blocking commands paint a busy overlay before they can freeze an
unpainted window (M75, the deferred-dispatch seam), and the deferred M42
polish pass runs on the human's live review (M78). The stage's binding
principles apply throughout: no affordance without feedback (tooltip plus
test pin, `layout_probe` rects where geometry matters), core owns
decisions when a surface is touched, i18n parity for every string, and L30
— every touched 480-499-band file splits within its step. I6 (localized
failure reasons) is already delivered and out of scope here; stages 1-4
of the ladder (windowed reads, filter-by-value, resize handles, command
palette) stay out. L36/L37 remain parked per ROADMAP §17. Risk classes:
all steps are Low (Rust/UI source and test edits within the step scope);
no new dependencies, fixtures or CI changes.

**Acceptance:** Every sweep item carries its test-first chain and its
pins: the Ctrl+Z/Y round-trip and the busy ordering are kittest-verified
frame by frame; the tooltip and shortcut tables are exhaustive-match
tested against the ribbon spec and the `Shortcut` enum; the notice
history is capped, ordered and clearable; the M42 review list is either
fixed or filed as items. `app.rs`, `flows.rs` and `ribbon.rs` land below
480 lines with their forced splits. The canonical verification set is
green on CI (fmt, clippy -D warnings, workspace tests, the lcov coverage
gate >= 95 %, file size < 500, docs_validate clean) and README, CHANGELOG
and ROADMAP state the sweep honestly.
**Verification:** canonical set green on CI run 43056361337 (fmt/clippy/tests ubuntu+windows+macos, coverage 95.6, hygiene, boundaries, rustdoc); the only red job is the ubuntu oracle apt install failing on DNS three times (L40), macOS oracle green

---

### Phase 69 — M42 review findings

**Scope (Steps):**

- **69.1 Hide MSys* system relationships from the mapper and never offer their deletion (H57)**
- **69.2 Ribbon toggle actions render their open state from the snapshot (M79)**
- **69.3 The shell follows lifecycle actions: open -> Home, create -> the new document (M80)**
- **69.4 A help glyph in the ribbon tab row opens the shortcuts page (M82)**
- **69.5 The status filter chip lists and removes individual filters (M83)**
- **69.6 Column headers show filter and sort state (M84)**
- **69.7 Tool windows open centered instead of top-left (M85)**
- **69.8 Phase 69 Quality Gate Review**

**Goal:** Close the M42 live review of 2026-11-11 as its own finding
batch: the relationship mapper stops presenting the file's MSys* system
metadata as user relationships and never offers their deletion (H57); the
three toggle actions show whether their window is open (M79); the shell
follows lifecycle actions — an open returns to Home, a create activates
the new document (M80); a help glyph in the ribbon tab row makes the
shortcuts page discoverable (M82); the status filter chip lists and
removes individual filters (M83); the table header shows filter and sort
state per column (M84); every tool window opens centered instead of
top-left (M85). M78 stays open for the remaining token polish (the dark
mode contrast check on the mapper's combo fields). All steps are UI
behavior/chrome — Low class except the documented system-relationship
filter (H57), which touches the snapshot contract and keeps the headless
behavior honest. No new dependencies, fixtures or CI changes.

**Acceptance:** Every finding's own acceptance holds: kittest pins per
behavior (mapper filtering, toggle rendering, lifecycle transitions,
filter chip and header indicators, centered windows); H57's filter is
documented on the snapshot and pinned by a core contract test; the
shortcuts page stays reachable from F1 and the new glyph. The canonical
verification set is green on CI and the band files touched here keep
their L30 splits.
**Verification:** Canonical set green locally (fmt, clippy -D warnings, workspace tests, lcov 17665/18478 = 95.600 %) and CI run 43074717327 fully green 13/13 (tests ubuntu/windows/macos, coverage, hygiene, boundaries, rustdoc, both oracles — the L40 DNS flake was absent this run); the grid.rs -> grid_model.rs split landed here; L30's live list refreshed; items H57/M79/M80/M82/M83/M84/M85 archived.

---

### Phase 70 — Review corrections

**Scope (Steps):**

- **70.1 Wildcard patterns become an implicit Like for text filters (H58)**
- **70.2 Notice and error history stores messages semantically and follows a live language switch (M86)**
- **70.3 Tool-window contrast and the accent frame in both palettes — tokens only (M78)**
- **70.4 Phase 70 Quality Gate Review**

**Goal:** Close the three findings from the second M42 review round
(2026-11-11): wildcard filter patterns become an implicit `Like` for text
columns instead of literal equality (H58); the notice/error history stores
messages semantically and translates at render time, so a live language
switch cannot leave mixed-language entries (M86); and the tool windows
raise their contrast in both palettes and carry the accent as a structural
frame — tokens only, no layout changes (M78). All steps are Low-class
UI/core edits; M78 stays inside `theme.rs` below the file gate. No new
dependencies, fixtures or CI changes.

**Acceptance:** H58: `*a*` matches on Name in the table filter and the QBE
path, while `=` stays literal, pinned on both. M86: a switch to German
after an English notice re-renders the history and the dialog; the log
stays bounded and ordered. M78: the mapper and the other tool windows show
a visible frame/title structure and legible secondary text in light and
dark; the palette contrast tests stay green. Canonical set green on CI; no
`.rs` file ≥ 500 lines.
**Verification:** Canonical set green locally (fmt, clippy -D warnings, workspace tests, lcov 17673/18486 = 95.602 %) and CI run 43105933972 fully green 13/13 (tests ubuntu/windows/macos, coverage, hygiene, boundaries, rustdoc, both oracles); items H58/M86/M78 archived; docs_validate clean.

---

### Phase 71 — Views

**Scope (Steps):**

- **71.1 Core: ViewDef in the sidecar, snapshot entries, create/delete/set-sql commands, SELECT-only validation, ObjectKind::View opens a tab (M93)**
- **71.2 UI: nav-pane Views group, create/edit/delete dialog, read-only datasheet from the view SQL, kittest (M93)**
- **71.3 SQLite: apply the view natively (CREATE VIEW / DROP VIEW), capability-gated, with contract pins (M93)**
- **71.4 Phase 71 Quality Gate Review**
**Verification:** Canonical set green locally (fmt, clippy -D warnings, workspace tests, lcov 17843/18715 = 95.341 %) and CI run 43127407577 fully green (all 13 jobs); views work end to end (sidecar design, shadow-engine execution, native SQLite apply with rollback); port.rs split kept the size gate green; docs_validate clean.

---

### Phase 72 — Object rename (M81)

**Scope (Steps):**

- **72.1 Core rename command, name rules and reference follow-through (M81)**
- **72.2 Jet/ACE table rename through the writer (M81)**
- **72.3 SQLite native rename and CSV boundary (M81)**
- **72.4 Nav-pane rename surface: context menu, F2, dialog (M81)**
- **72.5 Documentation and phase gate (M81)**

**Goal:** Give Access's rename to DataCraft — the live review (2026-11-11)
and the human's requirement (2026-11-12) report that objects cannot be
renamed, so a user must recreate a table/query/form/report. One
kind-tagged `RenameObject { object, new_name }` bus command (the kind
rides the existing `ObjectId`) covers all four kinds. A table rename is
physical: Jet/ACE updates the catalog row's `Name` (the TDEF carries no
table name) and every `MSysRelationships` row, SQLite runs
`ALTER TABLE … RENAME`; sidecar objects rename in memory. References
follow: open tabs, per-table datasheet view state, form filters, the
sidecar's query/form/report sources, combo row sources and button
actions. Stored view SQL text is deliberately not rewritten (free SQL,
not a structural reference) and keeps its precise execution failure. The
CSV driver answers its capability boundary. Low-class throughout (Rust
source/tests only; no dependency, fixture or CI change).

**Acceptance:** One `RenameObject` dispatch renames a table, query, form
and report; the table keeps its rows and the rename persists across
reopen on Jet/ACE and SQLite; relationships follow the table name on both
backends; open tabs, datasheet state, form filters and sidecar
references follow; collisions and empty/whitespace names answer precise
failures (case-only renames are handled per backend and pinned); the
nav-pane context menu and F2 reach the command with a localized dialog
(kittest); CLI/MCP wire parity is pinned; the canonical verification set
is green.

**Verification:** Canonical set (fmt, clippy `-D warnings`, workspace
tests, lcov ≥ 95 %), file-size gate, `docs_validate` clean, CI green.
**Verification:** Canonical set green locally (fmt, clippy -D warnings, workspace tests, lcov 18342/19250 = 95.283 %); file-size gate green (one forced split: datacraft-sqlite/src/lib.rs → rows.rs, 528 → 454); docs_validate clean; CI verification rides the phase push. (checkpoint: 045e1e9)

---

### Phase 73 — SQLite depth stage 0 (M90–M92)

**Scope (Steps):**

- **73.1 Native SQLite views as read-only objects: catalog, snapshot, nav-pane group, datasheet and SQL path (M90)**
- **73.2 Read-only index/trigger inventory per table in the snapshot and a UI surface (M90)**
- **73.3 File facts and a bounded quick_check at open: port channel, snapshot, Info page, precise notice (M91)**
- **73.4 Robust open and honesty: busy_timeout, WAL fact/notice, WITHOUT ROWID and STRICT refusals (M92)**
- **73.5 Documentation and phase gate (M90–M92)**

**Goal:** Open ROADMAP §19 stage 0 — SQLite as a first-class source is
*visible* and *honest* about foreign files, following the human decision of
2026-11-12 and the item wording of M90–M92. A foreign `.sqlite` file's own
views become read-only catalog objects (nav-pane Views group, read-only
datasheet, SQL path), its indexes and triggers get a read-only inventory,
and the Info page reports file facts (`sqlite_version`, page size, journal
mode, encoding, `user_version`, `STRICT`/`WITHOUT ROWID`) plus a bounded
`quick_check` verdict as a notice. The open itself becomes robust: a
bounded `busy_timeout`, a WAL co-existence fact, and precise refusals for
`WITHOUT ROWID` writes instead of a late SQLite error. Trigger logic is
never created or executed (inventory only, ADR-2 analog); the sidecar view
designs from M93 stay the writable design authority. All edits are Low
class (Rust source/tests only; test databases are generated in scratch
directories, no committed fixtures, no dependencies).

**Acceptance:** A generated fixture carrying a native view, an index and a
trigger lists all three read-only in snapshot and nav pane; the native view
opens a read-only datasheet and answers rows through the SQL path, with no
write affordance. The Info page shows the file facts and a `quick_check`
verdict; a deliberately corrupted file answers a precise open notice
instead of a late query error. A WAL file opens and reports its mode; a
`WITHOUT ROWID` table answers a precise `Unsupported` on writes (or, where
chosen, reads in declared-PK order) and `STRICT` is reported; a parallel
writer no longer trips a bare `SQLITE_BUSY` inside the bus timeout. Port
contract pins cover SQLite and the Jet/CSV side of every additive snapshot
field; kittest pins cover the UI surfaces; the canonical verification set
is green.

**Verification:** Canonical set (fmt, clippy `-D warnings`, workspace
tests, lcov ≥ 95 %), file-size gate, `docs_validate` clean, CI green.

---

### Phase 74 — Native Access objects (M87, M89)

**Scope (Steps):**

- **74.1 Native Access forms and reports: read-only MSysObjects inventory in snapshot and nav pane, no definition decoding (M87)**
- **74.2 Time-boxed feasibility verdict on decoding native form/report definitions against the committed SaveAsText exports (M89)**
- **74.3 Documentation and phase gate (M87, M89)**

**Goal:** Make native Access form/report objects *visible* and decide,
with evidence, whether their definitions can ever be decoded from the
file. M87 is the cheap honest first step: the catalog decodes the
`MSysObjects` form/report object types into a read-only inventory
(name, type) on the snapshot, the navigation pane gains a read-only
group with a precise "definitions are not decoded" hint, and no
definition bytes are parsed and no write path exists. M89 is a
time-boxed feasibility study, not a promise: one form blob from the
pinned `programmability.accdb` fixture is correlated with the committed
SaveAsText export of the *same* object (control order, rects, captions,
sources, section structure); the study either establishes the container
and property encoding for a supported subset or records "not feasible
without a specification" with its evidence. Nothing ships without the
verdict; clean-room rules apply (own black-box observation and public
documents only). Low class: Rust source/tests/docs, no dependencies, no
fixture changes, no write path.

**Acceptance:** M87: a fixture with native forms/reports (the
self-created `programmability.accdb`) lists their names and counts in
snapshot and nav pane, the hint names the boundary, and the read-only
group offers no write or definition tab; kittest and the port contract
pins cover the empty sidecars for SQLite/CSV. M89: a written verdict
with a blob↔text mapping for at least one form, a supported-subset
decision, or a documented stop; the study leaves the backlog item
accurately closed (follow-up work filed only when the verdict is
"feasible").

**Verification:** Canonical set (fmt, clippy `-D warnings`, workspace
tests, lcov ≥ 95 %), file-size gate, `docs_validate` clean, CI green.
**Verification:** Canonical set green locally (fmt, clippy -D warnings, workspace tests, lcov 18812/19688 = 95.551 %); file-size gate green (26 files ≥ 455, port_adapter entered the band at 488, no split forced); M89 verdict documented in FORMAT-JET; docs_validate clean; CI verification rides the push. (checkpoint: c85b7af)
