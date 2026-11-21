# PROGRESS_ARCHIVE.p92.md — Shard (Phase 92, append-only)

> Created 2026-11-21T18:38 — home for the detail block of Phase 92 on completion.
> Contains: Phase 92 — Open items sweep: macOS shortcuts, dock host, nav pane, render mode. Header is write-once — never edited.

---

### Phase 92 — Open items sweep: macOS shortcuts, dock host, nav pane, render mode

**Scope (Steps):**

- **92.1 Platform-aware shortcut matcher and labels — macOS ⌘, redo ⌘⇧Z, help/tooltip labels (M99)**
- **92.2 Docked task-pane host — dock right/bottom/float with the SQL window as pilot, per-surface geometry persistence (M108)**
- **92.3 Nav pane: search/filter across categories with a clear empty state (L48)**
- **92.4 Nav pane: multi-select with Shift/Ctrl, context menu acts on the selection (L48)**
- **92.5 Nav pane: list/grid overview, local groups and colours persisted per database in ui.json (L48)**
- **92.6 Low-power/RDP rendering mode — detection, override>setting>detection precedence, measurable effects, overlay (L51)**
- **92.7 Documentation and phase gate (M99, M108, L48, L51)**
**Verification:** Verification: docs_validate clean; the workspace suite green (cargo test --workspace, 259 test binaries), fmt/clippy -D warnings/rustdoc/file-size clean locally; FEATURES.md carries the nav-pane, dock-pane, shortcut and render-mode behaviour; CHANGELOG's Unreleased names the sweep; the phase's commits pushed to main with a fully green CI run (see the push at the gate).
