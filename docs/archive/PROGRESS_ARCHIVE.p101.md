# PROGRESS_ARCHIVE.p101.md — Shard (Phase 101, append-only)

> Created 2026-11-26T23:14 — home for the detail block of Phase 101 on completion.
> Contains: Phase 101 — Datasheet completeness (roadmap §16 stage 1). Header is write-once — never edited.

---

### Phase 101 — Datasheet completeness (roadmap §16 stage 1)

**Scope (Steps):**

- **101.1 Core: filter-by-value model — bounded distinct values, deterministic order, applied through the existing criteria path**
- **101.2 UI: the filter-by-value dialog at the column header**
- **101.3 UI/core: find/replace over the visible rows, replacements staged through the edit buffer**
- **101.4 UI: column drag-reorder and the visual Null marker in both palettes**
- **101.5 Core/port: windowed row reads with bounded caches and scroll-follow — the invasive stage-1 item**
- **101.6 Documentation and phase gate (README datasheet row, COMPATIBILITY evidence, CHANGELOG)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps and cargo test --workspace -j 2 are green; the file-size gate reports no file at 500 lines (datasheet.rs 496, grid.rs 497 after the body split and comment trims); the exported lcov line rate is 93.74 % on Windows with the new surfaces at 94-100 % — CI ubuntu-latest stays the binding coverage gate (TESTING §2 platform note). M132 and M136 are Done; the header drag gesture for column moves stays an open gap (recorded in COMPATIBILITY). (checkpoint: 6f4b48d)
