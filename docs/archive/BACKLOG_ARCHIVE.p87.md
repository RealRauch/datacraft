# BACKLOG_ARCHIVE.p87.md — Shard (Phase 87, append-only)

> Created 2026-11-16T00:12 — home for item blocks archived in the context of Phase 87 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 87. Header is write-once — never edited.

---

### [x] M103 — No foreign-key navigation from a datasheet row — 🟡
- **Location:** relationships are modelled and written (M16: `MSysRelationships`, SQLite foreign keys); the datasheet has no navigation from a key value to its target.
- **Problem:** Users cannot follow a foreign key from a row to the referenced record, return to where they came from, or see which rows reference the current one. Access's subdatasheet/relationship navigation and the reference client's F4-follow both exist; ours stops at showing the relationship canvas.
- **Approach:** An F4/follow affordance on key cells: declared foreign keys first, then reverse keys, then inferred `<table>_id` references derived from loaded schema metadata (never from a network call). Targets open filtered to the matching values. Composite keys stay paired per source row (no cross product); NULL keys answer a precise message. A per-tab back/forward history returns to the previous table and filter; a tab with pending edits is activated, not replaced. "Referenced by" counts run on a separate read with a cap and a per-table timeout, cancellable.
- **Acceptance:** Following a single and a composite key filters the target correctly; a NULL key answers the documented message; back/forward restores table and filter, including a closed tab; an all-NULL or partially selected composite selection never matches a wrong pairing; counts are capped and time-boxed and their truncation is visible; the feature is hidden where the driver declares no relationship support; behaviour doc with edge cases.
- **Steps:** none yet.
- **Done:** steps 87.1/87.2 — follow affordance, per-tab history and capped referenced-by counts (commits 582810a, 65201d3, 3df0e87, cf903be, 4f2a99d, 57d8cd0, e34f4b3, c8f48f8, 1e862ca, 21c29c2, 05ceed4)

---

### [x] M104 — Datasheet filtering is a criteria string — no structured AND/OR builder — 🟡
- **Location:** the datasheet filter dialog and criteria string (M55/M62, wildcard rule H58); the QBE grid already models AND/OR rows for queries.
- **Problem:** A table filter is one criteria string per column (or the documented syntax), with no grouped AND/OR precedence, no ordered sort list and no draft/apply distinction. Access's Filter by Form and the reference client's builder both offer structured conditions with visible grouping.
- **Approach:** A builder model in core (conditions with AND/OR, nested groups with visible parentheses, ordered sort columns, a text mode as escape hatch) that serializes to the existing criteria input, so the execution path and the wildcard rule stay unchanged. Editing a draft never runs a query; Apply reloads, Clear resets. Reuse the QBE row vocabulary where it fits.
- **Acceptance:** Builder to criteria round-trips for every supported condition; grouped precedence is visibly parenthesized and behaves accordingly; invalid drafts cannot run; a filter survives sort/filter commits and the FK navigation of M103; the text mode still accepts the documented syntax; behaviour doc with the dialect fence.
- **Steps:** none yet.
- **Done:** steps 87.3/87.4 — filter builder model, window with draft/apply, text-mode fence for the bare wildcard (commits f837454, 959099d, 3c872a1, 1fabe59, 587cf7b, d155b40, 6b3a6d6, 1e72e03)

---

### [x] F14 — Inline fix: broken intra-doc links reached CI because the local set lacked the rustdoc gate — 🟢
- **Location:** `crates/datacraft-core/src/fk_nav.rs` and `filter_builder.rs` doc comments; `AGENTS.md` §4 canonical verification set.
- **Problem:** The phase 87 pushes failed the CI rustdoc gate ("unresolved link to `NullKey`", "links to private item `condition_criteria`") while every local gate was green — the canonical set in AGENTS §4 lacked the rustdoc command that CI enforces (step 59.2, L33), so the failure only surfaced at the phase boundary.
- **Fix:** the enum-variant links and the private link are repaired in `883e341`; the rustdoc command joined the AGENTS §4 set in the same commit.
- **Done:** fixed in 883e341 (doc links + AGENTS §4 rustdoc gate)

## ✅ Done Index

> One-liner per completed item (details above).

- M103 — No foreign-key navigation from a datasheet row — done (steps 87.1/87.2 — follow affordance, per-tab history and capped referenced-by counts (commits 582810a, 65201d3, 3df0e87, cf903be, 4f2a99d, 57d8cd0, e34f4b3, c8f48f8, 1e862ca, 21c29c2, 05ceed4))
- M104 — Datasheet filtering is a criteria string — no structured AND/OR builder — done (steps 87.3/87.4 — filter builder model, window with draft/apply, text-mode fence for the bare wildcard (commits f837454, 959099d, 3c872a1, 1fabe59, 587cf7b, d155b40, 6b3a6d6, 1e72e03))
- F14 — Inline fix: broken intra-doc links reached CI because the local set lacked the rustdoc gate — done (fixed in 883e341 (doc links + AGENTS §4 rustdoc gate))
