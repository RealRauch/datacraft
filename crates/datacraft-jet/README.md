# datacraft-jet

The clean-room pure-Rust Jet 3/4 and ACE backend of the DataCraft engine:
the read path for `.mdb` (Jet 3/Jet 4) and `.accdb` (ACE) files plus the
Jet4/ACE writer (create tables, insert/update/delete rows, alter, drop,
indexes, relationships), behind the core-owned `StoragePort` seam.

Format knowledge comes from public specifications and own black-box
observation only; libmdb (MDB Tools) is a test-time CLI oracle — never
linked or shipped. The crate declares `datacraft-core` with
`default-features = false` and stays C-free by construction (no rusqlite
in its dependency graph).

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

Two surfaces are public and semver-covered:

- The port-facing API: `Jet4Port`, `StorageError`, the `types` vocabulary
- The raw format layer under `datacraft_jet::jet` — readers, writers and
  page/TDEF/row helpers — for consumers that need format-level access
  (and for this crate's integration tests); it follows the same semver
  rules

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-jet](https://docs.rs/datacraft-jet)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
