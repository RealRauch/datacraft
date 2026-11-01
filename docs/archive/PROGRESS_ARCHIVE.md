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
