# BACKLOG_ARCHIVE.p76.md — Shard (Phase 76, append-only)

> Created 2026-11-14T01:59 — home for item blocks archived in the context of Phase 76 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 76. Header is write-once — never edited.

---

### [x] L36 — Split datacraft-expr (piggyback; resolve the types boundary first) — 🟢
- **Location:** `crates/datacraft-core/src/expr/` (lexer, parser, evaluator, funcs, format, like, coerce — 10 files, ~3,100 lines), `crates/datacraft-core/src/schema/` (type boundary).
- **Problem:** The expression engine is a pure, well-tested domain but lives inside core, so every core change recompiles it and it cannot ship its own semver story. Splitting is sanctioned by the crate-split policy (`docs/ROADMAP.md` §17) as the only clean split win.
- **Approach:** Piggyback rule — execute only when a released step touches expr or schema anyway. First resolve the type boundary via a `datacraft-types` crate (NumericKind, field types) so `core → expr → types` stays acyclic, then move expr. New crates = Medium release gates per occurrence; direction review binding; update AGENTS §1, ARCHITECTURE §4, README and the crates.io order (31.4).
- **Acceptance:** expr compiles and tests standalone (`--no-default-features` like the CI matrix), core depends on it without cycles, all expression tests green, canonical verification set green.
- **Steps:** 76.1 (`datacraft-types`) + 76.2 (`datacraft-expr`) — phase 76, scheduled explicitly 2026-11-13 (the piggyback rule is answered by the scheduling decision).
- **Done:** done in phase 76 (67f1efd datacraft-types, e06008d datacraft-expr; phase gate d7d9abc) — expr compiles standalone, core re-exports every path, all expression tests moved unchanged

---

### [x] L37 — Split datacraft-query (optional; make the sql-engine boundary structural) — 🟢
- **Location:** `crates/datacraft-core/src/native_exec/` (3 files, ~1,000 lines), QBE evaluation and saved-query execution paths behind the `sql-engine` feature (`crates/datacraft-core/Cargo.toml`).
- **Problem:** The SQL dialect engine and native query execution are already feature-gated with optional rusqlite, but the optionality is not structural — the boundary exists only as a cargo feature, not as a crate. Second (optional) candidate of the crate-split policy (`docs/ROADMAP.md` §17).
- **Approach:** Piggyback rule — execute only when a released step touches the query/native_exec area anyway. Move the SQL dialect, QBE evaluation and native execution into a `datacraft-query` crate so default core stays rusqlite-free; keep the `sql-engine` feature as the enabling switch. New crate = Medium release gate; direction review binding; update AGENTS §1, ARCHITECTURE §4, README and the crates.io order (31.4).
- **Acceptance:** default-feature core builds without rusqlite; the query crate passes the standalone checks in the CI matrix style; all query/native-execution tests green; canonical verification set green.
- **Steps:** 76.3 (`datacraft-query` default surface) + 76.4 (`sql-engine` feature through the crate) — phase 76, scheduled explicitly 2026-11-13 (the piggyback rule is answered by the scheduling decision).
- **Done:** done in phase 76 (94f0bd4 datacraft-query, 1d92ddd sql-engine feature; phase gate d7d9abc) — default-feature core names no rusqlite (cargo tree proof), query standalone checks green, all query/native-execution tests moved unchanged

## ✅ Done Index

> Einzeiler je abgeschlossenem Item (Details oben).

- L36 — Split datacraft-expr (piggyback; resolve the types boundary first) — done (done in phase 76 (67f1efd datacraft-types, e06008d datacraft-expr; phase gate d7d9abc) — expr compiles standalone, core re-exports every path, all expression tests moved unchanged)
- L37 — Split datacraft-query (optional; make the sql-engine boundary structural) — done (done in phase 76 (94f0bd4 datacraft-query, 1d92ddd sql-engine feature; phase gate d7d9abc) — default-feature core names no rusqlite (cargo tree proof), query standalone checks green, all query/native-execution tests moved unchanged)
