# BACKLOG_ARCHIVE.p101.md — Shard (Phase 101, append-only)

> Created 2026-11-26T23:06 — home for item blocks archived in the context of Phase 101 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 101. Header is write-once — never edited.

---

### [x] M136 — Windowed datasheet reads need a row-count port contract — 🟡
- **Location:** `crates/datacraft-core/src/port.rs` (`StoragePort`), the five backends (`jet`, `sqlite`, `csv`, `xlsx`, `mock`), `crates/datacraft-ui/src/app/grid_state.rs` (row cache), `crates/datacraft-ui/src/datasheet.rs`.
- **Problem:** The datasheet caches the whole table on tab appearance (`app.rows`), so a large table reads every row before it paints — step 101.5's windowing needs a total row count to size the scrollbar, and the port has no such accessor (a `RowRange` beyond the end only surfaces through an `OutOfRange` error).
- **Approach (Medium gate — a port-contract change):** add `fn row_count(&self, table: &str) -> Result<usize, PortError>` with a precise default (`Unsupported`) and a real implementation per backend; then a bounded window cache in the app (`BTreeMap<start, Vec<Row>>` with an LRU cap), window loads on scroll, and the filters/sorts either stay whole-table (documented bound) or move into the port. Find/replace, staging and selection keep addressing cache rows, so the windowed source must stay transparent to them.
- **Acceptance:** a table far larger than the window opens without reading every row; scrolling stays responsive with a bounded cache; filters, sorts, find/replace and staged edits behave identically to the full cache; the wire surfaces stay unchanged (the snapshot never carried row data).
- **Done:** delivered in step 101.5 — the port contract and the bounded window cache are in the tree

---

### [x] M132 — Datasheet completeness: filter-by-value, find/replace, column order, Null marker, windowed reads (phase 101, roadmap §16 stage 1) — 🟡
- **Location:** `crates/datacraft-core/src/table_view.rs` + datasheet state, `crates/datacraft-ui/src/datasheet/*`, port row-window reads.
- **Problem:** Roadmap §16 stage 1 leaves the datasheet short of the user's Excel bar: filters are criteria strings only, there is no find/replace or column reordering, Null cells are blank rather than marked, and every tab caches a full-table read.
- **Approach:** Filter-by-value model and dialog (distinct values, bounded, deterministic order, applied through the existing criteria path); find/replace that uses the 85 edit buffer for replacements; column drag-reorder and a visual Null marker as presentation state; windowed row reads in core behind the port with bounded caches and scroll-follow.
- **Acceptance:** A value list filters the view without typing criteria; find/replace works over the visible rows and stages replacements through the batch; reordering columns persists per tab session; Null cells are visibly distinct in both palettes; a large table opens without reading every row up front and scrolling stays responsive; all four clients still see the same snapshot state.
- **Done:** phase 101 — five steps; windowed reads shipped with the row-count port contract

## ✅ Done Index

> One-liner per completed item (details above).

- M136 — Windowed datasheet reads need a row-count port contract — done (delivered in step 101.5 — the port contract and the bounded window cache are in the tree)
- M132 — Datasheet completeness: filter-by-value, find/replace, column order, Null marker, windowed reads (phase 101, roadmap §16 stage 1) — done (phase 101 — five steps; windowed reads shipped with the row-count port contract)
