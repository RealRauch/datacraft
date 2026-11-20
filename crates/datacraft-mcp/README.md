# datacraft-mcp

The headless stdio JSON-RPC 2.0 endpoint of the DataCraft command bus — a
standalone binary crate. It speaks the same `Command` vocabulary as the
desktop GUI and the CLI: every user-visible action is one command, every
observable outcome is queryable as serializable state.

Install and run (JSON-RPC 2.0 over stdin/stdout, no socket):

```console
cargo install datacraft-mcp
datacraft-mcp
```

The binary composes the full shipped stack — Jet 3/4 and ACE, SQLite and
the CSV directory driver through `datacraft-storage`, with the SQL engine
enabled.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- The `datacraft-mcp` binary (stdio JSON-RPC 2.0 server)

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
