# DataCraft

An open-source, clean-room reimplementation of the Microsoft Access database
experience — bit-for-bit compatible file formats, cross-platform, memory-safe.

DataCraft follows the ArtCraft ecosystem philosophy: lightweight, ultra-fast,
written in idiomatic Rust, and designed to be operated by humans **and** AI
agents (MCP / headless JSON-RPC).

> **Status:** Phase 1 (Blueprint & Skeleton) in progress — see [BACKLOG.md](BACKLOG.md)
> and [PROGRESS.md](PROGRESS.md).

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
| Formatting & lints | `rustfmt --check`, `clippy -D warnings` |
| Cross-platform tests | Matrix: ubuntu-latest, windows-latest, macos-latest |

## Storage engine decision (2026-10-31)

The Jet/ACE read **and** write paths are implemented **in pure Rust**
(clean-room, from public format documentation). `libmdb` (MDB Tools) is used
**only as a test-time oracle in CI** to cross-check our parser — it is not a
runtime dependency. Rationale: libmdb cannot write Access-compatible files,
so the bit-for-bit write goal requires our own implementation anyway.

## License

- DataCraft code: MIT OR Apache-2.0 (to be finalized with the skeleton)
- MDB Tools (`libmdb`): LGPL-2.1 — used exclusively at test time, never linked
  into release builds.
