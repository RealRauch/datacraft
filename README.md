<div align="center">

<img src="assets/app-icon/datacraft-256.png" width="140" alt="DataCraft icon: a database cylinder whose front is a datasheet">

**Databases and the classic Access workflow; an open-source, clean-room
reimplementation of the Microsoft Access experience, built in pure Rust.**

A fast, open-source database tool with a familiar workflow: the ribbon,
navigation pane, tabbed documents, datasheet and design views, the QBE query
builder and a form/report canvas. It reads Jet3/Jet4 `.mdb`, ACE `.accdb`,
SQLite databases and CSV directories, and writes Jet4/ACE and SQLite on
verified table/row paths (the
[compatibility matrix](#compatibility-matrix-verified-2026-11-12) states
exactly what is covered), runs natively on macOS, Windows and Linux, and is
designed to be operated by humans **and** AI agents through the same command
API.

[![Written in Rust](https://img.shields.io/badge/written%20in-Rust-0F8B7E?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Runs on macOS, Windows and Linux](https://img.shields.io/badge/runs%20on-macOS%20%7C%20Windows%20%7C%20Linux-0F8B7E?style=flat-square)](#)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-0F8B7E?style=flat-square)](#license-and-credits)
[![CI](https://img.shields.io/github/actions/workflow/status/RealRauch/datacraft/ci.yml?style=flat-square&label=CI)](https://github.com/RealRauch/datacraft/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/actions/workflow/status/RealRauch/datacraft/release.yml?style=flat-square&label=release&event=workflow_dispatch)](https://github.com/RealRauch/datacraft/actions/workflows/release.yml)

**Why DataCraft** ·
[What works today](#what-works-today) ·
[Download and install](#download-and-install) ·
[Architecture](#architecture) ·
[Roadmap](#roadmap) ·
[Contributing](#contributing) ·
[Design](#design--branding) ·
[License and credits](#license-and-credits)

</div>

![The DataCraft desktop shell with the Bike Shop sample open in the datasheet view](https://raw.githubusercontent.com/RealRauch/datacraft/main/docs/images/screenshot-light.png)

*The Bike Shop sample in the datasheet view — the same shell on Windows,
macOS and Linux.*

> [!NOTE]
> **DataCraft is in early development.** Phases 1–29 are complete: the
> UI shell wears the rebased look (neutral tokens, teal accent,
> Inter/JetBrains Mono typography, original vector iconography and
> Light/Dark/System appearance), the UI-owned `ui.json` saves atomically
> and restores the full window geometry, and the Access UI alignment landed
> (contextual ribbon tabs with the Views split-button, quick access toolbar,
> File backstage, datasheet/nav-pane/tab UX, window title and keyboard
> shortcuts/key tips). Phase 30 added the driver model (ADR-7): every
> data source docks behind the core-owned `StoragePort` and reports its
> per-session capabilities — the UI greys out what a source cannot do,
> and a CSV directory driver proves out-of-tree docking.
> The pure-Rust storage engine reads Jet3/Jet4 `.mdb` and ACE
> `.accdb` files (Jet4/ACE cross-checked against the libmdb oracle in CI) and
> writes verified Jet4 table/row paths; the headless command bus drives a
> serializable snapshot with JSON-RPC/MCP and CLI clients; the egui shell has
> the Access layout with datasheets, the QBE grid, the form/report canvas, the
> relationship mapper, a German UI (system-detected, English fallback; ADR-4)
> with the backstage Options page owning the language and appearance switchers,
> and a UI-owned `ui.json`
> for language, appearance and the full window geometry (position, size,
> maximized; ADR-5). Expect breaking changes. The honest, step-by-step
> picture lives in [BACKLOG.md](BACKLOG.md) and [PROGRESS.md](PROGRESS.md);
> the verified compatibility claims are in
> [Compatibility matrix](#compatibility-matrix-verified-2026-11-12).

## Download and install

Prebuilt archives for Windows, macOS and Linux are on the
[releases page](https://github.com/RealRauch/datacraft/releases/latest).
Each archive carries the desktop shell (`datacraft`), the CLI
(`datacraft-cli`), the MCP endpoint (`datacraft-mcp`), the sample
databases and the third-party notices. Unpack it and run the binary — no
Rust toolchain needed. The same page carries the `checksums-sha256.txt`.

From source (Rust 1.98+, pinned in `rust-toolchain.toml`; rustup installs
it automatically on build):

```bash
git clone https://github.com/RealRauch/datacraft
cd datacraft
cargo run -p datacraft-ui        # the desktop shell
cargo test --workspace           # unit, integration, proptest and golden-file tests
```

Headless clients of the same command bus:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"inspect"}' | cargo run -p datacraft-mcp
cargo run -p datacraft-cli -- open fixtures/testV2000.mdb --json
```

The libmdb oracle cross-check additionally runs in CI (ubuntu and macOS, with
`mdbtools` installed there); golden-file tests run everywhere.

## Why DataCraft

- **Familiar.** The Access workflow reimagined: ribbon, navigation pane,
  tabbed documents, datasheet and design views, the QBE grid, form and report
  canvas — rendered in an original, minimal visual language of its own.
- **Your files.** Verified read paths for `.mdb` (Jet3/Jet4) and `.accdb`
  (ACE), plus verified Jet4 and ACE write paths, in pure Rust. The
  [compatibility matrix](#compatibility-matrix-verified-2026-11-12) names
  every covered and unsupported path — no blanket "bit-for-bit" claim. No
  COM, OLE DB, ADO or winapi anywhere.
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

DataCraft reads Jet3/Jet4 `.mdb`, ACE `.accdb`, SQLite files and CSV
directories, and writes verified Jet4/ACE and SQLite table and row paths.
Around that engine sit the Access-style shell (ribbon, navigation pane,
datasheet, design view, QBE grid, form/report canvas), the expression and
query engines, native-query execution, macro/VBA extraction with static
analysis, SQL over every source, MCP/JSON-RPC and a CLI — all over one
command bus.

- **Formats:** Jet3/Jet4 read; Jet4 and ACE write (tables, rows, schema,
  indexes, relationships); SQLite read/write; CSV directories
  read/write; ACE complex values; RC4 passwords; Access lock-file
  co-existence.
- **Workbench:** datasheet with in-cell editing, sort/filter/freeze and
  undo/redo; table design with the property sheet; QBE builder; form and
  report designers with a runtime; relationship mapper; SQL window.
- **Programmability:** native saved queries (qualified dialect), macro
  inventory, VBA extraction, viewer and static analysis — no execution.
- **Automation:** the same commands through the GUI, `datacraft-cli` and
  the `datacraft-mcp` stdio endpoint; journaled writes and inspectable
  snapshots.
- **Data:** blank Jet4/SQLite databases, shipped templates and samples,
  CSV import, and the `datacraft-import` Access migration tool.

The full inventory — every capability with its phase provenance and known
limits — is in [docs/FEATURES.md](docs/FEATURES.md). The format claims are
in the [compatibility matrix](#compatibility-matrix-verified-2026-11-12);
the honest phase-by-phase picture is in
[docs/ROADMAP.md](docs/ROADMAP.md), and the format facts are catalogued in
[docs/FORMAT-JET.md](docs/FORMAT-JET.md).

## Compatibility matrix (verified 2026-11-12)

**Read** means decode through the `StoragePort`; **write** means the path
is exercised against a real file. Writes are verified as Access-readable
and usage-map-consistent — byte-identical output to Microsoft Access is
**not** claimed. The evidence behind every row — the test suites, oracle
lanes and CI runs — is in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

| Format / path | Read | Write | Known limits |
| --- | --- | --- | --- |
| **Jet3 `.mdb`** | ✅ tables, TDEF chains, jump-table rows, overflow/LVAL, embedded OLE, RC4 password | ⬜ | fixture-tested only (no libmdb oracle); no write path; Jet3 VBA/macro definitions stay name-only |
| **Jet4 `.mdb`** | ✅ tables, catalog, rows, OLE/memo, relationships, native queries, index metadata, RC4 password | ✅ create table, insert, update, delete, drop table, table rename (catalog + relationship rows), design-time alter (add/drop/rename a field/required), field indexes (metadata + uniqueness), relationship create/delete, journaled batches, LongInteger AutoNumber counter; created files and their tables are **engine-accepted** (2026-11-07 DAO oracle: `TableDefs` enumerates them, rows read and written through the engine, engine-side CREATE/INSERT/DROP round-trip on the same file) and **user-verified in real Access** (2026-11-07 walkthrough: opens without repair, template rows read, Access and DataCraft writes interleave with the autonumber continuing across engines, schema create/drop round-trips both ways) | writer verified on Jet4 `.mdb` only; created tables carry the engine's default ACL and the observed index-leaf maintenance, so catalog resolution works; Memo/OLE/Binary index keys on existing physical indexes answer `Unsupported` and leaves are rebuilt, never split (multi-page indexes stay roadmap M15); table drops free the dropped table's index pages (M66 contract test); engine-refused databases (e.g. pre-49.2 create-path output) are recovered by re-encoding through the create path (`examples/recover_mdb.rs`, 49.4 — `SaveDatabaseAs` copies pages and cannot repair, M67); MSysQueries attribute 12 and unmapped constructs answer `Unsupported` (the open carries `native_queries_error`, surfaced at the Queries group) |
| **ACE `.accdb`** | ✅ 2007/2010/2016 tables, complex values, named macros, VBA project | ✅ create table, insert, update, delete, drop table, table rename, design-time alter (add/drop/rename a field/required), field indexes (metadata + uniqueness), relationship create/delete on ACE 2007/2010 | no external ACE oracle (mdb-tools reads no `.accdb`) — the write reference is decode∘op∘decode against the pristine pinned bytes; office-encrypted files answer `Unsupported` at the reader and writer boundary |
| **Access co-existence** | ✅ a sibling Access lock file (`.ldb` / `.laccdb`) is detected at open — the session opens **read-only** with the lock file named in the snapshot (CLI/MCP data), the status bar and the Info page | ⬜ read-only while the lock file exists | DataCraft never creates or deletes lock files; detection is existence-only — the lock protocol's contents are clean-room-unclear (M59); close Access and remove the lock file to edit here |
| **SQLite** | ✅ catalog, rows, foreign keys; the file's own views (read-only), index/trigger inventory, file facts + bounded `quick_check` | ✅ create/drop table, insert/update/delete, `CREATE [UNIQUE] INDEX` from `IndexMode`, foreign-key create/delete (transactional table rebuild, enforced) | separate backend behind the same command vocabulary; schema alteration (and therefore index changes after create) remains unsupported; `WITHOUT ROWID` tables read in key order and refuse rowid-based updates/deletes; native view/trigger definitions are never created or executed |
| **CSV directories** | ✅ catalog, rows | ✅ create directory, create/drop table, insert/update/delete (one atomic rewrite per call) | column kinds inferred at open (all-integer → Long Integer, all-float → Double, all-ISO-date → Date/Time; one quoted cell or any ambiguity keeps Short Text; values read as text); no schema alteration, relationships, native queries, macros or VBA; no engine-assigned AutoNumbers |
| **Form runtime (ADR-8)** | ✅ bound text/checkbox/combo editing with per-record commit, button actions, form-scoped filters | ✅ row writes through the source driver (capability-gated) | no macro/VBA actions (ADR-2); the runtime's design data lives in the sidecar, filters are session state — never written into Jet/ACE; no Access SaveAsText/COM round trip |
| **Design layer (ADR-1)** | sidecar `.datacraft` forms, queries, reports, views (named SELECTs — SQLite applies them natively, Jet/ACE/CSV execute them through the shadow engine); native saved queries read-only; the navigation pane lists the catalog's native Access forms/reports read-only (names and counts, no definition decoding — M87; the M89 feasibility verdict is a documented stop in `docs/FORMAT-JET.md`) | ✅ sidecar save/load; sibling sidecar auto-loads on open (errors surface as `sidecar_error`); ⬜ native form/report writes | native Access form/report definitions are neither decoded nor written; the sidecar is versioned and database-name checked |
| **Samples & templates** | ✅ every shipped sample opens with its expected tables/rows | ✅ `CreateDatabase` (Jet4 + SQLite), `CreateDatabaseFromTemplate` (blank, contacts, inventory, bike_shop) | Jet3/ACE files are never created (`Unsupported`); the Jet engine accepts the shipped Jet4 samples and template-created databases (DAO oracle + the real-Access user walkthrough, 2026-11-07: opens, reads, writes and schema round-trips); template field properties persist through the `.datacraft` sidecar (H45) |
| **Programmability** | ✅ macro names + coarse arguments; VBA modules, source, static analysis | ⬜ read-only | no macro/VBA execution (ADR-2), no p-code; Jet3 macro/VBA definitions are not decoded |

**Sidecar vs native (ADR-1).** DataCraft's form, sidecar-query and report
designs live in `<database>.datacraft`; they are never written into Jet/ACE.
Native Access objects — saved queries, macros, VBA — are read-only. Opening a
database does not modify it.

## Architecture

| Layer | Crate | Job |
| --- | --- | --- |
| core | `datacraft-core` | domain model, command bus, application state, snapshots, JSON-RPC transport (Phases 3–4; the query/expression/type paths are re-exports since Phase 76) |
| query | `datacraft-query` | QBE model, criteria parsing, query evaluation, native saved-query execution and the shadow SQL engine behind the optional `sql-engine` feature (Phase 76, L37) |
| expr | `datacraft-expr` | Access-style expression engine — lexer, parser, evaluator, function library, format and `Like` matching (Phase 76, L36) |
| types | `datacraft-types` | the value and schema vocabulary below core: `FieldValue`, field kinds, table/field definitions, properties and the shared pure helpers (Phase 76, L36) |
| jet | `datacraft-jet` | clean-room pure-Rust Jet3/Jet4 read + Jet4/ACE write backends behind the core-owned `StoragePort` seam, libmdb oracle (`cfg(test)` only) (Phase 2, 10; split in Phase 31) |
| sqlite | `datacraft-sqlite` | SQLite backend behind the same seam (rusqlite, statically bundled) (Phase 13; split in Phase 31) |
| csv | `datacraft-csv` | CSV directory driver behind the same seam — a directory of `*.csv` files is one database (ADR-7, Phase 30) |
| storage | `datacraft-storage` | driver composite: format-detecting registry over the shipped backends plus the deterministic samples generator (Phase 29–30) |
| ui | `datacraft-ui` | eframe/egui desktop shell — binary `datacraft` (Phase 5+) |
| cli | `datacraft-cli` | the fourth command-bus client — binary `datacraft-cli` (Phase 9) |
| mcp | `datacraft-mcp` | headless stdio JSON-RPC 2.0 endpoint — standalone binary crate (Phase 9, own crate since Phase 31) |
| import | `datacraft-import` | optional Access migration tool: native Jet4/ACE form/report designs into the `.datacraft` sidecar without Microsoft Access (Phase 78, M88; links the third-party `jetdb` crate) |

```text
datacraft-ui (egui) ───────► datacraft-core (command bus) ──► datacraft-storage (composite)
datacraft-cli (shell) ──────┘            ▲                        ├─ datacraft-jet (Jet/ACE)
JSON-RPC / MCP (datacraft-mcp) ──────────┘                        ├─ datacraft-sqlite (SQLite)
                                                                  └─ datacraft-csv (CSV directories)

datacraft-core ──► datacraft-query ──► datacraft-expr ──► datacraft-types
datacraft-import ──► datacraft-core, jetdb 0.4
```

Dependency direction is one-way and review- and CI-enforced (the
boundary guard fails the push on a violation): `jet → core`,
`sqlite → core`, `csv → core`, `storage → jet, sqlite`,
`core → query → expr → types`; the three client binaries
(`datacraft-ui`, `datacraft-cli`,
`datacraft-mcp`) name `core + storage + csv` and are the composition
roots that build the composite. `core`'s `sql-engine` feature is off by
default and forwards to `datacraft-query/sql-engine`; the three client
roots enable it, so a default-feature core never pulls rusqlite.
`datacraft-import` is the one declared third-party adapter (ADR-9): it
depends on `datacraft-core` and `jetdb` 0.4 (pure Rust, no FFI/COM) and is
not linked into the `datacraft`, `datacraft-cli` or `datacraft-mcp`
binaries. Library and UI code never touches a driver directly — every
client speaks the same command language, and a ribbon button dispatches the
identical `Command` as an MCP tool call.
The crates.io publishing order, the Semver/MSRV policy and the release
checklist live in [docs/PUBLISHING.md](docs/PUBLISHING.md). The full
blueprint — command API shape, test-ID conventions, storage decision
record, rendering strategy — is in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Migrating from Microsoft Access

`datacraft-import` reads native Jet4/ACE `.mdb`/`.accdb` files **without
Microsoft Access** and merges the supported form/report designs into the
`.datacraft` sidecar next to the database:

```bash
cargo install datacraft-import
datacraft-import "MyApp.accdb"             # writes MyApp.datacraft
datacraft-import "MyApp.accdb" --strict    # exit 1 on any unsupported entry
```

Supported: label, text box, check box, combo box, command button and
subform with their rects, captions and bound fields. The database is opened
read-only and never modified; the sidecar is written atomically. Controls or
properties outside the subset are reported per object, never guessed —
combo row sources and controls whose geometry Access stored only as a
default are first-cut limitations. Access 97 (Jet3) form/report designs are
not supported; tables, queries and rows are opened by DataCraft itself.
The binding mapping contract is
[docs/M88-IMPORT.md](docs/M88-IMPORT.md).

## Roadmap

Delivered phases and open work live in [docs/ROADMAP.md](docs/ROADMAP.md);
the step-by-step history is in [PROGRESS.md](PROGRESS.md) and the
changelog. Compatibility is bought ring by ring — expression service →
query engine → macros → forms runtime. DataCraft implements **no VBA
interpreter or runtime**; VBA is extracted, displayed and statically
analyzed only (ADR-2). The verified claim set is the
[compatibility matrix](#compatibility-matrix-verified-2026-11-12).
## Contributing

The working agreement is [`AGENTS.md`](AGENTS.md); the practical entry —
test-first, the canonical verification set, the gate table and where the
detail lives — is [`CONTRIBUTING.md`](CONTRIBUTING.md). CI enforces
coverage ≥ 95 % lines, every `.rs` file below 500 lines, no banned
technical-debt markers, `rustfmt` and `clippy -D warnings`, rustdoc, the
crate-boundary guard and a three-OS test matrix; the oracle suite
cross-checks the Jet/ACE decoders against libmdb.

## Design & branding

The app icon follows the **office-symbol** visual grammar of the sibling
Crafting Apps (WordCraft, GridCraft, DeckCraft): a 512-unit rounded tile in **exactly
three colours** — ink `#0B0B0C`, paper `#EFE9DC` and DataCraft teal `#0F8B7E`
— with a flat paper-and-ink object rotated −6°. DataCraft's object is a
database cylinder whose front is a datasheet. Construction, palette and
acceptance checklist: [docs/brand/DATACRAFT-ICON.md](docs/brand/DATACRAFT-ICON.md);
renders and the Windows `.ico` live in [`assets/app-icon/`](assets/app-icon/). The app's UI rebases on a neutral token system (chrome, ribbon, canvas and panel surfaces in neutral greys; text and border ramps; ADR-6) while the accent family derives from the icon's teal tile field — the datasheet keeps the icon's header-row grammar (teal header, paper text), and hover/pressed states stay neutral. The backstage follows the Excel reference: a full-height deep-teal sidebar with icon+label navigation and the bottom-pinned Options entry, template/sample cards with painted mini-previews, and localized Modified stamps on the recent list (step 38.2); native file pickers and destructive-action confirmations run through `rfd` behind a scripted dialog seam, so the headless suite drives every flow with injected answers (step 38.3). Typography is Inter (Regular/Medium/SemiBold) with JetBrains Mono for the SQL editor; both families are SIL OFL 1.1, embedded unmodified and attributed in [`assets/fonts/`](assets/fonts/). Ribbon commands, navigation-pane objects and document tabs carry vector icons painted at paint time (no image assets beyond the app icon and the fonts); the backstage/chrome nav glyphs adapt the sibling Crafting Apps' in-code set (GridCraft/WordCraft, MIT OR Apache-2.0, attributed in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md)). Appearance is Light/Dark/System (default System, persisted in `ui.json`); both palettes are installed into egui and the token set is picked per frame, with the accent family always rendered in a contrast-checked teal variant. Phase 20 gate screenshots: [before (light)](docs/phase-20/before-light.png) · [after (light)](docs/phase-20/after-light.png) · [after (dark)](docs/phase-20/after-dark.png).

## License and credits

- **DataCraft code:** MIT OR Apache-2.0, dual-licensed — canonical texts in
  [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE), the
  Apache-side notice in [`NOTICE`](NOTICE).
- **Shipped third-party components** (egui stack, csv, bundled SQLite, the two
  SIL-OFL fonts): named in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md);
  the notices ship in every release archive.
- **Committed test fixtures** carry their provenance in
  [`fixtures/manifest`](fixtures/manifest) and their license/attribution rows
  in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) — every fixture is
  either generated by the Apache-2.0-licensed jackcess test suite, taken from
  the MIT-licensed mdb-reader test data, or self-created with real Access by
  DataCraft contributors.
- **MDB Tools (`libmdb`):** LGPL-2.1 — used exclusively as a test-time oracle,
  never linked into release builds.

Clean-room rules are binding: sources of format knowledge are public
specifications (e.g. Microsoft Open Specifications) and community format
documentation — never MDB Tools source code, never decompiled Microsoft
binaries. See `docs/ARCHITECTURE.md` §12 for the full policy.
