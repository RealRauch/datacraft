# PROGRESS_ARCHIVE.p98.md — Shard (Phase 98, append-only)

> Created 2026-11-22T03:40 — home for the detail block of Phase 98 on completion.
> Contains: Phase 98 — SQLite SQL pushdown, stage 1 (roadmap §19). Header is write-once — never edited.

---

### Phase 98 — SQLite SQL pushdown, stage 1 (roadmap §19)

**Scope (Steps):**

- **98.1 Core: read-only native SQL execution against a SQLite source — full dialect, index use, views, EXPLAIN QUERY PLAN; writes stay refused on the bus path**
- **98.2 UI: the SQL window names the dialect per source (SQLite native vs the shadow engine), EXPLAIN QUERY PLAN rendering**
- **98.3 Wire: CLI and MCP parity with contract pins; Jet/CSV sources stay on the shadow engine unchanged**
- **98.4 Documentation and phase gate (README SQL row, COMPATIBILITY evidence); the phase-98 method retrospective checkpoint (PLAYBOOK §8) rides the gate**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps, cargo test --workspace and the file-size gate all green; the lcov export parses 22251/23360 = 95.253 % line coverage (>= 95 %); the phase-98 method retrospective (PLAYBOOK §8) concludes all rules stay, recorded in AGENTS §11.
