# datacraft-csv

The CSV directory driver of the DataCraft engine — the ADR-7 out-of-tree
proof that a capability-poor data source docks through the core-owned
`StoragePort` seam alone: this crate depends on nothing but
`datacraft-core`.

A *database* is a directory; its tables are the `*.csv` files inside. The
header row names the columns; any write rewrites the whole table file once
(sibling temp file + rename), so a call is atomic — either every operation
of the call lands or none. Column types are inferred deterministically at
open from a full scan (`LongInteger`/`Double`/`DateTime`, everything else
Short Text; one quoted cell pins a column to text); encoding and delimiter
are detected (valid UTF-8, UTF-16 BOM streams, Windows-1252).

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- `CsvPort` implementing the core `StoragePort` contract, with its
  capability declaration and honest refusals for the unsupported paths

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-csv](https://docs.rs/datacraft-csv)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
