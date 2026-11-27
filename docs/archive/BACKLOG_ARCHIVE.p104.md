# BACKLOG_ARCHIVE.p104.md — Shard (Phase 104, append-only)

> Created 2026-11-27T21:47 — home for item blocks archived in the context of Phase 104 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 104. Header is write-once — never edited.

---

### [x] M135 — Decimal display ignores the declared scale — the datasheet shows the scaled integer — 🟡
- **Location:** `crates/datacraft-types/src/display.rs` (`display_value`, the `Decimal` arm), `crates/datacraft-csv/src/lib.rs` (`to_cell`), every display consumer (datasheet cells, status line, form runtime).
- **Problem:** A fixed-point decimal is a scale-scaled integer (field kind `Number(Decimal { precision, scale })`), but `display_value` prints the raw scaled integer: a value of 25.50 at scale 2 shows as `2550` in the datasheet and in the CSV driver's cells. Phase 99's export module works around it with its own exact decimal text (`export::decimal_text`), so the workaround exists twice-ish and the datasheet still lies.
- **Approach:** Make the decimal rendering scale-aware where the field kind is in reach — a kind-aware sibling of `display_value` (or an extra parameter) that divides by 10^scale and keeps the declared trailing zeros; scale 0 stays byte-identical to today. Then let the export module delegate instead of carrying its own copy. `row_edit` parses through `parse_scaled`, so the write path is unaffected.
- **Acceptance:** A scale-2 decimal shows `25.50` in the datasheet, the CSV driver's rewrite and the CSV/JSON export; scale-0 decimals render unchanged; a test pins each path and the export's own helper either disappears or delegates.
- **Done:** completed in step 104.1 (RED `197e7fd`, feat `be9d190`) — display_value_for is the kind-aware helper in datacraft-types; the export workaround delegates; datasheet, windowed body, form runtime, value viewer, batch preview, CSV rewrite and the filter-by-value pair share the rule; combo row-source values and computed expressions stay raw (no column kind in reach)

---

### [x] M137 — Jet write path ignores declared relationships — orphan inserts/updates succeed where SQLite refuses (roadmap §13a) — 🟡
- **Location:** `crates/datacraft-jet/src/` (the port write path: insert/update and the journal batch application), `crates/datacraft-core/src/journal.rs` (preview/apply gating), `crates/datacraft-sqlite/src/` (the enforced reference behavior).
- **Problem:** Roadmap §13(a) freezes the minimum write-parity ladder: SQLite enforces declared foreign keys (`PRAGMA foreign_keys=ON` — orphans refused) while the Jet path writes `MSysRelationships` metadata only, so an orphan insert/update succeeds on Jet and fails on SQLite — identical input, different visible outcome, breaking the parity principle (AGENTS §6: identical state transitions across backends).
- **Approach:** The Jet write path checks every inserted/updated row of a child table against the declared relationships' parent unique indexes and answers a precise, locatable error naming the relationship, the parent table and the offending value image; journal batches refuse in preview (the batch never applies partially). Delete/restore paths keep today's behavior (§13(b) cascades are a separate, later step); SQLite semantics stay as they are.
- **Acceptance:** "Same input → same visible outcome on both backends": an orphan insert and an orphan update answer the same refusal class on Jet and SQLite with locatable reasons; a valid child row still writes; a batch containing an orphan is refused in preview with the precise reason; contract tests pin both backends over the shared port contract.
- **Done:** completed in step 104.2 (RED `c3e927d`, feat `b470083`) — the rule lives in datacraft_core::referential; the Jet writer enforces it on insert/update/restore/batch, the journal preview refuses an orphan batch; the cross-driver port contract pins both refusals. Documented boundaries: composite keys enforce the first declared pair, deletes stay allowed (§13b), the parent read is a full-table projection per write on relationship-holding tables

## ✅ Done Index

> One-liner per completed item (details above).

- M135 — Decimal display ignores the declared scale — the datasheet shows the scaled integer — done (completed in step 104.1 (RED `197e7fd`, feat `be9d190`) — display_value_for is the kind-aware helper in datacraft-types; the export workaround delegates; datasheet, windowed body, form runtime, value viewer, batch preview, CSV rewrite and the filter-by-value pair share the rule; combo row-source values and computed expressions stay raw (no column kind in reach))
- M137 — Jet write path ignores declared relationships — orphan inserts/updates succeed where SQLite refuses (roadmap §13a) — done (completed in step 104.2 (RED `c3e927d`, feat `b470083`) — the rule lives in datacraft_core::referential; the Jet writer enforces it on insert/update/restore/batch, the journal preview refuses an orphan batch; the cross-driver port contract pins both refusals. Documented boundaries: composite keys enforce the first declared pair, deletes stay allowed (§13b), the parent read is a full-table projection per write on relationship-holding tables)
