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
> **DataCraft is in early development.** The Phase 1–9 plan is complete: the
> pure-Rust storage engine core reads Jet4 `.mdb` files (cross-checked
> against the libmdb oracle in CI), the headless command bus drives a
> serializable snapshot with JSON-RPC support, the egui shell has the
> Access layout with datasheets and a read-only design view, the
> relationship mapper is live, and the QBE grid runs an in-memory query
> engine (no SQL parser) with sortable, filterable results, and the
> form designer/runtime places Access-style controls on a WYSIWYG canvas
> with data binding and a versioned sidecar. Reports share the canvas
> and land as a follow-up (BACKLOG H31). Expect
> breaking changes. The honest, step-by-step picture lives in
> [BACKLOG.md](BACKLOG.md) and [PROGRESS.md](PROGRESS.md).

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
| **Storage — ACE read** | `.accdb` (ACE 2007/2010/2016) decodes through the same Jet4 4096-byte path — header/product consistency check, catalog, TDEF/TDAT, rows, relationships; three pristine fixtures are pinned and the mdb-tools oracle cross-checks columns, row counts and values in CI; unverified header combinations answer a precise `Unsupported` |
| **Storage — SQLite sources** | `SqlitePort` behind the same `StoragePort`: magic detection, `sqlite_master` catalog, `PRAGMA table_info` schema mapped onto Access field kinds (documented affinity table), foreign keys as relationships, row windows over rowid order, and writes (table create/drop, parameterized row inserts) — GUI, MCP and CLI open and fill `.sqlite` files through the same command vocabulary |
| **Storage — Jet3/Jet4 read** | `StorageEngine` trait boundary; Jet page layer and file-header decoder; TDEF/TDAT decoders for Jet4 (unicode/compression) and Jet3 (CP1252); embedded OLE columns (fixed 0x11 long binary) decode; golden-file tests against sha256-pinned fixtures; every parse cross-checked against the libmdb oracle in CI |
| **Storage — Jet4 write** | clean-room writer core: usage-map-consistent page allocation (global free map + per-table used/free maps, black-box verified against real Access files), table creation with catalog rows, row insertion with free-map page reuse, row update (in-place for same-length encodings, page repacking otherwise) and row delete (the Access `0x8000` flag) — **write-through behind the port** (`CreateTable`/`InsertRow`/`UpdateRow`/`DeleteRow` persist to the `.mdb` path, verified by reopen and `mdb-export`); AutoNumber assignment and table drops remain precise `Unsupported` (BACKLOG M7/M8) |
| **Domain model & schema** | Access field kinds (Short Text … OLE/Attachment) with typed properties and validation semantics; versioned `.datacraft` schema document (serde, migration ladder) per ADR-1 |
| **Headless command bus** | Total reducer (`dispatch` → deterministic events, failures as `Failed` events), serializable `inspect()` snapshots, deterministic test IDs, JSON-RPC 2.0 codec with full error semantics, and journaled writes (`PreviewWrite` diff → token-confirmed `ApplyWrite` with rollback info; the profile refuses unqualified field families precisely) — GUI and agents speak the same language |
| **Desktop shell** | Four-region egui shell (spec-driven ribbon, snapshot-driven navigation pane with click-to-open, tabbed workspace with close/activate, status bar) — all strings and logic computed in core, paint code renders only |
| **Datasheet & design view** | Real fixture databases open through the composition root; virtualized datasheet over cached rows with Access-style cell rendering and the new-record row (typed cell parsing, `InsertRow` through the bus); read-only design grid with Access type names and a full property sheet |
| **Headless automation** | `datacraft-mcp` — a stdio JSON-RPC 2.0 endpoint agents spawn; the headless E2E harness drives the full stack over the wire against real fixtures; `datacraft-cli` (open/exec/repl) as the fourth command-bus client with deterministic exit codes |
| **Forms** | WYSIWYG designer with snap-to-grid drag, the Access toolbox palette (Label/Text Box/Check Box/Combo Box/Button/Subform), the designer field list (record-source selector plus one-click bound controls at the next free slot), whitelisted data binding, runtime record navigation, and sidecar persistence (path defaults to the database's `.datacraft`, versioned and database-name checked, save/load reported in the status bar) |
| **Reports** | print-geometry designer (A4 page with margin guides) sharing the form canvas and designer palette; print-preview runtime paginating over source rows (one page per record; grouped/aggregate bands land with the expression service) |
| **SQL over all sources** | A `Database Tools > SQL` window runs one `SELECT`/`WITH` statement through the command bus (`RunSql`/`SqlRan`): the loaded Jet/ACE or SQLite tables are materialized into a throwaway in-memory SQLite database, so joins and aggregates work across every source without touching the file; the gate rejects every other statement and multi-statement input |
| **Relationships & QBE** | Relationship mapper window fed by MSysRelationships through the port; Access-style QBE grid (table/field/sort/show/criteria rows, schema-whitelisted, persisted in the sidecar); in-memory query engine with typed predicate coercion, AND/OR composition and stable multi-key sorts — no SQL parser |
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
cargo run -p datacraft-ui        # the desktop shell
cargo test --workspace           # unit, integration, proptest and golden-file tests
```

Headless clients of the same command bus:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"inspect"}' | cargo run -p datacraft-ui --bin datacraft-mcp
cargo run -p datacraft-cli -- open fixtures/testV2000.mdb --json
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
| core | `datacraft-core` | domain model, schema, command bus, query engine, application state, snapshots, JSON-RPC transport (Phases 3–4, 7) |
| storage | `datacraft-storage` | `StorageEngine` trait, pure-Rust Jet3/Jet4 read + Jet4 write backends, `StoragePort` adapter, libmdb oracle (`cfg(test)` only) (Phase 2, 10) |
| ui | `datacraft-ui` | eframe/egui desktop shell (`datacraft`) and the MCP endpoint binary (`datacraft-mcp`) (Phase 5+) |
| cli | `datacraft-cli` | the fourth command-bus client — binary `datacraft-cli` (Phase 9) |

```text
datacraft-ui (egui) ───────► datacraft-core (command bus) ──► datacraft-storage (Jet/ACE)
datacraft-cli (shell) ──────┘            ▲
JSON-RPC / MCP (datacraft-mcp) ──────────┘
```

Dependency direction is one-way and review-enforced: `ui → core`,
`cli → core`, `storage → core`, `core → nothing`; the binary targets
are the composition roots that name storage.

The UI never touches storage directly — every client speaks the same command
language, and a ribbon button dispatches the identical `Command` as an MCP
tool call. Dependency direction is one-way and review-enforced:
`ui → core`, `storage → core`, `core → nothing`. The full blueprint — command
API shape, test-ID conventions, storage decision record, rendering strategy —
is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Roadmap

Ten phases take DataCraft from skeleton to release:

| Phase | Delivers | Status |
| --- | --- | --- |
| 1 — Blueprint & Skeleton | architecture blueprint, workspace, CI gates, testing policy | ✅ |
| 2 — Storage Engine Core | `StorageEngine` trait, Jet page layer, TDEF/TDAT decoder, libmdb oracle | ✅ |
| 3 — Data Model & Schema | Access data types, field properties, serde persistence | ✅ |
| 4 — Headless Core & Command API | command bus, state query, test IDs, JSON-RPC stub | ✅ |
| 5 — UI Skeleton (egui) | app shell, ribbon, navigation pane, tabbed workspace | ✅ |
| 6 — Datasheet & Design View | shared grid core, datasheet, table design view | ✅ |
| 7 — QBE Query Builder | relationship mapper, QBE grid, query engine | ✅ |
| 8 — Form & Report Canvas | WYSIWYG canvas, Access control types, form runtime | ✅ |
| 9 — MCP Endpoint & Automation | MCP/JSON-RPC server, headless E2E harness, CLI client, release prep | ✅ |
| 10 — Jet4 Writer, Reports & Release | Jet4 writer workstream, Jet3 read, report designer + print preview, fixture pin integrity | ✅ |

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
renders and the Windows `.ico` live in [`assets/app-icon/`](assets/app-icon/). The app itself wears the same palette: the theme colour is the icon's teal tile field, chrome surfaces are paper, text and hairlines are ink — the datasheet renders in the icon's header-row grammar (teal header, paper text).

## License and credits

- **DataCraft code:** MIT OR Apache-2.0 (dual-licensed, to be finalized with
  the skeleton).
- **MDB Tools (`libmdb`):** LGPL-2.1 — used exclusively as a test-time oracle,
  never linked into release builds.

Clean-room rules are binding: sources of format knowledge are public
specifications (e.g. Microsoft Open Specifications) and community format
documentation — never MDB Tools source code, never decompiled Microsoft
binaries. See `docs/ARCHITECTURE.md` §12 for the full policy.
