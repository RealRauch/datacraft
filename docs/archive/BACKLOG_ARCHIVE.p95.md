# BACKLOG_ARCHIVE.p95.md — Shard (Phase 95, append-only)

> Created 2026-11-22T01:27 — home for item blocks archived in the context of Phase 95 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 95. Header is write-once — never edited.

---

### [x] F15 — Inline fix: DetectingPort dropped the session inventory and part of the port surface — 🟢
- **Location:** `crates/datacraft-storage/src/detecting.rs`.
- **Problem:** `DetectingPort` delegated most of the port surface but silently dropped `source_facts`, `native_views`, `indexes`, `triggers`, `native_objects`, `relationship_groups`, `supports_native_views`, `create_native_view`, `drop_native_view`, `rename_table` and `write_row_ops` — the trait defaults answered instead. Real clients therefore saw an empty SQLite inventory, no health facts and a non-native view path, while every driver-level test passed.
- **Fix:** delegate all of them to the open session (defaults only while nothing is open); pinned by `tests/composite_inventory.rs` (facts, inventory, native-view capability and creation/drop through the composite, Jet facts in the snapshot).
- **Found during:** phase 95, step 95.1 — the health facts had to reach the snapshot through the real composition.
- **Done:** c78c7be

## ✅ Done Index

> One-liner per completed item (details above).

- F15 — Inline fix: DetectingPort dropped the session inventory and part of the port surface — done (c78c7be)
