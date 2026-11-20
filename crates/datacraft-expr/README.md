# datacraft-expr

The Access-style expression engine of the DataCraft engine: a small, total
evaluator for field references, literals, arithmetic and concatenation,
comparison and three-valued logic with Jet NULL semantics, the `IIf`/`Nz`/
`Format` family, date and string functions, and `Like` with Access
wildcards.

The documented subset and its decisions (precedence, coercion, banker's
rounding, `True = -1`, case fences) are spelled out in the crate docs. The
evaluator is deterministic by design — `Now`/`Date` are absent on purpose.

Sits directly on `datacraft-types`; `datacraft-core` re-exports it as the
expression service behind default values, validation rules and calculated
fields.

Part of [DataCraft](https://github.com/RealRauch/datacraft) — a clean-room,
pure-Rust reimplementation of the classic Microsoft Access workflow.

## Public surface

- Lexer, parser and evaluator for the Access-style expression subset
- The `IIf`/`Nz`/`Format` function family with deterministic named
  date/time pictures
- `Like` matching with Access wildcards (case-insensitive)
- Canonical expression formatting

## Status

- **MSRV:** Rust 1.98 (the workspace `rust-version`)
- **License:** MIT OR Apache-2.0
- **Versioning:** lockstep with the DataCraft family (currently 0.2.0);
  publish order and semver policy in
  [docs/PUBLISHING.md](https://github.com/RealRauch/datacraft/blob/main/docs/PUBLISHING.md)
- **Docs:** [docs.rs/datacraft-expr](https://docs.rs/datacraft-expr)
- **Product page:** [github.com/RealRauch/datacraft](https://github.com/RealRauch/datacraft)
