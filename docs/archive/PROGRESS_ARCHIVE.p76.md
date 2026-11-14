# PROGRESS_ARCHIVE.p76.md — Shard (Phase 76, append-only)

> Created 2026-11-14T01:59 — home for the detail block of Phase 76 on completion.
> Contains: Phase 76 — Type, expression and query crates. Header is write-once — never edited.

---

### Phase 76 — Type, expression and query crates

**Scope (Steps):**

- **76.1 datacraft-types: move the value vocabulary and its pure helpers below core (L36)**
- **76.2 datacraft-expr: extract the expression engine onto the types crate (L36)**
- **76.3 datacraft-query: extract the QBE model, evaluation and native execution (L37)**
- **76.4 sql-engine: move the shadow SQL dialect behind the query crate's feature (L37)**
- **76.5 Documentation and phase gate (L36, L37)**

**Goal:** Answer the two crate-split candidates of the policy
(`docs/ROADMAP.md` §17) as one dependency chain, bottom-up. The human
scheduled them explicitly on 2026-11-13 — the items' piggyback rule
("only when a released step touches the area anyway") is thereby
answered: this phase *is* the touching step.

The target graph is `core → query → expr → types` (core keeps its direct
`expr` edge for the public re-export), with no cycle and no new
optionality in the default build:

- **76.1 — `datacraft-types`.** The value vocabulary leaves
  `schema/value.rs`: `FieldValue` and `ComplexRecord`, plus the pure
  helpers `expr` and the criteria parser share (`parse_date`,
  `days_from_civil`, `compare` — today `pub(crate)` in
  `query_criteria.rs`). The definitions move unchanged;
  `datacraft_core::schema` re-exports every public path, so
  `datacraft_core::schema::FieldValue` keeps resolving for all four
  clients. This resolves L36's stated prerequisite (the type boundary)
  before any expression code moves.
- **76.2 — `datacraft-expr`.** `crates/datacraft-core/src/expr/` (ten
  files, ~3,100 lines) becomes the crate; core keeps `pub use
  datacraft_expr as expr`, so `datacraft_core::expr::*` stays the
  public path. The expression tests move with their modules unchanged —
  moving is not deleting (no High gate). The crate is pure Rust with
  `datacraft-types` as its only datacraft dependency.
- **76.3 — `datacraft-query` (default surface).** The QBE model
  (`qbe.rs`, pure serde), the criteria parser (`query_criteria`), the
  QBE evaluation (`query_engine`), native execution (`native_exec/`)
  and the native-query port vocabulary (`port/native_query.rs`, pure
  serde) move into the crate; core re-exports the public paths (`qbe`,
  `query_engine`, `native_exec`, `port::{NativeQuery,
  NativeQueryColumn, NativeQueryKind, NativeJoin, NativeJoinKind}`).
  This surface stays rusqlite-free and is a plain dependency of core,
  so `default-features = false` consumers (jet/ACE) keep their C-free
  graph.
- **76.4 — the `sql-engine` feature moves with the dialect.** The
  shadow SQLite engine (`sql_engine.rs`, the only rusqlite consumer)
  and its `sql_types` vocabulary join the query crate behind that
  crate's own `sql-engine` feature; core's `sql-engine` feature becomes
  the forwarding switch (`sql-engine = ["datacraft-query/sql-engine"]`).
  Default core builds without rusqlite — the L37 acceptance.
- **76.5 — documentation and gate.** AGENTS §1 crate graph,
  ARCHITECTURE §4/§6, README crate table and `docs/PUBLISHING.md`'s
  publish order (types, expr, query strictly before core), CHANGELOG,
  and the release packaging of the three new published crates.

Content classes: code moves are Low; every new workspace member,
`Cargo.toml` dependency edge and feature rewrite is **Medium** and
pauses for the human release per occurrence, even after the phase
release. No fixture, workflow or wire shape changes; no test is
deleted, only moved. The dependency-boundary guard needs no script
change — it reads the manifests, and the new graph stays acyclic with
the binary roots unchanged.

**Acceptance:** L36: `datacraft-expr` compiles and tests standalone in
the CI matrix style (`--no-default-features`), core depends on it
without cycles, all expression tests pass unchanged, and
`datacraft_core::expr::*` paths keep resolving. L37: a default-feature
core builds without rusqlite in its graph, the query crate passes the
standalone checks, all query/native-execution tests pass unchanged, and
`datacraft_core::{qbe,query_engine,native_exec,sql_types}` plus the
native-query port re-exports keep resolving. Phase-wide: the
dependency-boundary guard, fmt, clippy `-D warnings`, workspace tests,
lcov ≥ 95 % and the file-size gate are green; `cargo package --list`
is clean for the three new crates (licenses, NOTICE, README,
repository); PUBLISHING's order and the lockstep version cover them;
`docs_validate` clean and CI green.

**Verification:** Canonical set (fmt, clippy `-D warnings`, workspace
tests, lcov ≥ 95 %), file-size gate, `docs_validate` clean, CI green;
plus the standalone builds `cargo build -p datacraft-expr
--no-default-features` and `cargo build -p datacraft-query
--no-default-features` and the rusqlite-free proof
(`cargo tree -p datacraft-core --no-default-features` names no
rusqlite).
**Verification:** Canonical set green (fmt, clippy -D warnings, workspace tests, lcov 95.575% = 18940/19817), file-size gate clean, rustdoc -D warnings green, boundary guard 11 crates acyclic, docs_validate clean; AGENTS/ARCHITECTURE/README/PUBLISHING/TESTING/ROADMAP/CHANGELOG updated; package --list clean for the three new crates.
