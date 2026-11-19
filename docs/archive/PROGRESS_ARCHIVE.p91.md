# PROGRESS_ARCHIVE.p91.md — Shard (Phase 91, append-only)

> Created 2026-11-20T00:37 — home for the detail block of Phase 91 on completion.
> Contains: Phase 91 — Design review fixes. Header is write-once — never edited.

---

### Phase 91 — Design review fixes

**Scope (Steps):**

- **91.1 Version display in UI, CLI and MCP (M109)**
- **91.2 SQL window height budget (M110)**
- **91.3 Import File honesty: wrong-surface rejection, single path, path fields across clients (M111, M112, L53)**
- **91.4 Status-line truncation contract — status bar and backstage (L52)**
- **91.5 Datasheet sort: total order with defined NULL placement and honest indicator (M113)**
- **91.6 Datasheet affordance clarity: AutoNumber hint, freeze tooltip and visible frozen state (M114, L54)**
- **91.7 Nav pane: sort-state honesty, whole-row click target, native-object chrome, category dropdown, New View placement (M115, L55–L58)**
- **91.8 Backstage: Save-page section hierarchy, Info fact-value column (M116, L59)**
- **91.9 Tool windows: Relationships scroll, Expression Close placement, VBA token errors, view-editor field labels, notices density, macro numbering (M117–M119, L60–L62)**
- **91.10 Form designer/runtime: visible selection ring, runtime combo caret, canvas scroll at the pinned minimum (M120–M122)**
- **91.11 Report preview: page geometry and text scale (M123, L63)**
- **91.12 Form record navigation: first/last/new-record (M124)**
- **91.13 Documentation and phase gate (M109–M124, L52–L63)**
**Verification:** cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps; cargo test --workspace (252 binaries green — the first run hit a transient LNK1104 AV/SDK file lock, resolved by retry with reduced parallelism); cargo llvm-cov --workspace --exclude datacraft-ui --lcov --output-path lcov.info; file-size gate empty; the ≥ 95 % line gate is binding on the CI ubuntu-latest run; CHANGELOG carries the phase's user-visible changes
