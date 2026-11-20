# datacraft-types

The pure Access domain vocabulary of the DataCraft engine — the type
boundary below `datacraft-core`. It carries the value model (`FieldValue`,
`ComplexRecord`), the logical field kinds (`FieldKind` and its precision
families), table and field definitions, field properties, the schema error
taxonomy, and the pure helpers the expression service and the criteria
parser share (family-safe `compare`, OLE date math, value rendering).

Nothing here knows about commands, ports, state or files — serde is the
only dependency. `datacraft_core::schema` re-exports every public path of
this crate, so clients keep addressing values as
`datacraft_core::schema::FieldValue`.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow
(desktop shell, headless command bus, JSON-RPC/MCP endpoint, CLI).

## Public surface

- Value model: `FieldValue`, `ComplexRecord`
- Schema: `FieldKind` with its precision families, table and field
  definitions, field properties, the schema error taxonomy
- Pure helpers: family-safe `compare`, OLE Automation date math,
  deterministic value rendering

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-types](https://docs.rs/datacraft-types)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
