# datacraft-sqlite

The SQLite backend of the DataCraft engine behind the core `StoragePort`
seam — `SqlitePort` on rusqlite with a statically bundled SQLite. SQLite
is the second source family next to Jet/ACE: files are opened by magic,
catalogued through `sqlite_master` and mapped onto the same Access domain
vocabulary (the declared-type affinity table is documented in the
repository's `docs/FORMAT-JET.md`, section "SQLite backend").

Supports reads and writes through the port (rows, schema, views, indexes,
relationships with enforced foreign keys), reports file facts and health
(`quick_check`), and answers precise refusals where SQLite semantics
diverge (WAL/locked databases, WITHOUT ROWID and STRICT reordering).

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- `SqlitePort` implementing the core `StoragePort` contract

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-sqlite](https://docs.rs/datacraft-sqlite)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
