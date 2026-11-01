<div align="center">

<img src="assets/app-icon/datacraft-256.png" width="140" alt="DataCraft icon: a database cylinder whose front is a datasheet">

**Databases and the classic Access workflow; an open-source, clean-room
reimplementation of the Microsoft Access experience, built in pure Rust.**

A fast, open-source database tool with a familiar workflow: the ribbon,
navigation pane, tabbed documents, datasheet and design views, the QBE query
builder and a form/report canvas. It reads and writes `.mdb` (Jet) and
`.accdb` (ACE) bit-for-bit, runs natively on macOS, Windows and Linux, and is
designed to be operated by humans **and** AI agents through the same command
API.

_By the ArtCraft team._

[![Written in Rust](https://img.shields.io/badge/written%20in-Rust-0F8B7E?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Runs on macOS, Windows and Linux](https://img.shields.io/badge/runs%20on-macOS%20%7C%20Windows%20%7C%20Linux-0F8B7E?style=flat-square)](#)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-0F8B7E?style=flat-square)](#license-and-credits)
[![CI](https://img.shields.io/github/actions/workflow/status/RealRauch/datacraft/ci.yml?style=flat-square&label=CI)](https://github.com/RealRauch/datacraft/actions/workflows/ci.yml)

**Why DataCraft** ·
[What works today](#what-works-today) ·
[Quick start](#quick-start) ·
[Architecture](#architecture) ·
[Roadmap](#roadmap) ·
[Quality gates](#quality-gates) ·
[Design](#design--branding) ·
[License and credits](#license-and-credits)

</div>

> [!NOTE]
> **DataCraft is in early development.** Phases 1–2 are complete: the
> pure-Rust storage engine core reads Jet4 `.mdb` files and is cross-checked
> against the libmdb oracle in CI. Expect breaking changes. The honest,
> step-by-step picture lives in [BACKLOG.md](BACKLOG.md) and
> [PROGRESS.md](PROGRESS.md).

## Why DataCraft

- **Familiar.** The Access workflow reimagined: ribbon, navigation pane,
  tabbed documents, datasheet and design views, the QBE grid, form and report
  canvas — rendered in an original, minimal ArtCraft aesthetic.
- **Your files.** Bit-for-bit compatible reading and writing of `.mdb` and
  `.accdb`, in pure Rust. No COM, OLE DB, ADO or winapi anywhere.
- **Clean-room.** Format knowledge comes only from public specifications and
  our own black-box observation. MDB Tools (`libmdb`) serves as a test-time
  oracle in CI and is never linked into release builds.
- **Built for agents.** Every GUI action is exactly one command on a decoupled
  command bus; the UI is never privileged. An MCP/JSON-RPC endpoint lets LLM
  agents drive the same tool headlessly (Phase 9).
- **Fast.** egui on the GPU, virtualized painting of visible grid cells only,
  no per-frame allocations on the hot path — 60 FPS with million-row
  scrolling as the design target.
- **Verifiable.** Deterministic test IDs on every UI element, serializable
  state snapshots, property-based round-trips and golden files — headless
  tests assert on data, never on pixels.

## What works today

| Area | Highlights |
| --- | --- |
| **Storage — Jet4 read** | `StorageEngine` trait boundary; Jet page layer and file-header decoder; table definition (TDEF) and data (TDAT) decoders; golden-file tests against committed fixtures; every parse cross-checked against the libmdb oracle in CI |
| **Desktop shell** | Minimal eframe/egui application window — the Phase 1 skeleton that the ribbon and workspace grow into |
| **CI quality gates** | Coverage ≥ 95 % lines, every `.rs` file < 500 lines, zero `TODO`/`FIXME`/`HACK`/`XXX`, `rustfmt --check`, `clippy -D warnings`, test matrix on ubuntu-latest, windows-latest, macos-latest |
| **Docs & process** | Architecture blueprint, Jet/ACE binary-format knowledge base, testing policy and the clean-room playbook |

The honest picture, phase by phase, is in
[docs/ROADMAP.md](docs/ROADMAP.md); the format facts are catalogued in
[docs/FORMAT-JET.md](docs/FORMAT-JET.md).

## Quick start

Requires the Rust toolchain pinned in `rust-toolchain.toml` (rustup installs
it automatically on build).

```bash
git clone https://github.com/RealRauch/datacraft
cd datacraft
cargo run -p datacraft-ui        # the desktop shell window (Phase 1 skeleton)
cargo test --workspace           # unit, integration, proptest and golden-file tests
```

The libmdb oracle cross-check additionally runs in CI (ubuntu and macOS, with
`mdbtools` installed there); golden-file tests run everywhere.

```bash
cargo llvm-cov --workspace --exclude datacraft-ui \
  --lcov --output-path lcov.info --fail-under-lines 95   # the coverage gate, as CI runs it
```

## Architecture

| Layer | Crate | Job |
| --- | --- | --- |
| core | `datacraft-core` | domain model, command bus, application state, UI-state snapshots, deterministic test IDs (Phases 3–4) |
| storage | `datacraft-storage` | `StorageEngine` trait, pure-Rust Jet/ACE backends, libmdb oracle (`cfg(test)` only) (Phase 2+) |
| ui | `datacraft-ui` | eframe/egui desktop shell — binary `datacraft` (Phase 5+) |

```text
datacraft-ui (egui) ──────► datacraft-core (command bus) ──► datacraft-storage (Jet/ACE)
JSON-RPC / MCP (Phase 9) ──┘
```

The UI never touches storage directly — every client speaks the same command
language, and a ribbon button dispatches the identical `Command` as an MCP
tool call. Dependency direction is one-way and review-enforced:
`ui → core`, `storage → core`, `core → nothing`. The full blueprint — command
API shape, test-ID conventions, storage decision record, rendering strategy —
is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Roadmap

Nine phases take DataCraft from skeleton to release:

| Phase | Delivers | Status |
| --- | --- | --- |
| 1 — Blueprint & Skeleton | architecture blueprint, workspace, CI gates, testing policy | ✅ |
| 2 — Storage Engine Core | `StorageEngine` trait, Jet page layer, TDEF/TDAT decoder, libmdb oracle | ✅ |
| 3 — Data Model & Schema | Access data types, field properties, serde persistence | ⬜ |
| 4 — Headless Core & Command API | command bus, state query, test IDs, JSON-RPC stub | ⬜ |
| 5 — UI Skeleton (egui) | app shell, ribbon, navigation pane, tabbed workspace | ⬜ |
| 6 — Datasheet & Design View | shared grid core, datasheet, table design view | ⬜ |
| 7 — QBE Query Builder | relationship mapper, QBE grid, query engine | ⬜ |
| 8 — Form & Report Canvas | WYSIWYG canvas, Access control types, form runtime | ⬜ |
| 9 — MCP Endpoint & Automation | MCP/JSON-RPC server, headless E2E harness, CLI client, release prep | ⬜ |

**Scope note:** compatibility is bought ring by ring — expression service →
query engine → macros → forms runtime. DataCraft implements **no VBA
interpreter or runtime**; VBA is extracted, displayed and statically analyzed
only. The ring model and rationale are in [docs/ROADMAP.md](docs/ROADMAP.md).

## Quality gates

Mechanically enforced in CI on every push and pull request:

| Gate | Enforcement |
| --- | --- |
| Test coverage > 95 % | `cargo-llvm-cov --fail-under-lines 95` |
| Source files < 500 lines | CI job fails on any `.rs` file ≥ 500 lines |
| **No technical debt** | CI scan: no `TODO`/`FIXME`/`HACK`/`XXX` in tracked files |
| Formatting & lints | `rustfmt --check`, `clippy -D warnings` |
| Cross-platform tests | Matrix: ubuntu-latest, windows-latest, macos-latest |
| Format oracle | libmdb cross-check of every decoded table and row |

## Design & branding

The app icon follows the **office-symbol** branch of the ArtCraft Crafting
Apps (WordCraft, GridCraft, DeckCraft): a 512-unit rounded tile in **exactly
three colours** — ink `#0B0B0C`, paper `#EFE9DC` and DataCraft teal `#0F8B7E`
— with a flat paper-and-ink object rotated −6°. DataCraft's object is a
database cylinder whose front is a datasheet. Construction, palette and
acceptance checklist: [docs/brand/DATACRAFT-ICON.md](docs/brand/DATACRAFT-ICON.md);
renders and the Windows `.ico` live in [`assets/app-icon/`](assets/app-icon/).

## License and credits

- **DataCraft code:** MIT OR Apache-2.0 (dual-licensed, to be finalized with
  the skeleton).
- **MDB Tools (`libmdb`):** LGPL-2.1 — used exclusively as a test-time oracle,
  never linked into release builds.

Clean-room rules are binding: sources of format knowledge are public
specifications (e.g. Microsoft Open Specifications) and community format
documentation — never MDB Tools source code, never decompiled Microsoft
binaries. See `docs/ARCHITECTURE.md` §12 for the full policy.
