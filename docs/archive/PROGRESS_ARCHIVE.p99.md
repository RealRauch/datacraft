# PROGRESS_ARCHIVE.p99.md — Shard (Phase 99, append-only)

> Created 2026-11-26T20:06 — home for the detail block of Phase 99 on completion.
> Contains: Phase 99 — Export matrix: CSV and JSON (roadmap §11). Header is write-once — never edited.

---

### Phase 99 — Export matrix: CSV and JSON (roadmap §11)

**Scope (Steps):**

- **99.1 Core: ExportTable command — CSV and the JSON schema+rows document, deterministic new-file write, source untouched (roadmap §11)**
- **99.2 UI: export affordance at the datasheet/nav pane with format and path dialog**
- **99.3 Wire: CLI and MCP parity with contract pins and precise refusals**
- **99.4 Documentation and phase gate (README export row, COMPATIBILITY evidence, CHANGELOG)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps and cargo test --workspace are green; the file-size gate reports no file at 500 lines; the exported lcov line rate is 93.81 % on Windows with the new export modules at 95.1 %/96.6 % — CI ubuntu-latest stays the binding coverage gate (TESTING §2 platform note). (checkpoint: 03edea4)
