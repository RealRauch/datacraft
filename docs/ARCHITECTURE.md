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
│                              │ StorageEngine trait (§6)          │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │ datacraft-storage                                         │  │
│  │  ┌─────────────────────────────┐   ┌───────────────────┐  │  │
│  │  │ jet-rs backend (product)    │   │ libmdb oracle     │  │  │
│  │  │ pure-Rust Jet/ACE reader &  │   │ (test-time only,  │  │  │
│  │  │ writer, clean-room          │   │ LGPL, cfg(test))  │  │  │
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
├── datacraft-storage/   StorageEngine trait + Jet/ACE backends (Ph. 2+)
└── datacraft-ui/        eframe/egui desktop shell (bin: `datacraft`)
```

Dependency direction rules (review-enforced, never cyclic):

```text
datacraft-ui      ──► datacraft-core
datacraft-storage ──► datacraft-core
datacraft-core    ──► (nothing internal)
```

- The UI never touches `datacraft-storage` directly; storage access happens
  exclusively through core commands.
- No async runtime (tokio & co.). The egui event loop drives the process;
  background work uses `std::thread` + channels behind core ports.
- `[workspace.dependencies]` pins every external version once;
  `[workspace.lints]` defines shared lints (`unsafe_code = "forbid"`).

## 5. Headless core & command API (Principles 3 + 4)

Target design (implemented in Phase 4; shape fixed here):

```rust
/// Every user-visible action, GUI or agent, is exactly one command.
pub enum Command {
    OpenDatabase { path: PathBuf },
    CloseDatabase,
    CreateTable { definition: TableDef },
    DropTable { table: String },
    OpenObject(ObjectId),
    CloseTab { tab_id: TabId },
    SetActiveTab { tab_id: TabId },
    ToggleNavPane,
    RunQuery { query: ObjectId },
    SetQbeCell { row: QbeRow, col: QbeCol, value: QbeValue },
    AddFormControl { form: ObjectId, control: FormControl },
    // ...
}

/// Observable outcome of a dispatch; the UI re-renders from state, agents assert on events.
pub enum Event { /* DatabaseOpened { .. }, TabOpened { .. }, .. */ }

impl Core {
    /// Pure reducer over application state; I/O happens behind ports (StorageEngine, fs).
    pub fn dispatch(&mut self, command: Command) -> Vec<Event>;
    /// Serializable snapshot of everything the UI shows — the headless test seam.
    pub fn inspect(&self) -> UiStateSnapshot; // serde::Serialize
}
```

**Test-ID convention** (deterministic, dot-separated, stable across releases):

```text
ribbon.<tab>.<group>.<action>     e.g. ribbon.create.tables.table_design
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

No Jet/ACE-specific types leak across this boundary — the core works against
`TableSchema`/`RowBatch` domain types. Backend selection is feature-gated:
`jet-rs` (default, product) and `libmdb-oracle` (test).

Since Phase 13 the same port also fronts a **SQLite** backend: `DetectingPort`
sniffs the file magic at `open` and delegates to the Jet/ACE or the SQLite
implementation, so all clients keep one command vocabulary for both source
families. `InsertRow`/`SqlRan` are port-level writes and the shadow-SQL
engine reads every source through the port (see `docs/FORMAT-JET.md`
§"SQLite backend").

**Format milestones:** JET4 (`.mdb`) read → ACE (`.accdb`) read → JET4 write →
ACE write. Each milestone: spec → decoder/encoder → proptest round-trip →
golden files → oracle comparison. Landed so far: Jet4 read, Jet3 (Access 97)
read incl. chained TDEFs, >255-byte jump-table rows and overflow/LVAL pages,
embedded-OLE columns (fixed 0x11 long binary), ACE read and complex values
(attachments, multi-value, version history; Phase 16), Jet3/Jet4 RC4 database
passwords and page encryption (Phase 16; ACE Office encryption stays a precise
`Unsupported`), the Jet4 writer core incl. update/delete and journaled batches
(tables + rows, usage-map-consistent; Phases 10–11), and the Phase 17
read-only programmability layer (named macros, VBA extraction/viewer/static
analysis). Remaining write-path work: AutoNumber counters (M8), `DropTable`
(M7), LVAL-chain writes for complex values, the ACE write path and the
Access-repair gate. The README compatibility matrix is the claim set.

## 7. Domain data model (preview of Phase 3)

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

## 8. Rendering & performance

- egui immediate mode; **no layout or business logic in paint code** — layout
  computations are pure functions in core (unit-testable, Principle 4).
- Shared grid core (Phase 6): virtualized painting of visible cells only,
  zero-copy row slices, columnar value cache — the GridCraft philosophy.
- Target: 60 FPS with million-row virtual scrolling; no per-frame allocations
  on the hot path (reuse `Vec`/`String` buffers).
- Original iconography: egui `Painter` vector primitives and an embedded
  open-source glyph set — no Microsoft assets (Principle 2).

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
│   ├── datacraft-storage/      lib — StorageEngine + Jet/ACE backends
│   └── datacraft-ui/           bin — eframe desktop shell
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

| Phase | Delivers |
|---|---|
| 1 — Blueprint & Skeleton | this document, workspace, CI gates, testing policy |
| 2 — Storage Engine Core | `StorageEngine` trait, Jet page layer, TDEF/TDAT decoder, libmdb oracle |
| 3 — Data Model & Schema | Access data types, field properties, serde persistence |
| 4 — Headless Core & Command API | command bus, state query, test IDs, JSON-RPC stub |
| 5 — UI Skeleton (egui) | app shell, ribbon, navigation pane, tabbed workspace |
| 6 — Datasheet & Design View | shared grid core, datasheet, table design view |
| 7 — QBE Query Builder | relationship mapper, QBE grid, query engine |
| 8 — Form & Report Canvas | WYSIWYG canvas, Access control types, form runtime |
| 9 — MCP Endpoint & Automation | MCP/JSON-RPC server, headless E2E harness, release prep |

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
config dir resolved without a new dependency), stores language and window
geometry now and appearance later, and treats unknown values as defaults
instead of failing. Precedence: explicit setting > `DATACRAFT_LANGUAGE` >
system detection > auto.

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
licensed (OFL) and attributed.
