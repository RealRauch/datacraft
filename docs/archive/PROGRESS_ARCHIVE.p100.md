# PROGRESS_ARCHIVE.p100.md — Shard (Phase 100, append-only)

> Created 2026-11-26T20:53 — home for the detail block of Phase 100 on completion.
> Contains: Phase 100 — Remote Stage 0: linked-table inventory (roadmap §9). Header is write-once — never edited.

---

### Phase 100 — Remote Stage 0: linked-table inventory (roadmap §9)

**Scope (Steps):**

- **100.1 Jet: linked-table record decode — MSysObjects types 4/6, CONNECT/DATABASE, foreign name and flags, read-only (roadmap §9 stage 0)**
- **100.2 Core: link inventory in the snapshot, capability honesty — no execution, no network, never written**
- **100.3 UI: read-only linked-table surface with localized labels**
- **100.4 Wire pins, documentation and phase gate (README, COMPATIBILITY, FORMAT-JET, CHANGELOG)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps and cargo test --workspace -j 2 are green; the file-size gate reports no file at 500 lines; the exported lcov line rate is 93.84 % on Windows (the new link surfaces at 100 %, present 97.4 %, snapshot 97.6 %, catalog 98.8 %) — CI ubuntu-latest stays the binding coverage gate (TESTING §2 platform note). Fixtures: three link databases pinned in fixtures/manifest and docs/FIXTURE-SOURCES.md §9. (checkpoint: 0609ca1)
