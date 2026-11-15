# BACKLOG_ARCHIVE.p85.md — Shard (Phase 85, append-only)

> Created 2026-11-15T13:20 — home for item blocks archived in the context of Phase 85 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 85. Header is write-once — never edited.

---

### [x] F13 — Inline fix: EditBuffer::ops() skipped unchanged rows in the evolving index count — 🟠
- **Location:** `crates/datacraft-core/src/edit_buffer.rs` (`ops()`), found by the 85.2 UI suite (staged deletes hit the wrong row).
- **Problem:** The evolving-index counter only advanced for update rows; unchanged rows between a staged delete and a later op kept the counter back, so `Delete{row}` and the following `Update{row}` addressed wrong rows. The 85.1 core tests passed by luck — every delete came before the unchanged tail.
- **Fix:** Unchanged rows advance the counter; the new core test `a_mid_buffer_delete_counts_the_unchanged_rows_behind_it` pins an update staged behind an unchanged neighbour after a mid-buffer delete.
- **Done:** fixed in `d2ff85b` (step 85.2) — ops() advances the evolving index over unchanged rows; pinned by a_mid_buffer_delete_counts_the_unchanged_rows_behind_it

## ✅ Done Index

> One-liner per completed item (details above).

- F13 — Inline fix: EditBuffer::ops() skipped unchanged rows in the evolving index count — done (fixed in `d2ff85b` (step 85.2) — ops() advances the evolving index over unchanged rows; pinned by a_mid_buffer_delete_counts_the_unchanged_rows_behind_it)
