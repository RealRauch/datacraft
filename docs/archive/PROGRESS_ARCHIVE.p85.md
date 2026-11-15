# PROGRESS_ARCHIVE.p85.md — Shard (Phase 85, append-only)

> Created 2026-11-15T13:45 — home for the detail block of Phase 85 on completion.
> Contains: Phase 85 — Datasheet pending-edit batch. Header is write-once — never edited.

---

### Phase 85 — Datasheet pending-edit batch

**Scope (Steps):**

- **85.1 Core: per-tab edit buffer vocabulary mapping datasheet updates/inserts/deletes onto the journal batch commands, RED first (H61)**
- **85.2 UI: pending markers, statement preview from the journal, Ctrl+S apply / Escape discard, grid lock during submission (H61)**
- **85.3 Capability gating without atomic_batch, read-only sources and the batch edge cases: cursor moves inside a batch, save-on-row-change decision (H61)**
- **85.4 Documentation and phase gate (H61)**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace are green (224 suites); edit_buffer.rs and the touched SQLite paths fully covered from the exported lcov; the phase gate push runs the ubuntu CI as the binding coverage gate (checkpoint: fe3c6ff)
