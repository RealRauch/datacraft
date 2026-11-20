# datacraft-core

The headless engine of DataCraft: domain model, command bus and application
state. Every user-visible action is exactly one `Command` dispatched through
the bus; every observable outcome is queryable as serializable state. The
desktop GUI, the CLI and the MCP endpoint are thin clients of this crate —
a ribbon click and an MCP call produce identical state transitions.

It carries the `StoragePort` seam (the single storage boundary every driver
implements), the driver capability vocabulary, the journal/undo history and
the JSON-RPC transport codec behind the headless clients, and re-exports
the type (`schema`), expression and query paths of the lower crates.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- Command bus: `Command`, `Event`, `Core` dispatch, machine-readable
  failure codes beside the English reasons
- State: `UiStateSnapshot`/`DatabaseSnapshot` (read-only snapshots)
- The `StoragePort` seam with `DriverCapabilities`/`Support` and
  `MockPort`
- JSON-RPC transport codec (the seam behind CLI and MCP)
- Re-exports: `datacraft_core::schema` (= `datacraft-types`), the
  expression service and the query paths

## Features

- `sql-engine` (default: **off**) — forwards to
  `datacraft-query/sql-engine`. A default-feature core stays C-free;
  the client roots enable it explicitly.

## Testing surface

`datacraft_core::port::MockPort` is public on purpose — out-of-crate
clients drive `Core` over the command bus in their own tests without a
database file. Its behaviour is part of the supported API (the same
contract the real drivers implement where they overlap); it is not a
production backend.

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-core](https://docs.rs/datacraft-core)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
