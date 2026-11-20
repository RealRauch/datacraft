# datacraft-query

The query layer of the DataCraft engine: the QBE document model (serde),
the typed criteria parser, the QBE evaluation, native saved-query execution
and the native-query port vocabulary.

Depends only on `datacraft-expr` and `datacraft-types`; the command bus
re-exports these modules, so clients keep addressing
`datacraft_core::{qbe, query_engine, native_exec, query_criteria}` and the
native-query port paths.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- QBE model (serde) and evaluation
- Typed Access-criteria parser (`query_criteria`) with a documented
  supported/unsupported fence
- Native saved-query execution (equality joins, WHERE expressions,
  `Table.*` projection, ORDER BY)
- Native-query port vocabulary

## Features

- `sql-engine` (default: **off**) — the shadow SQL engine (one
  SELECT/WITH over materialized sources) built on a throwaway in-memory
  SQLite database. This is the only C-backed surface of the query layer:
  a default-feature build never pulls rusqlite. `datacraft-core` forwards
  its own `sql-engine` switch to this feature.

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-query](https://docs.rs/datacraft-query)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
