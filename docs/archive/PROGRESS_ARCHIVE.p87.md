# PROGRESS_ARCHIVE.p87.md — Shard (Phase 87, append-only)

> Created 2026-11-16T00:12 — home for the detail block of Phase 87 on completion.
> Contains: Phase 87 — Datasheet navigation: foreign keys and structured filter. Header is write-once — never edited.

---

### Phase 87 — Datasheet navigation: foreign keys and structured filter

**Scope (Steps):**

- **87.1 FK follow on key cells: declared foreign keys, reverse keys and inferred references with composite-key pairing (M103)**
- **87.2 Per-tab back/forward history and capped, cancellable "referenced by" counts (M103)**
- **87.3 Core filter builder model: AND/OR groups with visible parentheses, ordered sort columns, criteria-string serialization (M104)**
- **87.4 Builder UI: draft/apply distinction, text mode as escape hatch, builder-to-criteria round-trip (M104)**
- **87.5 Documentation and phase gate (M103, M104)**
**Verification:** cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo llvm-cov --workspace --exclude datacraft-ui --lcov: line coverage 20891/21871 = 95.519 % (>= 95 %), the .rs size gate green, all RED→GREEN chains in the log; the phase's commits push at the boundary and the CI run gates the close (F7). (checkpoint: 7371a3e)
