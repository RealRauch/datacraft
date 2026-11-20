# datacraft-storage

The format-detecting driver composite of the DataCraft engine: a registry
that sniffs a file's format (Jet 3/Jet 4, ACE, SQLite, CSV directory) and
composes the matching shipped backend behind the core `StoragePort` seam —
plus the deterministic sample-database generator behind the product's
shipped samples.

This crate is the convenience composition for consumers that want the
shipped backends bundled: the composition roots (`datacraft-ui`,
`datacraft-cli`, `datacraft-mcp`) use it instead of naming concrete
drivers. Out-of-tree driver crates do not need it — they depend only on
`datacraft-core`.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- `detecting` — `DetectingPort`, the format-detecting registry with
  fallback composition and the unknown-file family hint
- `samples` — the deterministic sample-database generator

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-storage](https://docs.rs/datacraft-storage)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
