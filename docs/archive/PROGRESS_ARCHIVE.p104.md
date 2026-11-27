# PROGRESS_ARCHIVE.p104.md — Shard (Phase 104, append-only)

> Created 2026-11-27T22:38 — home for the detail block of Phase 104 on completion.
> Contains: Phase 104 — Write-parity and display correctness. Header is write-once — never edited.

---

### Phase 104 — Write-parity and display correctness

**Scope (Steps):**

- **104.1 Core: scale-aware decimal display across datasheet, CSV driver and exports (m135)**
- **104.2 Core: Jet orphan inserts/updates refuse against declared relationships (m137, roadmap §13a)**
- **104.3 Documentation and phase gate**
**Verification:** Canonical verification set green at the gate: cargo fmt --check, clippy -D warnings, RUSTDOCFLAGS=-D warnings cargo doc, cargo test --workspace, llvm-cov lcov parse 95.198% (>= 95, measured at 104.2), file-size gate clean. ROADMAP §13 carries ladder (a)'s delivery status; M138 records the one-surface-ring display remainder (report preview, FK-follow labels, grouping values). (checkpoint: 8c0d67e)
