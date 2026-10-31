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
