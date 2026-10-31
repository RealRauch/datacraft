# DataCraft

An open-source, clean-room reimplementation of the Microsoft Access database
experience — bit-for-bit compatible file formats, cross-platform, memory-safe.

DataCraft follows the ArtCraft ecosystem philosophy: lightweight, ultra-fast,
written in idiomatic Rust, and designed to be operated by humans **and** AI
agents (MCP / headless JSON-RPC).

> **Status:** Phase 1 (Blueprint & Skeleton) complete — Phase 2 (Storage Engine
> Core) is next. See [BACKLOG.md](BACKLOG.md) and [PROGRESS.md](PROGRESS.md).

## Quick start

Requires the Rust toolchain pinned in `rust-toolchain.toml` (rustup installs it
automatically on build).

```bash
cargo run -p datacraft-ui        # opens the Phase 1 shell window
cargo test --workspace           # runs the test suite
cargo llvm-cov --workspace --exclude datacraft-ui \
  --lcov --output-path lcov.info --fail-under-lines 95   # coverage gate
```

Crate layout: `crates/datacraft-core` (headless domain/commands),
`crates/datacraft-storage` (Jet/ACE engine, Phase 2),
`crates/datacraft-ui` (eframe desktop shell). Architecture:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Guiding principles

1. **Clean-room reverse engineering** — no proprietary Microsoft code, no
   decompiled artifacts. Behavior is rebuilt from public specifications and
   observable behavior only.
2. **Visual originality** — no Microsoft assets. Original, minimal iconography;
   the Access UX paradigm rendered in the ArtCraft aesthetic.
3. **MCP & headless automation** — every GUI action is backed by a decoupled
   command API; an MCP/JSON-RPC endpoint lets LLM agents drive the tool.
4. **Autonomous testability** — deterministic test IDs on every UI element;
   state is queryable as data, never via pixels.
5. **Strict cross-platform portability** — Linux, macOS, Windows; no COM,
   OLE DB, ADO, or winapi anywhere.

## Quality gates (mechanically enforced in CI)

| Gate | Enforcement |
|------|-------------|
| Test coverage > 95 % | `cargo-llvm-cov --fail-under-lines 95` |
| Source files < 500 lines | CI job fails on any `.rs` file ≥ 500 lines |
| **No technical debt** | CI scan: no `TODO`/`FIXME`/`HACK`/`XXX` in tracked files |
| Formatting & lints | `rustfmt --check`, `clippy -D warnings` |
| Cross-platform tests | Matrix: ubuntu-latest, windows-latest, macos-latest |

## Storage engine decision (2026-10-31)

The Jet/ACE read **and** write paths are implemented **in pure Rust**
(clean-room, from public format documentation). `libmdb` (MDB Tools) is used
**only as a test-time oracle in CI** to cross-check our parser — it is not a
runtime dependency. Rationale: libmdb cannot write Access-compatible files,
so the bit-for-bit write goal requires our own implementation anyway.

## Branding

The app icon follows the ArtCraft Crafting Apps icon system: a 512-unit rounded tile,
**exactly three colours** (ink `#0B0B0C`, paper `#EFE9DC`, DataCraft teal `#0F8B7E`) and
an engraved creature portrait. The current file is a suite-consistent **placeholder**
(database cylinder + datasheet); the final engraved-beaver artwork, its composition
spec and a copy-paste image-generation prompt are in
[`docs/brand/DATACRAFT-ICON.md`](docs/brand/DATACRAFT-ICON.md). Renders and the Windows
`.ico` live in [`assets/app-icon/`](assets/app-icon/).

## License

- DataCraft code: MIT OR Apache-2.0 (to be finalized with the skeleton)
- MDB Tools (`libmdb`): LGPL-2.1 — used exclusively at test time, never linked
  into release builds.
