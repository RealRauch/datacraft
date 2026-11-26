# BACKLOG_ARCHIVE.p99.md — Shard (Phase 99, append-only)

> Created 2026-11-26T20:06 — home for item blocks archived in the context of Phase 99 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 99. Header is write-once — never edited.

---

### [x] M130 — Export matrix: CSV and JSON table export (phase 99, roadmap §11) — 🟡
- **Location:** `crates/datacraft-core/src/command.rs` + a core export module, `crates/datacraft-ui/src/datasheet.rs`/`nav_*`, CLI/MCP tests.
- **Problem:** The export matrix (roadmap §11) names CSV and JSON as the first two targets, but no export command exists — a table can only leave the application by writing rows through a driver, never as a file.
- **Approach:** One `ExportTable` command naming table, format (CSV, JSON with schema) and target path; deterministic new-file write (fixed key order, no timestamps, existing target refused), source database untouched; UI affordance at the datasheet/nav pane with the save dialog; CLI/MCP parity.
- **Acceptance:** A populated table exports to CSV and to a JSON document carrying schema + rows; re-export is byte-identical; an existing target, an unknown table and an unwritable path answer precisely; the source database stays byte-identical; all four clients pin it.
- **Done:** phase 99 — core f39f35d, ui 465ba93, wire 949e931, docs 03edea4

## ✅ Done Index

> One-liner per completed item (details above).

- M130 — Export matrix: CSV and JSON table export (phase 99, roadmap §11) — done (phase 99 — core f39f35d, ui 465ba93, wire 949e931, docs 03edea4)
