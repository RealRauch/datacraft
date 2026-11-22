# PROGRESS_ARCHIVE.p95.md — Shard (Phase 95, append-only)

> Created 2026-11-22T01:59 — home for the detail block of Phase 95 on completion.
> Contains: Phase 95 — Compact & Repair: pre-open health check and the compact command (roadmap §10, salvage stays a later phase). Header is write-once — never edited.

---

### Phase 95 — Compact & Repair: pre-open health check and the compact command (roadmap §10, salvage stays a later phase)

**Scope (Steps):**

- **95.1 Core: pre-open health check — header/region/page sanity at open, read-only, health facts in the snapshot (roadmap §10 row 3)**
- **95.2 Core: compact — read everything, write a fresh verified Jet4/ACE file, atomic swap, the sidecar follows the path; Jet/ACE-scoped (roadmap §10 row 1)**
- **95.3 UI: health facts and the Compact affordance on the backstage/Info, capability-gated, localized**
- **95.4 Wire: compact and health through CLI and MCP with contract pins — all four clients**
- **95.5 Documentation and phase gate (roadmap §10 — README matrix, COMPATIBILITY evidence, CHANGELOG)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps, cargo test --workspace and the file-size gate all green; lcov export parses 21366/22426 = 95.273 % line coverage (>= 95 %); real Access 2026-11-22 opened both compacted variants with the expected four-table inventory.
