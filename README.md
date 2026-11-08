<div align="center">

<img src="assets/app-icon/datacraft-256.png" width="140" alt="DataCraft icon: a database cylinder whose front is a datasheet">

**Databases and the classic Access workflow; an open-source, clean-room
reimplementation of the Microsoft Access experience, built in pure Rust.**

A fast, open-source database tool with a familiar workflow: the ribbon,
navigation pane, tabbed documents, datasheet and design views, the QBE query
builder and a form/report canvas. It reads Jet3/Jet4 `.mdb`, ACE `.accdb`,
SQLite databases and CSV directories, and writes Jet4/ACE and SQLite on
verified table/row paths (the
[compatibility matrix](#compatibility-matrix-verified-2026-11-07) states
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
> [Compatibility matrix](#compatibility-matrix-verified-2026-11-07).

## Why DataCraft

- **Familiar.** The Access workflow reimagined: ribbon, navigation pane,
  tabbed documents, datasheet and design views, the QBE grid, form and report
  canvas — rendered in an original, minimal visual language of its own.
- **Your files.** Verified read paths for `.mdb` (Jet3/Jet4) and `.accdb`
  (ACE), plus verified Jet4 and ACE write paths, in pure Rust. The
  [compatibility matrix](#compatibility-matrix-verified-2026-11-07) names
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

| Area | Highlights |
| --- | --- |
| **Storage — ACE read** | `.accdb` (ACE 2007/2010/2016) decodes through the same Jet4 4096-byte path — header/product consistency check, catalog, TDEF/TDAT, rows, relationships; four ACE fixtures are pinned (three pristine templates plus the self-created programmability fixture), and the mdb-tools oracle cross-checks the three templates in CI; unverified header combinations answer a precise `Unsupported` |
| **Complex values (ACE)** | Attachments, multi-value and version-history columns resolve through `MSysComplexColumns` into ordered sub-records (`FieldValue::Complex`); backing `f_<guid>_*` tables are hidden from the catalog; the datasheet joins the scalar values (`FileName`/`Value`), QBE criteria on multi-value columns use documented EXISTS semantics, and complex writes stay a precise `Unsupported` |
| **Storage — SQLite sources** | `SqlitePort` behind the same `StoragePort`: magic detection, `sqlite_master` catalog, `PRAGMA table_info` schema mapped onto Access field kinds (documented affinity table), foreign keys as relationships, row windows over rowid order, and writes (table create/drop, parameterized row inserts, foreign-key create/delete through a transactional table rebuild with enforcement enabled on open) — GUI, MCP and CLI open and fill `.sqlite` files through the same command vocabulary |
| **Driver model & CSV sources** | Every data source docks behind the core-owned `StoragePort` (ADR-7) and reports a stable driver id plus per-session `DriverCapabilities` — a read-only file disables the write affordances, an open Jet3 session reports read-only, `atomic_batch` is a declared property — and the snapshot carries `driver`, `capabilities` and the disabled-affordance map so the UI greys out what a source cannot do with localized reasons (never a format match in paint code); `crates/datacraft-csv` proves out-of-tree docking: a directory of `*.csv` files opens as one database through the composite's registered fallback (tables are files, the header row names the columns, one atomic rewrite per write) |
| **Storage — Jet3/Jet4 read** | Jet page layer and file-header decoder behind the core-owned `StoragePort` seam (ADR-7); TDEF/TDAT decoders for Jet4 (unicode/compression) and Jet3 (CP1252, chained TDEFs, >255-byte jump-table rows, overflow/LVAL pages); embedded OLE columns (fixed 0x11 long binary) decode; golden-file tests against sha256-pinned fixtures; Jet4 parses are cross-checked against the libmdb oracle in CI (Jet3 is fixture-tested only) |
| **Storage — Jet4/ACE write** | clean-room writer core for Jet4 `.mdb` and ACE `.accdb` (ACE contracts proven on the pinned 2007/2010 fixtures in phase 25: every original page stays byte-identical except the writer's in-place updates, usage maps stay consistent, the 2016 fixture passes as a read-only reference): usage-map-consistent page allocation (global free map + per-table used/free maps, black-box verified against real Access files), table creation with catalog rows, row insertion with free-map page reuse, row update (in-place for same-length encodings, page repacking otherwise) and row delete (the Access `0x8000` flag), table drops that free the TDEF/TDAT pages and the catalog row — **write-through behind the port** (`CreateTable`/`InsertRow`/`UpdateRow`/`DeleteRow`/`DropTable` persist to the `.mdb`/`.accdb` path, verified by reopen; the `mdb-export` oracle cross-checks Jet4); LongInteger AutoNumbers are engine-assigned with a persisted TDEF counter |
| **Storage — encryption & passwords** | Jet3/Jet4 RC4: the fixed-key page-0 header region decodes on every open, a non-zero database key decrypts every page after page 0, and database passwords are validated (Jet3 plain text in the encrypted header, Jet4 creation-date mask) — `OpenDatabase` carries the optional password for GUI/CLI/MCP, missing/wrong answers `PasswordRequired`/`PasswordRejected`; writes keep encrypted files encrypted; `SaveDatabaseAs` writes an unencrypted or newly protected copy (the original stays untouched), `ChangePassword` rewrites the open file in place, both gated on the driver's `passwords` capability. ACE Office encryption (RC4 CryptoAPI/AES) is detected and rejected with a precise `Unsupported` |
| **Domain model & schema** | Access field kinds (Short Text … OLE/Attachment, multi-value, complex) with typed properties and validation semantics; versioned `.datacraft` schema document (serde, migration ladder) per ADR-1 |
| **Headless command bus** | Total reducer (`dispatch` → deterministic events, failures as `Failed` events), serializable `inspect()` snapshots, deterministic test IDs, JSON-RPC 2.0 codec with full error semantics, and journaled writes (`PreviewWrite` diff → token-confirmed `ApplyWrite` with rollback info; the profile refuses unqualified field families precisely, and drivers that cannot apply a batch atomically accept single-operation batches only) — GUI and agents speak the same language |
| **Undo/Redo (row writes)** | Bounded per-database history (50 entries) of row inserts/updates/deletes; `Undo`/`Redo` are ordinary bus commands, so GUI, CLI and MCP share them. Rows are addressed by their full value image, and a deleted row is restored with its original AutoNumber identity (`restore_row`, the deliberate write-rule exception). Journal batches, schema changes and file lifecycle are barriers that clear the history instead of replaying stale images; the QAT enables from the snapshot's history depth |
| **Field properties** | Captions, defaults, validation rules/text, formats, input masks, required/zero-length and index flags persist in the `.datacraft` sidecar (H45 authority); the Jet/ACE catalog keeps `NULL` LvProp, so Access display of DataCraft-set properties remains the LvProp roadmap item. A stale or kind-incompatible overlay fails precisely on an explicit load and surfaces as `sidecar_error` on auto-load; schema edits keep the other fields' properties |
| **Indexes** | Field-level `IndexMode` becomes real Jet TDEF index metadata (physical + logical blocks and names) and SQLite `CREATE [UNIQUE] INDEX`; `NoDuplicates` is enforced on insert/update/an entire journal batch with a precise error while NULLs stay free, existing fixture index sections survive unrelated row writes, and adding a unique index over existing duplicates is refused. physical index **leaves that exist in a file** (the engine's system catalogs, Access-created indexes) are maintained on every row insert/update/delete so the engine's B-tree lookups stay correct (phase 49.2), while indexes DataCraft creates itself still carry no leaf pages (`first_dp = 0`, roadmap M15 — Access may rebuild them from the metadata); sidecar `indexed` flags never override a physical index |
| **Desktop shell** | Four-region egui shell (spec-driven ribbon with contextual object tabs — Create ▸ Table Design opens a new table in the design view, Import File stays visible but names a planned feature honestly (step 41.5), quick access toolbar with history-gated Undo/Redo and the Views split-button, full-page File backstage for open/save/close/options with the bounded recent-database list (stale paths are pruned with a startup notice), snapshot-driven navigation pane with category dropdown, Open/Design/Delete context menu and name/type group sort, tabbed workspace with close/activate plus middle-click and Close/Close Others/Close All context actions, status bar with record navigation and the filter placeholder, Access-style `{db} : Database ({format})` window title, Ctrl+S/Ctrl+W/Esc shortcuts and Alt key tips derived from the spec ids) — all strings and logic computed in core, paint code renders only |
| **Datasheet & design view** | Real fixture databases open through the composition root; virtualized datasheet over cached rows with Access-style cell rendering, drag-resizable column widths, header-click sort through the QBE sort model (ascending/descending/clear), alternating row shading, a cursor-row marker and the new-record row (typed cell parsing, `InsertRow` through the bus); keyboard record navigation (a row-selector click arms the grid, then arrows walk records, Home/End jump to the ends, PageUp/PageDown by the viewport, and the virtualized window follows the cursor — step 40.1); horizontal body scrolling with freeze columns (the header context menu freezes and unfreezes leading columns, frozen columns and the row selector stay pinned in header and body while the rest scrolls, and every placement is pure core math — step 40.2); record deletion (the row selector's context menu and the Del key on the selected record dispatch exactly one confirmed `DeleteRow`, capability-gated with a disabled reason on read-only sources — step 40.3); in-place cell editing (a click into a cell of the selected record arms a real in-cell editor, Enter or a click-away commits exactly one `UpdateRow` with the full typed value list, Escape discards, a mistyped cell keeps the editor with the precise field-named failure, read-only sources keep the cells inert — step 40.4); design grid with Access type names, a full property sheet and design-time editing (add/rename/drop a field, toggle required — one command per edit, row data preserved); header-menu sorting and column filters over the core view state (sort caret on the header, a criteria dialog per column, and a status-bar chip naming the active filter count that clears with one click — steps 44.5/45.1) |
| **Headless automation** | `datacraft-mcp` — a stdio JSON-RPC 2.0 endpoint agents spawn; the headless E2E harness drives the full stack over the wire against real fixtures; `datacraft-cli` (open/exec/repl) as the fourth command-bus client with deterministic exit codes (0 ok, 1 usage, 2 command failed), successes on stdout and failures/usage on stderr, process-tested against the real binary |
| **Forms** | WYSIWYG designer with snap-to-grid drag, the Access toolbox palette (Label/Text Box/Check Box/Combo Box/Button/Subform), the designer field list (record-source selector plus one-click bound controls at the next free slot), whitelisted data binding, runtime record navigation, and sidecar persistence (path defaults to the database's `.datacraft`, versioned and database-name checked, save/load reported in the status bar) |
| **Expression service & report bands** | Access-style expression evaluator (field references, arithmetic/concatenation, three-valued Jet NULL semantics, `Like` wildcards, IIf/Nz/Format and the date/string/math families); field wiring: default values fill `Null` cells on insert, validation rules run on insert/update (only `True` accepts), calculated fields are engine-computed; `RunExpression` evaluates over the bus/MCP/CLI; report preview renders group header/detail/footer bands with `=Sum/Count/Min/Max/Avg` aggregate controls |
| **Native saved queries** | `MSysQueries` decodes into read-only query definitions (all nine Access flavors; the supported attribute subset carries tables/columns/joins/criteria verbatim); native queries appear in the navigation pane's Queries group next to sidecar queries, open read-only, and **execute over the bus** when they stay inside the qualified dialect: select queries over one or more tables with equality joins (inner/left/right), Access boolean WHERE expressions (`And`/`Or`/`In`/`Is [Not] Null`/`Like`/`Between`, brackets and double-quoted text), `Table.*` projection and ORDER BY. Other flavors, GROUP BY/HAVING, parameters, DISTINCT/TOP/PERCENT, computed columns and self-joins answer a precise `Unsupported`; a decode failure still opens the database and is carried as `native_queries_error` in the snapshot, shown at the Queries group instead of silently yielding an empty list |
| **Macros (read-only inventory)** | Named macros decode from `MSysObjects` plus the `MSysAccessStorage` `Scripts` tree: name, decoded-definition flag and the readable definition arguments in storage order (expressions, embedded SQL, form/report names) — visible in the navigation pane's Macros group, in `inspect()` snapshots and therefore to MCP/CLI agents; no execution semantics (ADR-2), Jet 3 definitions remain name-only |
| **VBA extraction & viewer** | Spec-driven `[MS-OVBA]` source extraction from `MSysAccessStorage` (`dir` + module streams, dynamic copy-token decompression, project code page): 8 modules with source on the self-created programmability fixture, empty jackcess templates included; the navigation pane's Modules group opens a read-only viewer with module tree, source and full-text search; `LoadVbaModule` returns one module over the bus/MCP — extraction only, no interpreter (ADR-2) |
| **VBA static analysis (headless)** | `AnalyzeVba` returns a deterministic, MCP-friendly analysis of the extracted project: call graph (qualified `Module.Procedure` calls, `New Module` instantiation, unambiguous bare procedures), embedded SQL string literals, table/query references by schema name and unused standard/class modules — conservative and documented as coarse, never executed. The VBA viewer carries an Analyze affordance dispatching the same command and renders every section as a read-only list in an analysis pane (step 41.3) |
| **Reports** | print-geometry designer (A4 page with margin guides) sharing the form canvas and designer palette, with a grouping picker over the record source's fields (step 41.4); print-preview runtime paginating over source rows (grouped reports add group header/footer bands) |
| **SQL over all sources** | A `Database Tools > SQL` window runs one `SELECT`/`WITH` statement through the command bus (`RunSql`/`SqlRan`): the loaded Jet/ACE or SQLite tables are materialized into a throwaway in-memory SQLite database, so joins and aggregates work across every source without touching the file; the gate rejects every other statement and multi-statement input. The dialect is SQLite semantics, stated in the window and pinned by tests (step 39.2): Currency materializes unscaled, `LIKE` uses `%` wildcards (not the Access `*`), text literals never match numeric columns, and integer overflow promotes to REAL. The `Database Tools > Expression` window evaluates one Access-like expression through the bus (`RunExpression`) and renders the result value or the precise failure; without a row context it evaluates over an empty scope (step 41.2) |
| **Relationships & QBE** | Relationship mapper window fed by the driver, with in-pane create (four pickers over the snapshot's schema) and per-row delete affordances (step 41.1); `CreateRelationship`/`DeleteRelationship` are ordinary bus commands (GUI, CLI, MCP): Jet writes `MSysRelationships` rows plus the parent unique/child plain indexes, SQLite rebuilds the referencing table with a `FOREIGN KEY` (existing orphans and duplicate parent values are refused precisely) and enforces it; the capability flips to `Yes` for Jet/SQLite while CSV stays `No`. Access-style QBE grid (table/field/sort/show/criteria rows, schema-whitelisted, persisted in the sidecar); in-memory query engine with typed predicate coercion, AND/OR composition and stable multi-key sorts — no SQL parser. The criteria forms are fenced precisely (step 39.3): `Like` works on text columns through the Access-wildcard matcher (case-insensitive), while `Between`/`In`/`Is Null`/`Not Like` are recognized and answered with a precise unsupported reason instead of degrading to equality; text equality compares case-sensitively |
| **Import File** | `External Data > Import File` turns an external CSV file into a table of the open Jet4/SQLite database over one bus chain (`ImportFile`): the window previews the field names with the L7-inferred column kinds and the leading rows, the target name defaults to the file stem, and create + row inserts run through the ordinary write path (capability gates, validation, defaults and rules apply per row) — the source file stays byte-identical (step 58.1/58.2) |
| **Sample databases & templates** | `CreateDatabase` builds brand-new Jet4 and SQLite files through the bus, `CreateDatabaseFromTemplate` materializes the shipped registry (blank, contacts, inventory, Bike Shop demo with a `.datacraft` sidecar that auto-loads on open and a SQLite twin); the backstage gains New Database and Samples pages, and the deterministic generator (`make_samples --check`, `samples/manifest`) byte-compares the shipped set on every CI run |
| **CI quality gates** | Coverage ≥ 95 % lines, every `.rs` file < 500 lines, zero `TODO`/`FIXME`/`HACK`/`XXX`, `rustfmt --check`, `clippy -D warnings`, `rustdoc -D warnings`, the crate dependency-boundary guard (acyclic graph; only the binary roots name storage/csv), a CLI smoke on every matrix OS, test matrix on ubuntu-latest, windows-latest, macos-latest |
| **Docs & process** | Architecture blueprint, Jet/ACE binary-format knowledge base, testing policy and the clean-room playbook |

The honest picture, phase by phase, is in
[docs/ROADMAP.md](docs/ROADMAP.md); the format facts are catalogued in
[docs/FORMAT-JET.md](docs/FORMAT-JET.md).

## Compatibility matrix (verified 2026-11-07)

Every compatibility claim below names its evidence; unsupported paths are
stated in the same row. Re-verified 2026-11-07 for the engine-accepted
create path (phase 49.2, item M63): the Jet engine accepts DataCraft-created
databases and resolves the tables DataCraft creates. **Read** means decode through the `StoragePort`;
**write** means the path is exercised against a real file in the named
suite. Writes are verified as Access-readable and usage-map-consistent —
byte-identical output to Microsoft Access is **not** claimed.

| Format / path | Read | Write | Evidence | Known limits |
| --- | --- | --- | --- | --- |
| **Jet3 `.mdb`** | ✅ tables, TDEF chains, jump-table rows, overflow/LVAL, embedded OLE, RC4 password | ⬜ | `jet3.rs`, `jet3_remainders.rs`, `jet_encryption.rs` against `testV1997.mdb` / `test2V1997.mdb` | fixture-tested only (no libmdb oracle); no write path; Jet3 VBA/macro definitions stay name-only |
| **Jet4 `.mdb`** | ✅ tables, catalog, rows, OLE/memo, relationships, native queries, index metadata, RC4 password | ✅ create table, insert, update, delete, drop table, design-time alter (add/drop/rename/required), field indexes (metadata + uniqueness), relationship create/delete, journaled batches, LongInteger AutoNumber counter; created files and their tables are **engine-accepted** (2026-11-07 DAO oracle: `TableDefs` enumerates them, rows read and written through the engine, engine-side CREATE/INSERT/DROP round-trip on the same file) and **user-verified in real Access** (2026-11-07 walkthrough: opens without repair, template rows read, Access and DataCraft writes interleave with the autonumber continuing across engines, schema create/drop round-trips both ways) | `jet_table.rs`, `jet_catalog.rs`, `native_queries.rs`, `jet_encryption.rs`; `jet_writer*.rs`, `jet_port_writes.rs`, `jet_row_writes.rs`, `jet_autonumber.rs`, `jet_table_drop.rs`, `jet_alter_table.rs`, `jet_index_writes.rs`, `jet_create_shape.rs`; oracle + `mdb-export` CI plus the Windows DAO open/read/write oracle (phase 49.2) | writer verified on Jet4 `.mdb` only; created tables carry the engine's default ACL and the observed index-leaf maintenance, so catalog resolution works; Memo/OLE/Binary index keys on existing physical indexes answer `Unsupported` and leaves are rebuilt, never split (multi-page indexes stay roadmap M15); table drops free the dropped table's index pages (M66 contract test); engine-refused databases (e.g. pre-49.2 create-path output) are recovered by re-encoding through the create path (`examples/recover_mdb.rs`, 49.4 — `SaveDatabaseAs` copies pages and cannot repair, M67); MSysQueries attribute 12 and unmapped constructs answer `Unsupported` (the open carries `native_queries_error`, surfaced at the Queries group) |
| **ACE `.accdb`** | ✅ 2007/2010/2016 tables, complex values, named macros, VBA project | ✅ create table, insert, update, delete, drop table, design-time alter (add/drop/rename/required), field indexes (metadata + uniqueness), relationship create/delete on ACE 2007/2010 | `ace_fixtures.rs`, `complex_values.rs`, `macros.rs`, `vba.rs`; oracle on three template fixtures; `ace_writer.rs`, `ace_port_writes.rs`, `server_e2e_ace.rs` (phase 25) | no external ACE oracle (mdb-tools reads no `.accdb`) — the write reference is decode∘op∘decode against the pristine pinned bytes; office-encrypted files answer `Unsupported` at the reader and writer boundary |
| **Access co-existence** | ✅ a sibling Access lock file (`.ldb` / `.laccdb`) is detected at open — the session opens **read-only** with the lock file named in the snapshot (CLI/MCP data), the status bar and the Info page | ⬜ read-only while the lock file exists | `jet_capabilities.rs` (locked-session caps + precise `Unsupported` on every write), `driver_caps.rs` (the snapshot data), `ui_layout.rs` / `ui_kittest.rs` (localized notices); FORMAT-JET §Access co-existence | DataCraft never creates or deletes lock files; detection is existence-only — the lock protocol's contents are clean-room-unclear (M59); close Access and remove the lock file to edit here |
| **SQLite** | ✅ catalog, rows, foreign keys | ✅ create/drop table, insert/update/delete, `CREATE [UNIQUE] INDEX` from `IndexMode`, foreign-key create/delete (transactional table rebuild, enforced) | `sqlite_port.rs`, `sqlite_writes.rs`, `sqlite_row_writes.rs`, `sqlite_indexes.rs` | separate backend behind the same command vocabulary; schema alteration (and therefore index changes after create) remains unsupported |
| **CSV directories** | ✅ catalog, rows | ✅ create directory, create/drop table, insert/update/delete (one atomic rewrite per call) | `crates/datacraft-csv` (`tests/driver.rs`, `tests/error_paths.rs`, `tests/inference.rs`), `server_e2e_csv.rs` (phase 30, 36) | column kinds inferred at open (all-integer → Long Integer, all-float → Double, all-ISO-date → Date/Time; one quoted cell or any ambiguity keeps Short Text; values read as text); no schema alteration, relationships, native queries, macros or VBA; no engine-assigned AutoNumbers |
| **Design layer (ADR-1)** | sidecar `.datacraft` forms, queries, reports; native saved queries read-only | ✅ sidecar save/load; sibling sidecar auto-loads on open (errors surface as `sidecar_error`); ⬜ native form/report writes | `schema_document.rs`, `canvas.rs`, `form_runtime.rs`; `native_queries.rs`; `sidecar_auto_load.rs`, `server_e2e_samples.rs` (phase 29) | native Access forms/reports are neither read nor written; the sidecar is versioned and database-name checked |
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
| core | `datacraft-core` | domain model, schema, command bus, query engine, application state, snapshots, JSON-RPC transport (Phases 3–4, 7) |
| jet | `datacraft-jet` | clean-room pure-Rust Jet3/Jet4 read + Jet4/ACE write backends behind the core-owned `StoragePort` seam, libmdb oracle (`cfg(test)` only) (Phase 2, 10; split in Phase 31) |
| sqlite | `datacraft-sqlite` | SQLite backend behind the same seam (rusqlite, statically bundled) (Phase 13; split in Phase 31) |
| csv | `datacraft-csv` | CSV directory driver behind the same seam — a directory of `*.csv` files is one database (ADR-7, Phase 30) |
| storage | `datacraft-storage` | driver composite: format-detecting registry over the shipped backends plus the deterministic samples generator (Phase 29–30) |
| ui | `datacraft-ui` | eframe/egui desktop shell — binary `datacraft` (Phase 5+) |
| cli | `datacraft-cli` | the fourth command-bus client — binary `datacraft-cli` (Phase 9) |
| mcp | `datacraft-mcp` | headless stdio JSON-RPC 2.0 endpoint — standalone binary crate (Phase 9, own crate since Phase 31) |

```text
datacraft-ui (egui) ───────► datacraft-core (command bus) ──► datacraft-storage (composite)
datacraft-cli (shell) ──────┘            ▲                        ├─ datacraft-jet (Jet/ACE)
JSON-RPC / MCP (datacraft-mcp) ──────────┘                        ├─ datacraft-sqlite (SQLite)
                                                                  └─ datacraft-csv (CSV directories)
```

Dependency direction is one-way and review-enforced: `jet → core`,
`sqlite → core`, `csv → core`, `storage → jet, sqlite`, `core → nothing`;
the three client binaries (`datacraft-ui`, `datacraft-cli`,
`datacraft-mcp`) name `core + storage + csv` and are the composition
roots that build the composite. Library and UI code never touches a
driver directly — every client speaks the same command language, and a
ribbon button dispatches the identical `Command` as an MCP tool call.
The crates.io publishing order, the Semver/MSRV policy and the release
checklist live in [docs/PUBLISHING.md](docs/PUBLISHING.md). The full
blueprint — command API shape, test-ID conventions, storage decision
record, rendering strategy — is in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

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

**Scope note:** compatibility is bought ring by ring — expression service →
query engine → macros → forms runtime. DataCraft implements **no VBA
interpreter or runtime**; VBA is extracted, displayed and statically analyzed
only (Phase 17, ADR-2). The ring model and rationale are in
[docs/ROADMAP.md](docs/ROADMAP.md); the verified claim set is the
[compatibility matrix](#compatibility-matrix-verified-2026-11-07).

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
