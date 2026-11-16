# PROGRESS_ARCHIVE.p88.md — Shard (Phase 88, append-only)

> Created 2026-11-16T18:30 — home for the detail block of Phase 88 on completion.
> Contains: Phase 88 — Shell chrome: ribbon overflow, tabs, glyphs. Header is write-once — never edited.

---

### Phase 88 — Shell chrome: ribbon overflow, tabs, glyphs

**Scope (Steps):**

- **88.1 Ribbon overflow: group-overflow layout math in core and the overflow control with identical command dispatch (M98)**
- **88.2 Tab strip scroll-to-selected on activation and reachability by pointer and keyboard (L45)**
- **88.3 Close-to-side bus commands and the idle-tab reuse policy in the open-object path (L49)**
- **88.4 Glyph coverage test over both catalogs and hostile names with an explicit painted-glyph allowlist (T11)**
- **88.5 Documentation and phase gate (M98, L45, L49, T11)**
**Verification:** cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps && cargo test --workspace — all green (240 test binaries); the exported lcov parses to 21003/21984 = 95.538 % >= 95 (datacraft-ui exception #1); the file-size gate is clean after compacting command.rs to 497 lines; CHANGELOG carries the coarse phase-88 entry and FEATURES the three chrome behaviors; green CI run on the pushed phase commits closes the phase verification (checkpoint: 00ca6ea)
