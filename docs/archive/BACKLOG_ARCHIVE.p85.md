# BACKLOG_ARCHIVE.p85.md — Shard (Phase 85, append-only)

> Created 2026-11-15T13:20 — home for item blocks archived in the context of Phase 85 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 85. Header is write-once — never edited.

---

### [x] F13 — Inline fix: EditBuffer::ops() skipped unchanged rows in the evolving index count — 🟠
- **Location:** `crates/datacraft-core/src/edit_buffer.rs` (`ops()`), found by the 85.2 UI suite (staged deletes hit the wrong row).
- **Problem:** The evolving-index counter only advanced for update rows; unchanged rows between a staged delete and a later op kept the counter back, so `Delete{row}` and the following `Update{row}` addressed wrong rows. The 85.1 core tests passed by luck — every delete came before the unchanged tail.
- **Fix:** Unchanged rows advance the counter; the new core test `a_mid_buffer_delete_counts_the_unchanged_rows_behind_it` pins an update staged behind an unchanged neighbour after a mid-buffer delete.
- **Done:** fixed in `d2ff85b` (step 85.2) — ops() advances the evolving index over unchanged rows; pinned by a_mid_buffer_delete_counts_the_unchanged_rows_behind_it

---

### [x] H61 — Datasheet edits are immediate — no pending batch with preview and atomic submit — 🟠
- **Location:** the datasheet edit path (`crates/datacraft-ui/src/datasheet.rs`, `crates/datacraft-ui/src/app/grid_state.rs`) dispatches `UpdateRow`/`InsertRow`/`DeleteRow` immediately; the core already has `preview_write`/`apply_write` and the `atomic_batch` capability (M20), which no grid uses.
- **Problem:** Every cell edit is its own committed row write. There is no pending state, no multi-row transaction and no SQL preview in the GUI, while Access and the reference client collect updates, inserts and deletions, preview the generated statements, submit them together, keep them on failure and lock the grid during submission. The headless batch exists — the datasheet is the missing client.
- **Approach:** A per-tab edit buffer (updates/inserts/deletes) in UI state mapped onto the existing journal batch commands; amber pending markers (selected changed cells keep the marker plus an outline); Preview renders the journal's own statements — no second SQL generator; Ctrl+S applies, Escape discards the active record, a failed apply keeps the buffer, the grid refuses input while applying. Capability-gated on `atomic_batch`; single-cell behaviour on drivers without it stays as today.
- **Acceptance:** Pending updates, inserts and deletes submit as one atomic batch on Jet4 and SQLite; a failed batch leaves every edit pending and the database unchanged; Preview matches the applied statements exactly; the grid is unavailable during submission; no second SQL generator exists; the behaviour and its edge cases (cursor moves inside a batch, save-on-row-change if adopted, read-only sources) are documented.
- **Steps:** none yet.
- **Done:** done in `d42825d` (85.1 buffer vocabulary + snapshot summary), `d2ff85b` (85.2 UI staging, preview, apply; F13 evolving-index fix), `fe3c6ff` (85.3 SQLite atomic batches + gating pins); edge cases and the save-on-row-change decision documented in `docs/FEATURES.md`

## ✅ Done Index

> One-liner per completed item (details above).

- F13 — Inline fix: EditBuffer::ops() skipped unchanged rows in the evolving index count — done (fixed in `d2ff85b` (step 85.2) — ops() advances the evolving index over unchanged rows; pinned by a_mid_buffer_delete_counts_the_unchanged_rows_behind_it)
- H61 — Datasheet edits are immediate — no pending batch with preview and atomic submit — done (done in `d42825d` (85.1 buffer vocabulary + snapshot summary), `d2ff85b` (85.2 UI staging, preview, apply; F13 evolving-index fix), `fe3c6ff` (85.3 SQLite atomic batches + gating pins); edge cases and the save-on-row-change decision documented in `docs/FEATURES.md`)
