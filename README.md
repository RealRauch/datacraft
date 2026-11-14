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
[Quick start](#quick-start) ·
[Architecture](#architecture) ·
[Roadmap](#roadmap) ·
[Quality gates](#quality-gates) ·
[Design](#design--branding) ·
[License and credits](#license-and-credits)

</div>

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

Every compatibility claim below names its evidence; unsupported paths are
stated in the same row. Re-verified 2026-11-10 for the v0.1.1 release:
every evidence suite named below is green on CI `main` (run 42895207902;
the mechanical claim); the real-engine checks stand from the 2026-11-07
Access walkthrough and the DAO create-path oracle (phase 49.2, item M63). **Read** means decode through the `StoragePort`;
**write** means the path is exercised against a real file in the named
suite. Writes are verified as Access-readable and usage-map-consistent —
byte-identical output to Microsoft Access is **not** claimed.

| Format / path | Read | Write | Evidence | Known limits |
| --- | --- | --- | --- | --- |
| **Jet3 `.mdb`** | ✅ tables, TDEF chains, jump-table rows, overflow/LVAL, embedded OLE, RC4 password | ⬜ | `jet3.rs`, `jet3_remainders.rs`, `jet_encryption.rs` against `testV1997.mdb` / `test2V1997.mdb` | fixture-tested only (no libmdb oracle); no write path; Jet3 VBA/macro definitions stay name-only |
| **Jet4 `.mdb`** | ✅ tables, catalog, rows, OLE/memo, relationships, native queries, index metadata, RC4 password | ✅ create table, insert, update, delete, drop table, table rename (catalog + relationship rows), design-time alter (add/drop/rename a field/required), field indexes (metadata + uniqueness), relationship create/delete, journaled batches, LongInteger AutoNumber counter; created files and their tables are **engine-accepted** (2026-11-07 DAO oracle: `TableDefs` enumerates them, rows read and written through the engine, engine-side CREATE/INSERT/DROP round-trip on the same file) and **user-verified in real Access** (2026-11-07 walkthrough: opens without repair, template rows read, Access and DataCraft writes interleave with the autonumber continuing across engines, schema create/drop round-trips both ways) | `jet_table.rs`, `jet_catalog.rs`, `native_queries.rs`, `jet_encryption.rs`; `jet_writer*.rs`, `jet_port_writes.rs`, `jet_row_writes.rs`, `jet_autonumber.rs`, `jet_table_drop.rs`, `jet_alter_table.rs`, `jet_index_writes.rs`, `jet_create_shape.rs`; oracle + `mdb-export` CI plus the Windows DAO open/read/write oracle (phase 49.2) | writer verified on Jet4 `.mdb` only; created tables carry the engine's default ACL and the observed index-leaf maintenance, so catalog resolution works; Memo/OLE/Binary index keys on existing physical indexes answer `Unsupported` and leaves are rebuilt, never split (multi-page indexes stay roadmap M15); table drops free the dropped table's index pages (M66 contract test); engine-refused databases (e.g. pre-49.2 create-path output) are recovered by re-encoding through the create path (`examples/recover_mdb.rs`, 49.4 — `SaveDatabaseAs` copies pages and cannot repair, M67); MSysQueries attribute 12 and unmapped constructs answer `Unsupported` (the open carries `native_queries_error`, surfaced at the Queries group) |
| **ACE `.accdb`** | ✅ 2007/2010/2016 tables, complex values, named macros, VBA project | ✅ create table, insert, update, delete, drop table, table rename, design-time alter (add/drop/rename a field/required), field indexes (metadata + uniqueness), relationship create/delete on ACE 2007/2010 | `ace_fixtures.rs`, `complex_values.rs`, `macros.rs`, `vba.rs`; oracle on three template fixtures; `ace_writer.rs`, `ace_port_writes.rs`, `server_e2e_ace.rs` (phase 25) | no external ACE oracle (mdb-tools reads no `.accdb`) — the write reference is decode∘op∘decode against the pristine pinned bytes; office-encrypted files answer `Unsupported` at the reader and writer boundary |
| **Access co-existence** | ✅ a sibling Access lock file (`.ldb` / `.laccdb`) is detected at open — the session opens **read-only** with the lock file named in the snapshot (CLI/MCP data), the status bar and the Info page | ⬜ read-only while the lock file exists | `jet_capabilities.rs` (locked-session caps + precise `Unsupported` on every write), `driver_caps.rs` (the snapshot data), `ui_layout.rs` / `ui_kittest.rs` (localized notices); FORMAT-JET §Access co-existence | DataCraft never creates or deletes lock files; detection is existence-only — the lock protocol's contents are clean-room-unclear (M59); close Access and remove the lock file to edit here |
| **SQLite** | ✅ catalog, rows, foreign keys; the file's own views (read-only), index/trigger inventory, file facts + bounded `quick_check` | ✅ create/drop table, insert/update/delete, `CREATE [UNIQUE] INDEX` from `IndexMode`, foreign-key create/delete (transactional table rebuild, enforced) | `sqlite_port.rs`, `sqlite_writes.rs`, `sqlite_row_writes.rs`, `sqlite_indexes.rs`, `foreign_views.rs`, `foreign_objects.rs`, `file_facts.rs`, `robust_open.rs` (phase 73) | separate backend behind the same command vocabulary; schema alteration (and therefore index changes after create) remains unsupported; `WITHOUT ROWID` tables read in key order and refuse rowid-based updates/deletes; native view/trigger definitions are never created or executed |
| **CSV directories** | ✅ catalog, rows | ✅ create directory, create/drop table, insert/update/delete (one atomic rewrite per call) | `crates/datacraft-csv` (`tests/driver.rs`, `tests/error_paths.rs`, `tests/inference.rs`), `server_e2e_csv.rs` (phase 30, 36) | column kinds inferred at open (all-integer → Long Integer, all-float → Double, all-ISO-date → Date/Time; one quoted cell or any ambiguity keeps Short Text; values read as text); no schema alteration, relationships, native queries, macros or VBA; no engine-assigned AutoNumbers |
| **Form runtime (ADR-8)** | ✅ bound text/checkbox/combo editing with per-record commit, button actions, form-scoped filters | ✅ row writes through the source driver (capability-gated) | `row_edit_zls.rs`, `form_control_data.rs`, `form_filter.rs`, `button_action.rs`, `ui_kittest_form_edit.rs`, `ui_kittest_control_content.rs`; ADR-8 (ARCHITECTURE §17) | no macro/VBA actions (ADR-2); the runtime's design data lives in the sidecar, filters are session state — never written into Jet/ACE; no Access SaveAsText/COM round trip |
| **Design layer (ADR-1)** | sidecar `.datacraft` forms, queries, reports, views (named SELECTs — SQLite applies them natively, Jet/ACE/CSV execute them through the shadow engine); native saved queries read-only; the navigation pane lists the catalog's native Access forms/reports read-only (names and counts, no definition decoding — M87; the M89 feasibility verdict is a documented stop in `docs/FORMAT-JET.md`) | ✅ sidecar save/load; sibling sidecar auto-loads on open (errors surface as `sidecar_error`); ⬜ native form/report writes | `schema_document.rs`, `canvas.rs`, `form_runtime.rs`; `native_queries.rs`; `native_objects.rs`; `sidecar_auto_load.rs`, `server_e2e_samples.rs` (phase 29) | native Access form/report definitions are neither decoded nor written; the sidecar is versioned and database-name checked |
| **Samples & templates** | ✅ every shipped sample opens with its expected tables/rows | ✅ `CreateDatabase` (Jet4 + SQLite), `CreateDatabaseFromTemplate` (blank, contacts, inventory, bike_shop) | `create_database.rs`, `create_template.rs`, `create_from_template.rs`, `samples.rs`, `server_e2e_samples.rs` against the committed `samples/` set; `make_samples --check` + `sha256sum -c samples/manifest` | Jet3/ACE files are never created (`Unsupported`); the Jet engine accepts the shipped Jet4 samples and template-created databases (DAO oracle + the real-Access user walkthrough, 2026-11-07: opens, reads, writes and schema round-trips); template field properties persist through the `.datacraft` sidecar (H45) |
| **Programmability** | ✅ macro names + coarse arguments; VBA modules, source, static analysis | ⬜ read-only | `macros.rs`, `port_macros.rs`, `vba.rs`, `vba_project.rs`, `vba_analysis.rs` | no macro/VBA execution (ADR-2), no p-code; Jet3 macro/VBA definitions are not decoded |

**Sidecar vs native (ADR-1).** DataCraft's form, sidecar-query and report
designs live in `<database>.datacraft`; they are never written into Jet/ACE.
Native Access objects — saved queries, macros, VBA — are read-only. Opening a
database does not modify it.

**Review by format.** Jet3 is the narrowest read-only path (fixture-tested,
no oracle); Jet4 has the deepest verified read/write path; ACE adds complex
values and read-only programmability; SQLite round-trips through its own
backend. These rows are the only compatibility statements the project makes —
broader wording was removed in step 17.4.

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
echo '{"jsonrpc":"2.0","id":1,"method":"inspect"}' | cargo run -p datacraft-mcp
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

Delivered phases and the open work:

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
| 11 — Port Write-Through & Journal | insert/update/delete through the port, journaled write profiles | ✅ |
| 12 — Access Look & Theme | Original palette, Access ribbon/nav-pane/status/datasheet styling | ✅ |
| 13 — SQLite & SQL | SQLite source backend, SQL window, insert flows | ✅ |
| 14 — ACCDB Read | ACE fixtures and the ACE read path | ✅ |
| 15 — Expressions & Saved Queries | expression service, field wiring, `MSysQueries` decode | ✅ |
| 16 — Complex Values & Encryption | attachments/multi-value/version history, RC4 passwords, Jet3 remainders | ✅ |
| 17 — Macros, VBA & Docs Accuracy | read-only macro inventory, VBA extraction/viewer/static analysis, verified compatibility matrix | ✅ |
| 18–19 — UI Localization & Settings | i18n kernel, complete German catalog, `ui.json`, language switcher | ✅ |
| 20 — Look & Typography | neutral tokens, Inter/JetBrains Mono typography, original vector iconography, Light/Dark/System appearance | ✅ |
| 21 — Access UI Alignment | contextual ribbon tabs, quick access toolbar, backstage, datasheet/nav-pane/tab UX, keyboard | ✅ |
| 22 — Settings Hardening | atomic `ui.json` save, window position/maximized state, off-screen rescue | ✅ |
| 23 — Hardening & Test Gaps | CLI process E2E (exit codes, stderr), headless kittest UI suite with AccessKit snapshots, native-query decode errors surfaced at open | ✅ |
| 24 — Jet Writer Remainder | AutoNumber counter maintenance, table drops through the writer, design-time schema editing (add/drop/rename/required) | ✅ |
| 25 — ACCDB Write | ACE write contracts, port write-through and bus E2E on `.accdb` | ✅ |
| 26–29 — Release, Queries, UI Rest, Samples | release engineering (matrix, checksums, packaged smoke), native query execution + encrypted save-as, undo/redo + recent databases, create-database flow, templates and shipped samples | ✅ |
| 30 — Driver Model | declarative per-session capabilities, registry detection, CSV directory driver as the out-of-tree proof, capability-disabled UI affordances (ADR-7) | ✅ |
| 31 — Publishing & Crate Split | jet/sqlite/mcp as standalone crates behind the composite, publish-ready vocabulary, Semver/MSRV/changelog policy | ✅ |
| 32 — Schema Writing | persisted field properties, index creation, relationship/foreign-key creation | ✅ |
| 33 — Journal Capability Gating | journaled batches gate on the declared atomic_batch capability | ✅ |
| 34 — Cross-Cutting Review | storage/drivers, core, UI and public-API reviews with in-phase fixes | ✅ |
| 35 — Documentation Sweep | entry-point docs, ARCHITECTURE stale sections, FORMAT-JET/TESTING, matrix re-verification | 🔄 |
| 36 — Driver Edge Suites | CSV/detecting/MCP coverage, driver inventory in the snapshot, CSV type inference, tri-state Boolean NULL decision | ✅ |
| 37 — Release Readiness | self-created fixtures replace Northwind, fixture rebuild tooling, licenses/attribution, history hygiene | ✅ |
| 38 — UI Restyle | icon glyph port, backstage restyle, native dialogs behind the seam, window chrome polish | ✅ |
| 39 — Correctness Fences | named date pictures, SQL-dialect fence, QBE criteria fence, cache load errors surfaced | ✅ |
| 40 — Datasheet UX | keyboard record navigation, horizontal scroll + freeze columns, row deletion, in-place cell editing | ✅ |
| 41 — GUI Parity | relationship mapper affordances, expression window, VBA analysis viewer, report grouping picker, ribbon truth pass | ✅ |
| 42 — Core Correctness | SQLite affinity edge cases, capability-gated design edits, by-name QBE projection | ✅ |
| 43 — Coverage & CI Pinning | coverage gap audit, cargo-llvm-cov pinned in the CI coverage job | ✅ |
| 44 — UI Layout & Affordance Audit | content-sized ribbon groups, panel-geometry pins, Close Database, German label semantics, datasheet sort/filter with the core view state, the layout-rect seam | ✅ |
| 45 — Walkthrough Fixes | column filter dialog replaces the in-menu editor (menus cannot host text editing), record-navigation arrow order | ✅ |
| 46 — Form Runtime Layout | the runtime renders the designed layout: the shared designer canvas, controls at their stored rects and sizes | ✅ |
| 47 — Walkthrough Fixes II | activating an object surfaces its contextual ribbon tab (backstage and same-kind switches never yank) | ✅ |
| 48 — Walkthrough Fixes III | record counter reads its cursor, datasheet trailing panels stay above the status bar (the add-record path), filter-dialog criteria syntax | ✅ |
| 49 — Page-0 Tail & Access Compatibility | the writer emits and maintains the tail and remaining region fields; DataCraft-created files open in real Access; the recovery example re-encodes engine-refused files | ✅ |
| 50 — CI macOS Re-Admission | macOS back in the test and oracle matrices; README/AGENTS/TESTING aligned | ✅ |
| 51 — Changelog Cadence | per-phase changelog maintenance (coarse entries) and the Unreleased catch-up | ✅ |
| 52 — CI Ubuntu Diagnosis | the ld SIGBUS root-caused to disk exhaustion on the migrated ubuntu-26 image; the headroom step keeps the job green | ✅ |
| 53 — Access Co-existence | a sibling lock file (`.ldb`/`.laccdb`) opens the database read-only with the localized notice; DataCraft never creates or deletes lock files | ✅ |
| 54 — Next-Version Note | the unreleased set names its target version at every phase close | ✅ |
| 55 — Docs Truth Pass | coverage-gate wording, the save-as repair boundary, the headless-only journal batch, the German step titles | ✅ |
| 56 — Release Readiness | publish metadata, workspace dependency versions and the lexer's last `expect` — the six crates publish cleanly | ✅ |
| 57 — Report Grouping Bookkeeping | the M48 grouping picker reconciled (delivered in 41.4) | ✅ |
| 58 — Import File | external CSV files import as tables of the open Jet4/SQLite database over one bus chain (preview, inferred kinds, capability-gated) | ✅ |
| 59 — CI Hardening | dependency-boundary guard, rustdoc `-D warnings` and the CLI smoke join the gates | ✅ |
| 60 — Small Closes | the expression window's row context; the oracle skip is visible as `ignored` | ✅ |
| 61 — Fixture Generators | deterministic canonical-database generators beside the pins | ✅ |
| 62 — Failure Codes | every dispatch failure carries a stable machine-readable code; the UI localizes known codes | ✅ |
| 63 — Designer Control Content | the selected control's content property (label text, caption, bound field, subform) in both designers over one bus command | ✅ |
| 64 — Form Runtime Completion | Access-verified runtime: per-record editing with Esc semantics, zero-length strings, two-state checkboxes, combo row sources with free entry, button actions and form-scoped filters | ✅ |

**Scope note:** compatibility is bought ring by ring — expression service →
query engine → macros → forms runtime. DataCraft implements **no VBA
interpreter or runtime**; VBA is extracted, displayed and statically analyzed
only (Phase 17, ADR-2). The ring model and rationale are in
[docs/ROADMAP.md](docs/ROADMAP.md); the verified claim set is the
[compatibility matrix](#compatibility-matrix-verified-2026-11-12).

## Quality gates

Mechanically enforced in CI on every push and pull request:

| Gate | Enforcement |
| --- | --- |
| Test coverage ≥ 95 % | CI exports lcov and evaluates ≥ 95 % lines with an explicit parse — the tool's embedded `--fail-under-lines` check diverged from its own export (M64); a local `--fail-under-lines 95` run is an approximation, `datacraft-ui` is the registered exception |
| Source files < 500 lines | CI job fails on any `.rs` file ≥ 500 lines |
| **No technical debt** | CI scan: no `TODO`/`FIXME`/`HACK`/`XXX` in tracked files (append-only archives excluded) |
| Formatting & lints | `rustfmt --check`, `clippy -D warnings` |
| Cross-platform tests | Matrix: ubuntu-latest, windows-latest, macos-latest (macOS re-admitted 2026-11-07, item L25; the release workflow still builds and smokes macOS artifacts) |
| Format oracle | libmdb cross-check of the decoded Jet4/ACE fixture tables and rows |

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
