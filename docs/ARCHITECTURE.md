# DataCraft — Architecture Blueprint

> Step 1.1 (item K2). Living document — updated at every architectural change.
> Language: English (binding, see `docs/PLAYBOOK.md` §2).
> Clean-room project: no Microsoft code, no MDB Tools code; public
> specifications and own observation only (see §12).

## 1. Mission

DataCraft is an open-source, cross-platform alternative to Microsoft Access:
verified reading of `.mdb` (Jet 3/4) and `.accdb` (ACE) files with a verified
Jet 4 write path for tables and rows — every compatibility claim is bounded by
the matrix in `README.md` — the classic Access UX paradigm (Ribbon,
Navigation Pane, tabbed documents, Datasheet/Design views, QBE query builder,
Form/Report canvas) rendered in an original, minimal, hardware-accelerated
ArtCraft aesthetic — and a headless core that AI agents can drive exactly like
the GUI does.

## 2. Principles → mechanisms

| # | Principle (binding) | Concrete mechanism | Enforced by |
|---|---------------------|--------------------|-------------|
| 1 | Clean-room reverse engineering | Format knowledge only from public specs + own black-box observation; `libmdb` used exclusively as test oracle | `docs/LESSONS.md` #12; storage decision §6; High content gate |
| 2 | Visual originality | No Microsoft assets of any kind; original egui vector iconography; Access UX paradigm, ArtCraft styling | Review; no MS binary assets in repo |
| 3 | MCP & headless automation | Every GUI action = one `Command`; UI is a thin projection of the headless core; JSON-RPC/MCP transport over the same command bus | §5; crate graph rules §4 |
| 4 | Autonomous testability | Deterministic test IDs (`ui_test_id()`), state query API returning serializable snapshots, no pixel assertions | `datacraft-core::ObjectId`; Phases 4/9 |
| 5 | Cross-platform portability | No COM/OLE DB/ADO/winapi anywhere; pure Rust + portable FFI (test-time only) | CI matrix Linux/macOS/Windows; `unsafe_code = forbid` |

## 3. System overview

```text
┌─────────────────────────────────────────────────────────────────┐
│                            Clients                              │
│  ┌───────────────┐  ┌────────────────┐  ┌────────────────────┐  │
│  │  egui shell   │  │ JSON-RPC / MCP │  │  headless test     │  │
│  │ (datacraft-ui)│  │ server (Ph. 9) │  │  harness (Ph. 9)   │  │
│  └───────┬───────┘  └───────┬────────┘  └─────────┬──────────┘  │
│          │ Command          │ Command             │ Command     │
│          ▼                  ▼                     ▼             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ datacraft-core — command bus, application state, pure     │  │
│  │ reducers, UI-state snapshots, deterministic test IDs      │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │ StoragePort seam (ADR-7)          │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │ datacraft-storage (composite)                             │  │
│  │  ┌─────────────────────────────┐   ┌───────────────────┐  │  │
│  │  │ datacraft-jet (pure-Rust    │   │ datacraft-sqlite  │  │  │
│  │  │ Jet/ACE reader & writer)    │   │ (rusqlite,        │  │  │
│  │  │ libmdb oracle: test-time    │   │  statically       │  │  │
│  │  │ only, LGPL, cfg(test))      │   │  bundled)         │  │  │
│  │  └─────────────────────────────┘   └───────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

All three clients speak the same command language. The GUI is **not** privileged:
a ribbon button and an MCP tool call dispatch the identical `Command` and
produce identical state transitions. That is what makes the GUI verifiable
headlessly (Principle 4) and the tool agent-operable (Principle 3).

## 4. Workspace & crate graph

```text
crates/
├── datacraft-core/      domain model, command bus, app state, test IDs
├── datacraft-jet/       clean-room pure-Rust Jet/ACE backend (Ph. 2+, split 31.2)
├── datacraft-sqlite/    SQLite backend, rusqlite statically bundled (Ph. 13, split 31.2)
├── datacraft-storage/   driver composite: detection + samples (Ph. 30)
├── datacraft-csv/       CSV directory driver — ADR-7 out-of-tree proof (Ph. 30)
├── datacraft-mcp/       headless stdio JSON-RPC binary (Ph. 9; own crate since 31.3)
├── datacraft-cli/       headless CLI client binary (Ph. 9)
└── datacraft-ui/        eframe/egui desktop shell (lib + bin `datacraft`)
```

`datacraft-ui` builds as a library with a thin `main` so the headless
`egui_kittest` suite (`tests/ui_kittest.rs`, T5) can drive the same widgets
through AccessKit; the layer boundary is unchanged — paint code only, all
decisions in core.

Dependency direction rules (review-enforced, never cyclic):

```text
datacraft-ui      ──► datacraft-core, datacraft-storage, datacraft-csv
datacraft-cli     ──► datacraft-core, datacraft-storage, datacraft-csv
datacraft-mcp     ──► datacraft-core, datacraft-storage, datacraft-csv
datacraft-storage ──► datacraft-jet, datacraft-sqlite, datacraft-core
datacraft-jet     ──► datacraft-core   (clean-room Jet/ACE backend)
datacraft-sqlite  ──► datacraft-core   (rusqlite backend)
datacraft-csv     ──► datacraft-core   (out-of-tree driver, ADR-7)
datacraft-core    ──► (nothing internal)
```

Out-of-tree drivers depend only on `datacraft-core` and are composed at
the binary composition roots via `DetectingPort::with_fallback` (ADR-7,
§16) — the storage crate never names them.

- Library and UI code never touches a driver directly; storage access
  happens exclusively through core commands. Only the binary targets
  (`crates/*/src/main.rs`, `crates/datacraft-ui/src/bin/*`) name
  `datacraft-storage` and `datacraft-csv` as the composition roots.
- No async runtime (tokio & co.). The egui event loop drives the process;
  background work uses `std::thread` + channels behind core ports.
- `[workspace.dependencies]` pins every external version once;
  `[workspace.lints]` defines shared lints (`unsafe_code = "forbid"`).

## 5. Headless core & command API (Principles 3 + 4)

Implemented in Phase 4 and extended phase by phase since; this section
pins the shape, not the variant list. The authoritative command
vocabulary lives in `datacraft-core/src/command.rs` (the enum is
`#[non_exhaustive]` per M22 — clients match with a wildcard arm), the
event vocabulary in `event.rs` and the serializable state surface in
`snapshot.rs`. Row writes, undo/redo, save-as/password, relationships,
templates/samples, capability-gated dispatch (ADR-7) and the UI-parity
commands of phases 38–42 all speak it.

```rust
/// Every user-visible action, GUI or agent, is exactly one command.
pub enum Command { /* the authoritative list is command.rs */ }

/// Observable outcome of a dispatch; the UI re-renders from state, agents assert on events.
pub enum Event { /* DatabaseOpened { .. }, TabOpened { .. }, .. */ }

impl Core {
    /// Pure reducer over application state; I/O happens behind ports (StoragePort, fs).
    pub fn dispatch(&mut self, command: Command) -> Vec<Event>;
    /// Serializable snapshot of everything the UI shows — the headless test seam.
    pub fn inspect(&self) -> UiStateSnapshot; // serde::Serialize
}
```

**Test-ID convention** (deterministic, dot-separated, stable across releases):

```text
ribbon.<tab>.<group>.<action>     e.g. ribbon.create.tables.table_design
qat.<action>                       e.g. qat.save_design
backstage.<page>.<control>         e.g. backstage.open.open_database
navpane.<state>.node.<kind>.<name> e.g. navpane.expanded.node.table.customers
workspace.tab.<object-id>          e.g. workspace.tab.table.customers
datasheet.<table>.cell.<row>.<col> e.g. datasheet.customers.cell.12.name
qbe.grid.<row>.<col>               e.g. qbe.grid.criteria.3
form.canvas.control.<name>         e.g. form.canvas.control.txt_name
```

Domain identifiers carry the scheme: `ObjectId::ui_test_id()` renders
`navpane.<state>.node.<kind>.<name>` so the navigation pane and the tab bar
derive their IDs, never invent them. Headless tests assert on
`UiStateSnapshot` JSON — never on pixels (Principle 4).

## 6. Storage engine — decision record

**Decision (2026-10-31, human-approved):** the Jet/ACE read **and** write paths
are implemented in pure Rust (clean-room). `libmdb` (MDB Tools) is used
**only as a test-time oracle** behind `#[cfg(test)]`/feature gates; release
builds never link it.

| Criterion | libmdb FFI (runtime) | Pure Rust engine |
|---|---|---|
| `.mdb`/`.accdb` read | mature | built by us, spec-based |
| **write** | practically absent — no accurate Access-compatible writer | **only viable route to Access-compatible writes** (verified claim set: README matrix) |
| license | LGPL-2.1, dynamic linking only | none (product code) |
| cross-platform build | GLib dependency, vcpkg/MSYS2 pain | `cargo build` everywhere |
| correctness confidence | battle-tested reader | oracle cross-check in CI replaces that |

**Alternatives considered:** (a) libmdb runtime backend — fastest read path,
but no write path and a second implementation of the same boundary later;
(b) pure Rust without oracle — loses a free, battle-tested correctness
reference. Rejected for the reasons above.

**Consequences:** Phase 2 is longer than an FFI shortcut; LGPL stays confined
to dev/CI; every Jet 4/ACE read milestone is proven by (1) golden-file
fixtures and (2) strict schema/row comparison against the oracle. Jet 3 has
fixture coverage only (no libmdb oracle). The verified claim set and its
limits live in the README compatibility matrix.

**Trait boundary** (implemented in Phase 2.1):

```rust
pub trait StorageEngine: Send {
    fn open(&mut self, path: &Path) -> Result<DatabaseMeta, StorageError>;
    fn schema(&self) -> &[TableSchema];
    fn read_rows(&self, table: &str, range: RowRange) -> Result<RowBatch, StorageError>;
    fn write_rows(&mut self, table: &str, ops: &[RowOp]) -> Result<WriteReport, StorageError>;
    fn flush(&mut self) -> Result<(), StorageError>;
}
```

No Jet/ACE-specific types leaked across that boundary — the core worked
against `TableSchema`/`RowBatch` domain types, and backend selection was
feature-gated (`jet-rs` product, `libmdb-oracle` test). Both mechanisms are
historical: today the oracle exists only behind `#[cfg(test)]` and product
composition is the driver registry below.

Since Phase 13 the same port also fronts a **SQLite** backend, and since
Phase 30 the **CSV** directory driver docks the same way (ADR-7):
`DetectingPort` sniffs the file magic (or the directory shape) at `open` and
delegates to the registered driver — Jet/ACE, SQLite or CSV — so all clients
keep one command vocabulary for every source family. `InsertRow`/`SqlRan`
are port-level writes and the shadow-SQL engine reads every source through
the port (see `docs/FORMAT-JET.md` §"SQLite backend").

The `StorageEngine` snippet above is the historical Phase-2 record; the
living boundary is the core-owned `StoragePort` trait. The Phase-2 trait —
with its in-memory reference backend and the `engine_boundary` suite — was
**retired in Phase 31 (M21)**: one seam only (`StoragePort`, ADR-7). The
contract assertions live on as the cross-driver `tests/port_contract.rs`
suite and the per-driver capability invariants (`jet_capabilities`,
`sqlite_capabilities`). Since Phase 30 the driver
model — declarative capabilities, registry detection, out-of-tree driver
crates — is governed by ADR-7 (§16).

**Format milestones:** JET4 (`.mdb`) read → ACE (`.accdb`) read → JET4 write →
ACE write. Each milestone: spec → decoder/encoder → proptest round-trip →
golden files → oracle comparison. Landed so far: Jet4 read, Jet3 (Access 97)
read incl. chained TDEFs, >255-byte jump-table rows and overflow/LVAL pages,
embedded-OLE columns (fixed 0x11 long binary), ACE read and complex values
(attachments, multi-value, version history; Phase 16), Jet3/Jet4 RC4 database
passwords and page encryption (Phase 16; ACE Office encryption stays a precise
`Unsupported`), the Jet4 writer core incl. update/delete, drops, design-time
alter, index/relationship creation and journaled batches (tables + rows,
usage-map-consistent; Phases 10–11, 24, 32), the ACE write path (Phase 25),
the SQLite read/write backend (Phase 13; indexes/foreign keys in 32), the CSV
directory driver (Phase 30), the Phase 17 read-only programmability layer
(named macros, VBA extraction/viewer/static analysis), native saved-query
decode and the qualified-dialect execution (Phases 15, 27) and encrypted
Save-As with password change (Phase 27). Remaining write-path work:
LVAL-chain writes for complex values, physical Jet index pages (the writer
carries index metadata + uniqueness so far), the ReplicationId AutoNumber
flavor, Jet's tri-state Boolean NULL (documented rejection, M24) and the
manual Access-repair gate of the shipped samples. The README compatibility
matrix is the claim set.

## 7. Domain data model (delivered in Phase 3)

Access type mapping (native Jet/ACE types ↔ DataCraft domain types):

| Access UI type | Jet/ACE storage | DataCraft domain |
|---|---|---|
| Short Text | text, fixed/variable, UTF-16 | `Text { max_len }` |
| Long Text | memo pages (multi-page) | `Memo` |
| Number (Byte/Integer/Long/Single/Double/Decimal) | 1/2/4/4f/8f/16-byte numeric | `Number(NumericKind)` |
| Date/Time | OLE Automation DATE (f64 days since 1899-12-30) | `DateTime` |
| Currency | scaled 64-bit integer (10^-4) | `Currency` |
| AutoNumber | Long/Integer + counter or GUID | `AutoNumber(AutoKind)` |
| Yes/No | 1 byte bitfield | `Boolean { tri_state }` |
| OLE Object / Attachment | multi-page binary + header maps | `Binary { kind }` |

Field properties (Input Mask, Format, Default Value, Validation Rule/Text,
Required, Allow Zero Length, Indexed, Caption) are modeled as a typed property
map with Access-compatible semantics — validated per type in Phase 3.

**Decision record (ADR-1, 2026-10-31 — resolves BACKLOG item M1):** persistence
of the form/report design layer. Real `.accdb` files store forms/reports in
binary system tables; until the ACE write path exists, DataCraft persists its
design layer (forms, reports, control layouts, UI state) in an **own,
versioned sidecar document** (serde JSON, `.datacraft` next to the database
file, schema version field, forward-migration rule). Native accdb embedding
becomes available only when the ACE writer lands; the sidecar format stays the
portable fallback and is what MCP agents script against.

**ADR-1 addendum (2026-11-03 — resolves BACKLOG item H45):** the sidecar is
also the **authority for field properties**. The Jet/ACE writer leaves
`MSysObjects.LvProp` `NULL` (the file stays Access-readable); the full
`FieldProperties` sheet lives in the sidecar's `tables` list and is overlaid
onto the physical catalog on open and after design-time schema refreshes.
A stale table/field or a property that does not fit the live kind is a
precise error (explicit load) or `sidecar_error` (auto-load), never a silent
loss. Encoding the `MR2` property pack into `LvProp` so Access itself shows
defaults/captions is a separate, deferred storage-format item
(`docs/ROADMAP.md` §6).

## 8. Rendering & performance

- egui immediate mode; **no layout or business logic in paint code** — layout
  computations are pure functions in core (unit-testable, Principle 4).
- Shared grid core (Phase 6): virtualized painting of visible cells only,
  zero-copy row slices, columnar value cache — the GridCraft philosophy.
- Target: 60 FPS with million-row virtual scrolling; no per-frame allocations
  on the hot path (reuse `Vec`/`String` buffers).
- In-code iconography: egui `Painter` vector primitives; the backstage/chrome
  nav glyphs adapt the sibling Crafting Apps' in-code set (GridCraft/WordCraft,
  MIT OR Apache-2.0, attributed in `THIRD-PARTY-NOTICES.md`) — no Microsoft
  assets (Principle 2).

## 9. Test strategy

Details and the coverage-exception registry live in `docs/TESTING.md`.
Layers: colocated unit tests → crate integration tests → `proptest`
(property-based parser/round-trip tests) → golden-file tests against committed
fixture databases → libmdb oracle comparison (`cfg(test)`) → headless E2E over
the command bus (Phase 9). Mechanical gates in CI: coverage ≥ 95 % lines,
all `.rs` files < 500 lines, `clippy -D warnings`, `rustfmt --check`,
test matrix on Linux/macOS/Windows.

## 10. Module & file rules

- Every `.rs` file < 500 lines (CI gate; design target < 400).
- One module per concern; parser modules split by page type, UI modules by
  component (ribbon, nav pane, workspace, datasheet, design grid, QBE, canvas).
- Tests colocated (`#[cfg(test)] mod tests`) for units; cross-crate behavior
  in `crates/*/tests/`.
- `unsafe_code = "forbid"` workspace-wide; the eventual FFI oracle module is
  the only scoped exception, feature-gated and confined (LESSONS #11).

## 11. Repository layout

```text
datacraft/
├── Cargo.toml                  workspace manifest (single version source)
├── rust-toolchain.toml         pinned toolchain (H3)
├── crates/
│   ├── datacraft-core/         lib — domain, commands, state
│   ├── datacraft-csv/          lib — CSV directory driver (ADR-7)
│   ├── datacraft-jet/          lib — clean-room Jet/ACE backend (pure Rust)
│   ├── datacraft-sqlite/       lib — SQLite backend (rusqlite bundled)
│   ├── datacraft-storage/      lib — driver composite (detection + samples)
│   ├── datacraft-cli/          bin — headless CLI client
│   ├── datacraft-mcp/          bin — headless stdio JSON-RPC endpoint
│   └── datacraft-ui/           lib+bin — eframe desktop shell
├── fixtures/                   reference .mdb/.accdb databases (Ph. 2)
├── docs/                       PLAYBOOK, LESSONS, ARCHITECTURE, TESTING
│   └── archive/                stepwell append-only archives
├── .github/workflows/ci.yml    quality gates (High-class file: changes need human release)
├── BACKLOG.md                  open items only
└── PROGRESS.md                 phase table + active phase blocks
```

## 12. Licensing & clean-room policy

- DataCraft code: MIT OR Apache-2.0 (dual-licensed; finalized with the skeleton).
- `libmdb` (MDB Tools): LGPL-2.1 — test-time only, never linked into release
  binaries, attributed in README/NOTICE.
- Clean-room rules (binding): sources of format knowledge are public
  specifications (Microsoft Open Specifications, e.g. the ACCDB binary format
  documents) and community format documentation — **never** MDB Tools source
  code, never decompiled Microsoft binaries. Review findings quoting internal
  upstream code are a High-class gate (LESSONS #12).

## 13. Phase mapping

The live status of every step is `PROGRESS.md`; completed phase detail
blocks live in `docs/archive/PROGRESS_ARCHIVE.md`. This table maps each
phase to what it delivered:

| Phase | Delivers |
|---|---|
| 1 — Blueprint & Skeleton | this document, workspace, CI gates, testing policy |
| 2 — Storage Engine Core | Jet page layer, TDEF/TDAT decoder, libmdb oracle (the trait snippet of §6's historical record) |
| 3 — Data Model & Schema | Access data types, field properties, serde persistence |
| 4 — Headless Core & Command API | command bus, state query, test IDs, JSON-RPC stub |
| 5 — UI Skeleton (egui) | app shell, ribbon, navigation pane, tabbed workspace |
| 6 — Datasheet & Design View | shared grid core, datasheet, table design view |
| 7 — QBE Query Builder | relationship mapper, QBE grid, query engine |
| 8 — Form & Report Canvas | WYSIWYG canvas, Access control types, form runtime |
| 9 — MCP Endpoint & Automation | MCP/JSON-RPC server, headless E2E harness, CLI client, release prep |
| 10 — Jet4 Writer, Reports & Release | Jet4 writer workstream, Jet3 read, report designer + print preview, fixture pin integrity |
| 11 — Port Write-Through & Journal | insert/update/delete through the port, journaled write profiles |
| 12 — Access Look & Theme | Access ribbon/nav-pane/status/datasheet styling over the token system |
| 13 — SQLite & SQL | SQLite source backend, SQLite writes, SQL window |
| 14 — ACCDB Read | ACE fixtures and the ACE read path |
| 15 — Expressions & Saved Queries | expression service, field wiring, `MSysQueries` decode |
| 16 — Complex Values & Encryption | attachments/multi-value/version history, RC4 passwords, Jet3 remainders |
| 17 — Macros, VBA & Docs Accuracy | read-only macro inventory, VBA extraction/viewer/static analysis, verified compatibility matrix |
| 18–19 — UI Localization & Settings | i18n kernel, complete German catalog, `ui.json`, language switcher |
| 20 — Look & Typography | neutral tokens, Inter/JetBrains Mono typography, original vector iconography, Light/Dark/System appearance |
| 21 — Access UI Alignment | contextual ribbon tabs, quick access toolbar, backstage, datasheet/nav-pane/tab UX, keyboard |
| 22 — Settings Hardening | atomic `ui.json` save, window position/maximized state, off-screen rescue |
| 23 — Hardening & Test Gaps | CLI process E2E, headless kittest UI suite with AccessKit snapshots, native-query decode errors surfaced at open |
| 24 — Jet Writer Remainder | AutoNumber counter maintenance, table drops through the writer, design-time schema editing |
| 25 — ACCDB Write | ACE write contracts, port write-through and bus E2E on `.accdb` |
| 26–29 — Release, Queries, UI Rest, Samples | release engineering, native query execution + encrypted save-as, undo/redo + recent databases, create-database flow, templates and shipped samples |
| 30 — Driver Model | declarative per-session capabilities, registry detection, CSV directory driver, capability-disabled UI affordances (ADR-7, §16) |
| 31 — Publishing & Crate Split | jet/sqlite/mcp as standalone crates behind the composite, publish-ready vocabulary, Semver/MSRV/changelog policy |
| 32 — Schema Writing | persisted field properties, index creation, relationship/foreign-key creation |
| 33 — Journal Capability Gating | journaled batches gate on the declared atomic_batch capability |
| 34 — Cross-Cutting Review | storage/drivers, core, UI and public-API reviews with in-phase fixes |
| 35 — Documentation Sweep | entry-point docs, ARCHITECTURE stale sections, FORMAT-JET/TESTING, matrix re-verification |
| 36 — Driver Edge Suites | CSV/detecting/MCP coverage, driver inventory in the snapshot, CSV type inference, tri-state Boolean NULL decision |
| 37 — Release Readiness | self-created fixtures replace Northwind, fixture rebuild tooling, licenses/attribution, history hygiene |
| 38 — UI Restyle | icon glyph port, backstage restyle, native dialogs behind the seam, window chrome polish |
| 39 — Correctness Fences | named date pictures, SQL-dialect fence, QBE criteria fence, cache load errors surfaced |
| 40 — Datasheet UX | keyboard record navigation, horizontal scroll + freeze columns, row deletion, in-place cell editing |
| 41 — GUI Parity | relationship mapper affordances, expression window, VBA analysis viewer, report grouping picker, ribbon truth pass |
| 42 — Core Correctness | SQLite affinity edge cases, capability-gated design edits, by-name QBE projection |
| 43 — Coverage & CI Pinning | coverage gap audit, cargo-llvm-cov pinned in the CI coverage job |
| 44 — UI Layout & Affordance Audit | content-sized ribbon groups, panel-geometry pins, Close Database, German label semantics, datasheet sort/filter with the core view state, the layout-rect seam |
| 45 — Walkthrough Fixes | column filter dialog replaces the in-menu editor (menus cannot host text editing), record-navigation arrow order |
| 46 — Form Runtime Layout | the runtime renders the designed layout: the shared designer canvas, controls at their stored rects and sizes |
| 47 — Walkthrough Fixes II | activating an object surfaces its contextual ribbon tab (backstage and same-kind switches never yank) |

## 14. Compatibility scope & VBA — decision record

**Decision record (ADR-2, 2026-10-31):** compatibility is bought ring by ring —
expression service → query engine → macros → forms runtime — and DataCraft
implements **no VBA interpreter or runtime**. VBA is extracted, displayed and
statically analyzed only; byte-compatible VBA execution is out of scope with a
defined re-evaluation gate. Ring model, rationale and gate: `docs/ROADMAP.md`
§2–§5.

**Implemented scope (Phase 17, 2026-11-01):** the in-scope part of ADR-2 is
delivered — `MSysAccessStorage` is decoded into the ACE application tree,
named macros become a read-only inventory (name + coarse readable arguments),
the VBA project is extracted per `[MS-OVBA]` (`dir` records, compressed module
streams, project code page) with a read-only viewer, and `AnalyzeVba` returns
the call graph, embedded SQL literals, table/query references and unused
modules over the bus/MCP. Deliberately absent: macro/VBA execution, the
`_VBA_PROJECT` p-code cache, and Jet 3 project extraction
(`MSysAccessObjects` chunking — BACKLOG remainder).

**Decision record (ADR-3, 2026-10-31):** no own scripting language. Automation
is command-bus-first (headless JSON-RPC/MCP + CLI); an embedded *existing*
sandboxed language behind a reserved, unimplemented `ScriptEngine` trait is a
post-Ring-4 decision — never a custom DSL (`docs/ROADMAP.md` §4).

## 15. Localization, settings & look — decision record

**Upstream model reference (2026-11-01):** the WordCraft citations in this
section and in the backlog point at `storytold/wordcraft` @ `fd7f662`; the
named paths, colours, type rungs and mechanics were verified against that
revision.

**Decision record (ADR-4, 2026-11-01): UI localization lives entirely in the
UI layer.** UI settings — the display language first among them — never write
into other areas of the app: `datacraft-core`, the `.datacraft` sidecar and
the wire stay free of language state. Core remains locale-neutral and
English; snapshots and events are byte-identical regardless of the UI
language, so MCP/CLI agents see one stable contract. The translation layer
follows the WordCraft (`crates/ui-egui/src/i18n.rs`) pattern: `t()`/`tr()`
with the English source string as the lookup key, an embedded German catalog,
and a graceful fallback — a missing translation renders the English source.
Language selection: setting `"auto"` (default) → system detection via
`sys-locale` (`de*` → German, anything else → English), overridable by the
`DATACRAFT_LANGUAGE` environment variable. V1 scope is the UI surface
(ribbon, nav pane, views, status bar, toasts); `Event::Failed.reason` texts
stay English until core carries stable failure codes (BACKLOG I6).

**Decision record (ADR-5, 2026-11-01): app settings live in a UI-owned
`ui.json`, strictly separate from the sidecar.** The `.datacraft` sidecar is
per-database design data (ADR-1) and must not absorb app preferences. The
settings file sits at `<platform config dir>/datacraft/ui.json` (serde JSON,
config dir resolved without a new dependency), stores language, appearance and
window geometry, and treats unknown values as defaults
instead of failing. Precedence: explicit setting > `DATACRAFT_LANGUAGE` >
system detection > auto. Writes are atomic — a sibling temporary file renamed
over the target, so an interrupted save can never truncate the file. The
stored geometry is the inner size plus the outer position and the maximized
state; a saved position that lands off-screen is rescued onto the visible
monitor on the first frame (step 22.2, U13). The language and appearance
switchers
sit in the File ribbon group for now because no Options dialog exists yet;
they move into the backstage with U7, whereas the model keeps the language
under File ▸ Options ▸ Interface language.

**Decision record (ADR-6, 2026-11-01): the look rebase adopts the WordCraft
neutral system; the accent family stays DataCraft teal.** Neutral tokens,
geometry (corner radii 8/6/4, 1 px strokes, spacing) and typography (Inter
families, type rungs Chrome 12.5 / Control 11.5 / Caption 11.0 px) come from
the WordCraft `ui-egui` theme as the optical model. **No accent colour is
copied from the model** — its blue is explicitly rejected; every
accent-adjacent token (accent, accent_text, checked/selection, hover wash,
on_accent, and the dark-mode pendants) derives from the brand teal
`#0F8B7E`. Dark mode ships as `Appearance { Light, Dark, System }` (default
System) with WCAG contrast tests against the teal variants. Fonts are openly
licensed (OFL) and attributed. Verified @ `fd7f662`: the model keeps its
palette in a `Tokens` struct with `light()`/`dark()`, and its accent is
`#3B5BDB` (`accent_text` `#2B47B5`; dark `#7A93F0`/`#9CB0F5`) — the blue this
rule rejects. Its token set is larger (`tab_strip`, `status_bar`,
`input_border`, `nav`); DataCraft requires the neutral and accent-adjacent
tokens named above and adds a slot only when a region needs it. Typography:
the model registers Inter Regular/Medium/SemiBold, JetBrains Mono for
monospace and Source Sans 3 as a fallback behind Inter; DataCraft keeps
Inter + JetBrains Mono and lets egui's faces be the fallback. Both palettes
are installed into the light and dark styles, and the tokens are picked
per frame from `visuals.dark_mode`; on Linux, where `system_theme` is `None`,
the model polls the XDG desktop portal every 4 s.

**Implemented scope (Phase 18, 2026-11-01):** the UI surface is localized —
`crates/datacraft-ui/src/i18n.rs` provides `t()`/`tr()`/`plural()` over the
embedded German catalog, split into `i18n/de/ui_chrome.rs` and
`i18n/de/views.rs` to stay below the file-size gate. Every paint-code
literal and every data-dependent label (field kinds from
`FieldSnapshot.kind`, cell text from `FieldValue`, the status line from
snapshot counts) renders through it, while `inspect()` JSON stays
byte-identical in every language (wire-stability test). Parity tests scan
the UI sources in both directions — no untranslated key, no orphaned catalog
entry — and enforce identical placeholder sets; layout tests measure the
German strings against the ribbon and status-bar budgets with egui's real
font metrics. `sys-locale` (already in the dependency tree transitively) is
now a direct UI-only dependency. Deliberately absent: the language switcher
and the `ui.json` settings store (Phase 19) and `Event::Failed.reason`
localization, which waits on core failure codes (BACKLOG I6).

**Implemented scope (Phase 19, 2026-11-01):** the UI-owned settings store
lands in `crates/datacraft-ui/src/settings.rs`:
`<config>/datacraft/ui.json` (`%APPDATA%`, `$XDG_CONFIG_HOME`/`~/.config`,
`~/Library/Application Support`; resolved without a new dependency), parsed
per field so unknown or corrupt values fall back to defaults instead of
failing. It stores the language choice (`auto`/`de`/`en`) and the inner
window size; the File-tab switcher (Auto/Deutsch/English) applies live and
saves immediately, geometry is captured per frame and saved on exit.
Language resolution is an explicit startup step (setting >
`DATACRAFT_LANGUAGE` > detection) — `t()` renders English until it runs,
which keeps headless tests independent of the host locale. The sidecar,
core and wire remain untouched by the settings path. Two deltas from the
model are recorded rather than hidden: the write is a plain write (the model
writes a temporary file and renames it) and the stored geometry is the inner
size only (the model keeps position and maximized state and rescues
off-screen windows) — both under BACKLOG U13. The switcher sits in the File
ribbon group because no Options dialog exists yet; it moves into the
backstage with U7, while the model keeps it under
File ▸ Options ▸ Interface language.

**Implemented scope (Phase 20, 2026-11-01):** the look rebase is implemented
per ADR-6. `crates/datacraft-ui/src/theme.rs` holds `Tokens` with
`light()`/`dark()` and the `Geometry` rungs; `apply` installs both palettes
into the light and dark styles, and `tokens(visuals)` selects the set per
frame. All paint sites — ribbon, navigation pane, tab strip, status bar,
workspace canvas and windows — read tokens; `accent_fill` is the
contrast-checked variant for text-bearing accent surfaces (datasheet header).
WCAG contrast tests cover both palettes. `theme/fonts.rs` embeds Inter
Regular/Medium/SemiBold and JetBrains Mono (SIL OFL 1.1, unmodified,
attributed under `assets/fonts/`), registers the named families
`medium`/`semibold` and installs the type scale; the German layout budgets
are measured against the shipped fonts. `icons/` is one name-keyed painter
dispatcher (section painters `file`, `data`, `objects`) covering every ribbon
action id and `ObjectKind` slug; ribbon buttons, navigation rows and document
tabs paint them, and the AccessKit labels stay intact. `appearance.rs` owns
`Appearance { Light, Dark, System }`, persists it in `ui.json` and applies it
as the egui theme preference; on Linux a `gdbus` portal poll (4 s, tested
parser) makes System follow the OS live. Phase-gate screenshots
(before/after light, after dark) live in `docs/phase-20/`. One defect found
by the gate was fixed inline: the empty workspace now paints the canvas token
instead of showing the window clear colour (BACKLOG F3).

## 16. Driver model & engine packaging — decision record

**Decision record (ADR-7, 2026-11-02): data sources are pluggable drivers that
report their capabilities; the engine layer is the product, the clients are
applications.** Approved with the Phase 30 plan release.

**Decision:**

1. **One driver interface.** Every data source implements the core-owned
   `StoragePort` trait. There is no second, format-specific seam: Jet3/Jet4/ACE,
   SQLite and future sources differ only in their driver implementation and
   their reported capabilities.
2. **Declarative capabilities.** Every driver reports a stable `DriverId` and a
   `DriverCapabilities` value per *opened session* — capabilities depend on
   runtime state (a read-only file disables the write capabilities; an
   Office-encrypted ACE file opens read-only). The vocabulary covers the port
   surface (`read_tables`, `write_rows`, `create_table`, `drop_table`,
   `create_database`, `alter_table`, `atomic_batch`, `passwords`, `autonumber`,
   plus `Support { No, ReadOnly, Yes }` for `relationships`, `native_queries`,
   `macros` and `vba`) and reserves `triggers`/`stored_procedures` (all current
   drivers report `No`). Growing the vocabulary is an additive, versioned
   change. A capability reported `No` is a contract: the matching port call
   answers a precise `Unsupported`.
3. **Registry instead of dispatch.** Format detection is an ordered registry of
   drivers with a `sniff(path, header) -> bool` probe; the composite answering
   port stays the composition-root default. Adding a driver must not require
   edits in core, the UI or the wire vocabulary.
4. **Engine as product, apps as consumers.** `datacraft-core` (command bus,
   port trait, state, snapshots, JSON-RPC transport) is the standalone headless
   engine. The GUI (`datacraft`), `datacraft-cli` and `datacraft-mcp` are
   applications that compose the engine with a driver set at their composition
   roots. Out-of-tree driver crates depend only on `datacraft-core`;
   `datacraft-jet` keeps the clean-room Jet/ACE implementation and its
   oracle. The target packaging (driver-per-crate split, standalone
   `datacraft-mcp` binary crate, Semver/MSRV policy) was decided here and
   realized in Phase 31 (steps 31.2–31.4, BACKLOG M19); it does not change
   this decision. The binding publishing policy is
   [`docs/PUBLISHING.md`](PUBLISHING.md).
5. **UI affordances are core-computed.** Core derives, per open source, an
   availability map (enabled + disabled reason) for command affordances into
   the snapshot; the UI only renders disabled controls with localized reasons
   (`t()`). UI code never matches on formats and never duplicates capability
   logic. Defense in depth: the dispatch consults the capabilities and answers
   the same `Unsupported` events if a stale client still dispatches.
6. **Publish-ready vocabulary (addendum, 2026-11-03, M22).** (a) `driver_id`
   and `capabilities` are **required** trait methods — the former optimistic
   `DriverCapabilities::full()` fallback made a forgetful driver lie about its
   surface at runtime; now the compiler asks for the declaration (rejected:
   a conservative runtime default that silently answers `Unsupported`
   everywhere — a compile error is honest and cheaper to diagnose).
   `DriverCapabilities::none()` is the conservative constructor a minimal
   driver starts from; `full()` stays the optimistic reference set for tests
   and the mock. (b) The wire-facing vocabulary `Support`, `PortError` and
   `Command` is `#[non_exhaustive]` — adding a variant is not a breaking
   change for out-of-tree drivers and clients; they match with wildcard arms.
   (c) `DriverCapabilities` deliberately keeps public, constructible fields
   (a `#[non_exhaustive]` struct would block field *reads* and break every
   out-of-tree driver, contradicting the csv-drives-unchanged proof);
   field additions are shielded by the `none()`/`full()` constructors and the
   Semver policy that both constructors carry a value for every new field
   (formalized in step 31.4).

**Consequences:** the snapshot gains additive `driver`/`capabilities` fields
(wire-compatible); batch atomicity becomes a declared property — the SQLite
default loop is visible instead of silent; `DatabaseFormat` grows one variant
per driver family (CSV in Phase 30); server-hosted sources (`postgres://…`)
are a documented future path that requires a `SourceSpec` extension of the
open commands — deliberately out of scope here.

**Alternatives considered:** (a) runtime `Unsupported` only, no declarative
model — rejected: agents and the UI cannot adapt, and silently non-atomic
batches stay invisible; (b) an out-of-process plugin protocol — rejected for
now: it adds ABI/IPC surface against the no-async, pure-Rust constraints;
in-process out-of-tree crates give the same extension point without it.
