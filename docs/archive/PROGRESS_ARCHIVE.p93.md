# PROGRESS_ARCHIVE.p93.md — Shard (Phase 93, append-only)

> Created 2026-11-21T20:45 — home for the detail block of Phase 93 on completion.
> Contains: Phase 93 — Dock rollout and the details grid. Header is write-once — never edited.

---

### Phase 93 — Dock rollout and the details grid

**Scope (Steps):**

- **93.1 The Relationships mapper docks through the pane host (M125, M108 follow-up)**
- **93.2 The Expression window docks through the pane host (M125)**
- **93.3 Object Details gains its Type column — the grid overview (L64, L48 follow-up)**
- **93.4 Documentation and phase gate (M125, L64)**
**Verification:** Verification: `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` + `cargo test --workspace` + file-size gate — all green; the coverage gate (>= 95% lines) is CI ubuntu-latest's binding measure (M64) and rides the phase push.
