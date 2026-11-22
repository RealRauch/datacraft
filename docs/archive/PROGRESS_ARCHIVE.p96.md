# PROGRESS_ARCHIVE.p96.md — Shard (Phase 96, append-only)

> Created 2026-11-22T02:47 — home for the detail block of Phase 96 on completion.
> Contains: Phase 96 — Excel read driver .xlsx/.xlsm/.xls/.xlsb (roadmap §7, read-only first). Header is write-once — never edited.

---

### Phase 96 — Excel read driver .xlsx/.xlsm/.xls/.xlsb (roadmap §7, read-only first)

**Scope (Steps):**

- **96.1 Driver crate skeleton: datacraft-xlsx (ADR-7, core-only dependency), calamine pin in workspace deps, sniff, DatabaseFormat variant (Medium gate: new crate + dependency — needs the human release per occurrence)**
- **96.2 Read path: sheets as tables, typed cell values, the record's format-matrix boundaries (.xlsm values, .xls values, .xlsb values), fixtures pinned (Medium gate: fixtures)**
- **96.3 Composition + UI: registry registration, nav pane and datasheet through the snapshot, read-only capability honesty**
- **96.4 Import File: xlsx as an import source over the M50 chain (roadmap §7 downstream)**
- **96.5 Documentation and phase gate (README matrix, COMPATIBILITY evidence, FIXTURE-SOURCES provenance)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps, cargo test --workspace and the file-size gate all green; the lcov export parses 21989/23079 = 95.277 % line coverage (>= 95 %); the four Excel families deliver identical inferred kinds and typed values in the pinned fixtures.
