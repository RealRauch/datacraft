# datacraft-import

The declared Access-migration adapter of the DataCraft engine (ADR-9):
migrates native Access form and report designs into `.datacraft` sidecar
designs. It reads a Jet4/ACE database through the pinned, pure-Rust
`jetdb` crate (no COM/FFI), maps the supported control subset onto the
core sidecar vocabulary and reports every skipped control or property.

This is the only crate of the family that depends on a third-party
reverse-engineering crate; the adapter boundary is recorded in
`THIRD-PARTY-NOTICES.md`, the mapping contract in `docs/M88-IMPORT.md`.
The engine itself stays clean-room — nothing of `jetdb` is linked into
`datacraft-core` or the shipped backends.

Binary and library in one crate:

```console
cargo install datacraft-import
datacraft-import <database> [--sidecar <path>] [--strict] [--json]
```

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- The `datacraft-import` binary (CLI over the mapping below)
- Library: `import_database`, `merge_documents`, `has_unsupported`,
  the `Import`/`ImportReport`/`ImportError` vocabulary

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-import](https://docs.rs/datacraft-import)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
