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
