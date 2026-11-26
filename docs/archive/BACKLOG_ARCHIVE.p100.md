# BACKLOG_ARCHIVE.p100.md — Shard (Phase 100, append-only)

> Created 2026-11-26T20:52 — home for item blocks archived in the context of Phase 100 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 100. Header is write-once — never edited.

---

### [x] F17 — The composite port did not forward the new linked-table accessor — 🟡
- **Location:** `crates/datacraft-storage/src/detecting.rs` (the `StoragePort` delegation block).
- **Problem:** The composite port enumerates every session surface by hand (`indexes`, `triggers`, `native_objects`, `relationship_groups`, …) and the new `linked_tables()` accessor was not among them, so the inventory was empty in the GUI, the CLI and the MCP reply while the jet-level suite was green — the phase-100 wire pins caught it.
- **Fix:** One delegating method added inside step 100.4 (Low, one place, no API decision): `self.session.map_or_else(|| Ok(Vec::new()), |session| session.linked_tables())`.
- **Note for the future:** a new `StoragePort` method with a default implementation needs a matching delegation in the composite, otherwise the client roots silently see the default. The wire pins are the guard; they must keep exercising every session surface.
- **Done:** phase 100.4 — fixed in 5947298

---

### [x] M131 — Remote Stage 0: read-only linked-table inventory (phase 100, roadmap §9) — 🟡
- **Location:** `crates/datacraft-jet/src/jet/catalog.rs` + a link-record decoder, `crates/datacraft-core/src/snapshot.rs`, `crates/datacraft-ui/src/nav_*`.
- **Problem:** Linked tables in Jet/ACE files are silently invisible — the catalog reads only Id/Name/Type/Flags and skips non-TDEF rows, so migration cannot see which objects point at external databases (roadmap §9 Stage 0).
- **Approach:** Decode MSysObjects link records read-only (Type 4 ODBC-linked, Type 6 Access-file-linked; CONNECT, DATABASE, foreign name, flags), carry the inventory in the snapshot, surface it read-only in the UI with localized labels; never dial out, never execute, never write.
- **Acceptance:** A linked-table fixture reports each link's kind, target database/connect string and foreign name; opening the file performs no network or external access; every client sees the inventory; a link-free file reports an empty list; all strings go through t().
- **Done:** phase 100 — jet 46778ea, core c959b50, ui, wire 5947298, docs 0609ca1; three fixtures (two upstream, one self-built)

## ✅ Done Index

> One-liner per completed item (details above).

- F17 — The composite port did not forward the new linked-table accessor — done (phase 100.4 — fixed in 5947298)
- M131 — Remote Stage 0: read-only linked-table inventory (phase 100, roadmap §9) — done (phase 100 — jet 46778ea, core c959b50, ui, wire 5947298, docs 0609ca1; three fixtures (two upstream, one self-built))
