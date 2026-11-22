# BACKLOG_ARCHIVE.p98.md — Shard (Phase 98, append-only)

> Created 2026-11-22T03:40 — home for item blocks archived in the context of Phase 98 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 98. Header is write-once — never edited.

---

### [x] M126 — Compact & Repair: pre-open health check and the compact command (phase 95, roadmap §10) — 🟡
- **Location:** `crates/datacraft-jet/src/port_adapter.rs` (open/facts), `crates/datacraft-core/src/command.rs` + `core.rs` (command), `crates/datacraft-ui/src/backstage/info.rs` (surface).
- **Problem:** No maintenance track exists: no pre-open health facts for Jet/ACE, no Compact command, no capability-gated UI affordance and no wire pins — while the verified Jet4/ACE writer already supports the compact recipe (read everything, write a fresh verified file, atomic swap).
- **Approach:** Jet/ACE health facts and bounded page/region sanity at open (read-only, non-blocking, precise damage reason); a Compact command that reads the whole database, writes a fresh verified Jet4/ACE file, swaps it atomically at the same path and reloads the session; UI Info surface with facts, health and a capability-gated Compact action; CLI/MCP contract pins. Salvage stays a later phase.
- **Acceptance:** A healthy fixture reports no health finding and full facts; a damaged page reports a locatable health reason without blocking the open; compacting a populated Jet4/ACE fixture produces a reopened, byte-verified, usable database at the same path with the sidecar still resolved; SQLite/CSV refuse precisely; all four clients pin the behavior.
- **Done:** phase 95 (c78c7be, fb206c3, d8f84cc, 0d8d81d)

---

### [x] M127 — Excel read driver .xlsx/.xlsm/.xls/.xlsb (phase 96, roadmap §7) — 🟡
- **Location:** new crate `crates/datacraft-xlsx`, `crates/datacraft-core/src/port.rs` (DatabaseFormat), `crates/datacraft-storage/src/detecting.rs` (registry), composition roots, `crates/datacraft-core/src/import.rs` (import seam).
- **Problem:** Excel workbooks are not a source family: no driver crate, no format variant, no registry entry and no import source, although roadmap §7 froze the read-only scope (calamine, sheets as tables, values only, macros never read).
- **Approach:** ADR-7 driver crate over calamine with a core-only dependency; one `DatabaseFormat` variant; sheets as tables with typed cells; multi-driver fallback registry; read-only capability honesty in nav pane/datasheet; xlsx as an Import File source through a core-owned reader seam; fixtures built with the Excel COM builder and pinned with provenance.
- **Acceptance:** .xlsx/.xlsm/.xls/.xlsb fixtures open through the composed binaries, list their sheets as tables and read typed values through the snapshot; every write affordance is disabled with the precise reason; a workbook imports as a table into an open Jet4/SQLite database; the fixture manifest and FIXTURE-SOURCES carry the new pins.
- **Done:** phase 96 (756aa9b, 7d1aa37, 62b3d4a)

---

### [x] M128 — PDF report export over the frozen scope (phase 97, roadmap §11) — 🟡
- **Location:** `crates/datacraft-core/src/report_preview.rs`/`report_bands.rs` (page model), new optional-core-feature writer, `crates/datacraft-ui/src/report_design.rs` (affordance), CLI/MCP tests.
- **Problem:** The report preview paginates in core, but its output cannot leave the application: no PDF export command, no deterministic renderer, no UI or wire surface, although roadmap §11 froze the scope.
- **Approach:** A pure-Rust PDF writer behind an optional core feature (sql-engine precedent), rendering exactly the preview's page sequence from the shared page model; fixed metadata and stable order for byte-identical re-export; Inter embedded from `assets/fonts` without a UI dependency; one export command, a capability-gated preview affordance and CLI/MCP parity.
- **Acceptance:** An exported fixture report shows the preview's page count and breaks in a standard viewer, its text extracts as text (not raster), re-exporting unchanged input is byte-identical, an unwritable target answers a precise error, and the page model is pinned against the preview as state.
- **Done:** phase 97 (cd71681)

---

### [x] M129 — SQLite SQL pushdown, stage 1 (phase 98, roadmap §19) — 🟡
- **Location:** `crates/datacraft-sqlite/src` (native execution), `crates/datacraft-core/src/core.rs` (`run_sql`), `crates/datacraft-ui/src/sql_window.rs`, CLI/MCP tests.
- **Problem:** The SQL window always materializes sources into the shadow engine, so SQLite's full dialect, native indexes, native views and `EXPLAIN QUERY PLAN` are unreachable; roadmap §19 stage 1 freezes read-only pushdown.
- **Approach:** A read-only native-SQL port seam implemented by SQLite (prepare/query, typed results, views, `EXPLAIN QUERY PLAN`, writes refused with a precise reason); core dispatch selects native vs shadow per source; the SQL window names the dialect and renders the plan; CLI/MCP pins; Jet/CSV stay on the shadow engine unchanged.
- **Acceptance:** Native SQLite statements that the shadow engine cannot run (full dialect, index use, native views, EXPLAIN) execute read-only through the bus and all four clients; any write statement is refused precisely on the bus path; Jet/CSV results are byte-for-byte unchanged; the dialect is named in the snapshot/UI.
- **Done:** phase 98 (8c26031)

## ✅ Done Index

> One-liner per completed item (details above).

- M126 — Compact & Repair: pre-open health check and the compact command (phase 95, roadmap §10) — done (phase 95 (c78c7be, fb206c3, d8f84cc, 0d8d81d))
- M127 — Excel read driver .xlsx/.xlsm/.xls/.xlsb (phase 96, roadmap §7) — done (phase 96 (756aa9b, 7d1aa37, 62b3d4a))
- M128 — PDF report export over the frozen scope (phase 97, roadmap §11) — done (phase 97 (cd71681))
- M129 — SQLite SQL pushdown, stage 1 (phase 98, roadmap §19) — done (phase 98 (8c26031))
