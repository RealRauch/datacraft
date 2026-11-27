# Changelog

All notable, user-visible changes of the DataCraft crate family. The family
versions in lockstep (see `docs/PUBLISHING.md`); entries are
crate-attributed. Format based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

### Added
- jet/core: referential write parity (phase 104, M137; roadmap §13a) —
  an orphan insert, update, restore or batch operation against a
  declared relationship refuses on Jet exactly where SQLite's engine
  refuses it (precise, locatable reason; `NULL` child values pass), and
  a journal batch containing an orphan is refused in preview. Deletes
  stay allowed — cascades are §13b.
- types/core/ui/csv: scale-aware decimal display (phase 104, M135) — a
  fixed-point decimal renders with its declared scale (`25.50`, not the
  raw scaled integer) in the datasheet, the windowed tabs, the form
  runtime, the value viewer, the batch preview, the CSV driver's
  rewrite and the CSV/JSON exports; scale 0 stays byte-identical and
  the phase-99 export workaround delegates to the shared helper.
- core/ui/cli/mcp/tools: migration readiness and the benchmark
  instrument (phase 103, M134; roadmap §14/§18) — one read-only
  `AnalyzeApp` command aggregates per-object ring coverage with precise
  unsupported reasons, a VBA transpilation estimate, the linked-table
  inventory and an effort summary; the Database Tools > Migration
  readiness window renders every section read-only, and CLI `exec` and
  MCP `dispatch` carry the same report over the additive wire. The
  family-A benchmark harness (`tools/dao-oracle/bench.ps1`) and
  `docs/BENCHMARKS.md` record the roadmap §18 methodology and the first
  measured matrix against DAO (10k/100k/1M, narrow/wide) with the honest
  gaps.
- core/ui: designer parity (phase 102, M133; roadmap §16 stage 2) — both
  designers gain the missing interaction vocabulary. A placed control's
  property sheet edits its name, visibility, enabled state, weight and
  alignment (one command per edit, precise validation that leaves the
  design unchanged on failure) and persists the set in the sidecar; older
  sidecars load the defaults. Eight resize handles resize through the
  shared core geometry (grid snap, one-grid minimum, canvas/page clamp,
  one rect command per gesture). Bring to Front/Send to Back and Move
  Up/Move Down edit the paint order and the persisted tab order
  independently, each one command. Dragging a field from the palette
  binds it at the drop point (the click keeps the next-free-slot rule),
  and the print preview's fixed 0.9 zoom became a per-surface setting
  (50–200 %, persisted in `ui.json`).
- core/ui: windowed datasheet reads (101.5, M132) — the port gains
  `row_count` (jet from the TDEF counter, sqlite via `COUNT(*)`, csv and
  xlsx from their parsed tables; a backend without a cheap count answers
  a precise `Unsupported`), and a table above 1000 rows opens from a
  bounded 200-row window cache: the count sizes the list, scrolling loads
  the next window, "Load all rows" upgrades explicitly, and a filter, a
  sort or a staged batch upgrades on its own. The wire is unchanged — the
  snapshot never carried row data.
- ui/core: datasheet completeness (101.1–101.4, M132) — filter by value
  from the column's own list (bounded, ordered, blank entry where blanks
  exist, refused precisely when the list changed), find and replace over
  the visible rows (text columns, staged into the batch on atomic drivers
  and one `UpdateCells` command otherwise), movable columns in the header
  menu with every render and edit path following the display order, and a
  visible `(NULL)` marker.
- jet/core/ui/cli/mcp: linked-table inventory (100, M131; roadmap §9 stage
  0) — a Jet/ACE file's `MSysObjects` link records (`Type` 4 ODBC, `Type`
  6 Access-file) decode into a read-only inventory: local name, kind,
  external database path or connect string, foreign name and flags. The
  port boundary redacts `PWD`/`PASSWORD` values, so no snapshot, CLI reply,
  log or suite output carries the secret. The nav pane gains a counted
  `Linked tables` group with the one-line target and a boundary hint, the
  search matches the target, and the snapshot (CLI `--json`, MCP `inspect`)
  carries the same records. Nothing resolves or dials: opening a link
  answers a precise `Unsupported`, a link never enters the table catalog,
  and the file stays byte-identical. Three fixtures pin the shape (two
  pristine jackcess files plus a self-built Access fixture); the storage
  composite now forwards the inventory like every other session surface.
- core/ui/cli/mcp: table export to CSV and JSON (99, M130) — one
  `ExportTable` command writes a whole table to a new file: CSV with a
  header row and the datasheet text (RFC 4180 quoting, LF terminators, a
  decimal keeps its field's declared scale), JSON with the field-kind
  schema plus typed rows (numbers, booleans, `null`, dates as text,
  binary as hex, multi-value as an array). Bytes are deterministic, the
  target is created exclusively (an existing file is refused and left
  alone), a value JSON cannot carry answers precisely before anything is
  written, and the source database stays byte-identical. The datasheet's
  trailing row carries one entry per format with the save dialog and a
  completion notice; CLI and MCP dispatch the same command.

## [0.2.2] — 2026-11-23

> Release note: 0.2.1 was published incompletely — `datacraft-mcp` could not
> build against the registry tarball because the PDF font lived outside the
> `datacraft-core` package. The 0.2.1 versions are yanked; use 0.2.2.

### Fixed
- core: the PDF export font now travels inside the `datacraft-core` crate
  (`assets/fonts/` with its OFL text), so a consumer enabling the
  `pdf-export` feature builds from the published tarball; the dependent
  `datacraft-mcp` release resolves again.

### Added
- core/sqlite/ui/cli/mcp: SQLite SQL pushdown (98, M129) — the SQL window
  now executes statements natively on a SQLite source: full dialect
  (window functions, CTEs), real indexes, native views, typed results and
  EXPLAIN QUERY PLAN. The port keeps it read-only through the engine's
  own statement flag, so every write form (including `WITH … DELETE` and
  PRAGMA writes) answers a precise capability refusal; Jet/ACE/CSV stay
  on the shadow engine unchanged. The snapshot names the dialect and the
  SQL window shows it plus a Query plan toggle that renders SQLite's
  plan in the result grid; CLI and MCP dispatch the same command.
- core/ui/cli/mcp: PDF report export (97, M128) — a report's print
  preview exports to a new PDF file as its second render target: one PDF
  page per preview page, the report's own page geometry, text kept as
  text through an embedded Type0 Inter font with a ToUnicode map, and
  byte-identical re-export (fixed metadata, stable object order). The
  pure-Rust writer sits behind the optional core `pdf-export` feature
  (the shipped clients enable it), the preview toolbar gains an Export
  PDF… button with the save dialog and a completion notice, and CLI/MCP
  dispatch the same command. An existing target is refused; the source
  database and sidecar stay untouched.
- xlsx/core/ui/cli/mcp: Excel workbooks as read-only sources (96, M127) —
  a new `datacraft-xlsx` driver (calamine, core-only dependency) opens
  `.xlsx`, `.xlsm`, `.xls` and `.xlsb` as a database of sheets: the first
  row names the columns, kinds are inferred conservatively and every cell
  maps onto the domain values (empty → Null, formulas → their cached
  value, macros never read); all four families deliver identical kinds
  and values. The composite registry now carries multiple out-of-tree
  fallbacks, the snapshot disables every write affordance with the
  localized reason, and the datasheet renders the sheets like tables.
  Import File accepts Excel workbooks through a core-owned reader seam
  (preview and the ordinary create/insert chain). ODF and
  password-protected workbooks refuse precisely; fixtures are built by
  the new `tools/xlsx-fixtures/build.ps1` and pinned.
- jet/ace/core/ui/cli/mcp: maintenance — health facts and Compact (95,
  M126) — every Jet/ACE open now carries page-0 facts (version, page
  size/count, byte size, code page, encryption, password) and a bounded
  page-type scan whose damage findings fill the snapshot's `health`
  without blocking the open; `CompactDatabase` rewrites the open Jet4/ACE
  database from the embedded engine-written template, restores rows with
  their AutoNumber identities and user relationships, verifies the fresh
  file against the source, swaps it atomically at the same path and
  refreshes the session (identical input → identical bytes; real Access
  opens both the compacted `.mdb` and `.accdb`). The Info page renders
  the facts and a capability-gated Compact button with a localized reason
  and a completion notice; CLI and MCP dispatch the same command. Jet3,
  password-protected, Office-encrypted and Access-locked sessions refuse
  precisely; SQLite/CSV declare the capability as unavailable. Also fixes
  the detecting composite dropping the session's file facts, view/index/
  trigger/native-object inventory, relationship groups and the native
  view path (F15).
- ui: the dock rollout and the details Type column (93, M125/L64) —
  the Relationships mapper and the Expression window dock through the
  pane host like the SQL window (dock right/bottom or float, per-surface
  geometry in ui.json, the workspace never overlaps a docked pane, the
  pane header localizes its title), and the Object Details list renders
  each row's localized kind as a trailing Type column while keeping the
  nav sort, the search narrowing and the multi-select semantics.
- ui: the open-items sweep (92, M99/M108/L48/L51) — platform-chord
  keyboard shortcuts (⌘S/⌘W/⌘Z/⌘⇧Z on macOS, Ctrl+S/W/Z/Y elsewhere,
  with the help page and tooltips rendering ⌘ through one
  platform-aware formatter), the docked task-pane host piloted by the
  SQL window (dock right/bottom with resizable bounds or float, per-
  surface geometry in ui.json, the workspace never overlaps a docked
  pane), a navigation-pane search across categories, multi-select with
  Ctrl/Shift clicks and a selection-aware context menu, the flat Object
  Details view beside the category tree with local groups and palette
  colours persisted per database, and the low-power/RDP rendering mode
  (an Options tri-state over the detection; zero animations, no blurred
  shadows, ~30 fps interaction repaints, idle repaint at zero).
- cli/core/ui: version reporting across all three clients (91, M109) —
  the CLI answers `--version`/`-V` with `datacraft-cli <version>` on
  stdout (exit 0, before any database work, flag position irrelevant),
  the JSON-RPC transport gains the additive `version` method answering
  name and version, and the backstage Info page shows the row above the
  database facts — with and without an open database. One shared source
  (`datacraft_core::VERSION`, the lockstep workspace version); the label
  is localized, the value stays locale-neutral.
- ui/core: the import surface is honest about wrong-surface files
  (91, M111/M112/L53) — a Jet/ACE or SQLite file chosen in the Import
  File window answers a format-naming rejection naming the right surface
  ("open it via File → Open") instead of a decoder byte error or a
  silently imported garbage table (Jet bytes decode as text); the
  open-file pickers filter to their surfaces' extensions (text import:
  csv/txt/tsv, database open: mdb/accdb/sqlite/sqlite3/db, localized
  "Supported files"); composed import errors carry the source path
  exactly once; every path field names the chosen file prominently with
  the full path on hover.
- ui/core: shell chrome keeps every action reachable at narrow widths
  (88, M98/L45/L49) - ribbon groups collapse whole trailing groups behind
  one localized overflow control when the row runs out of width (the fit
  math lives in core, the menu stacks the hidden groups' own buttons so
  dispatch, disabled states and tooltips stay identical; no command
  leaves the window silently), the tab strip scrolls horizontally and
  scrolls the selected tab back into view on every activation, and the
  tab context menu gains Close Tabs to the Left/Right as bus commands
  (`CloseTabsToLeft`/`CloseTabsToRight` over the wire, activation moves
  to the anchor only when it was on the closed side); activating an
  already-open object re-activates its tab, now matched case-insensitively,
  and never touches a tab with a staged batch; a headless glyph-coverage
  contract walks both catalog languages and hostile names against the
  shipped font stack (T11).
- ui/core: datasheet selection, clipboard, fill and the cell value viewer
  (86, M101/M102) - a multi-cell selection model (anchor rectangle,
  Shift+arrows/Shift+click extension, whole-column selection through the
  header, Ctrl+A; the selection survives scroll untouched and clamps into a
  smaller row set after sort/filter changes); Ctrl+C copies the selection as
  spreadsheet-compatible TSV and Ctrl+V validates the pasted matrix's fit and
  every value before any cell changes, then writes through the tab's normal
  path (the H61 batch on an atomic-batch driver, direct row writes
  otherwise); Ctrl+D fills the selection from its top row and Del over an
  extended selection clears the cells to NULL - both as one command and one
  undo step; Shift+F2, Alt+click and the cell context menu open a value
  viewer (JSON indents only when it parses, binary values show a bounded hex
  view naming the truncation) whose multiline editor writes editable long
  text through the same path.
- ui/core: datasheet foreign-key navigation and the structured filter
  builder (87, M103/M104) - a key cell's context menu and F4 follow the
  declared key (composite constraints stay paired over the port's
  constraint groupings), the inferred `<table>_id` reference or the
  reverse referenced-by target: the target opens or activates with the
  row's key values as exact-equality filters whose encoding the real
  criteria parser validates, empty keys and unencodable values answer
  precisely, and drivers without relationship support hide the
  affordance; the datasheet's trailing row carries Back/Forward over the
  per-tab navigation stacks (a closed source reopens with its stored
  filter, a source with pending edits only activates); the reverse menu
  entries carry capped referenced-by counts (window-capped separate
  reads, the truncation named, counts cleared with the tab); the Filter
  Builder window edits a per-tab draft that serializes onto the existing
  criteria map through the core model (OR groups and duplicate columns
  fence precisely), the text mode stays the raw-criteria escape hatch
  including the bare wildcard's implicit Like.

### Changed
- ui/core: the datasheet sort order is total (91, M113) — NULLs sort
  first ascending and last descending (the Access convention), mixed
  families follow the documented `FieldValue` declaration-order rank,
  non-finite floats tier after their family's comparable values, and a
  renamed-away sort field clears its state instead of claiming an order
  the rows do not follow; the semantics are documented in FEATURES and
  a COMPATIBILITY matrix row.
- ui: the L47 design-review fix sweep (91 — M110/M114, L52/L54,
  M116–M122, L56–L63, M124) — the SQL window's result area is
  height-budgeted (a small result no longer grows the window toward
  full screen; large results scroll inside the bound), status lines
  truncate with an ellipsis and carry the full text on hover, the
  AutoNumber new-record cell renders its gate before input and the
  rejection names the column, freeze carries tooltips and a visible
  frozen boundary, the navigation pane reads honestly (the sort menu
  names its pane-wide scope and marks the active order, a row opens from
  its whole rect, the native inventory renders in the pane's row
  language, the category header is a plain label, view creation lives in
  the Views group menu), the Save page names every section, the Info
  values align into one column, the tool windows gain scroll (the
  relationship mapper), token-clean errors (the VBA viewer), labelled
  fields (the view editor), window-chrome close (the expression window),
  painted severity glyphs (the notices dialog) and aligned macro
  numbering, the form canvases scroll both axes with an accent selection
  ring and a painted combo caret, the report preview follows the
  report's PageSetup with page-scaled text, and the form navigation row
  gains First, Last and New record (capability-gated, the edit buffer
  flushed, the form sits on the new record).
- ui: datasheet edits stage into a pending batch instead of writing
  immediately (85, H61) — on Jet4 and SQLite sources every cell edit,
  new record and deletion collects into a per-tab edit buffer with
  amber pending markers; Ctrl+S previews the journal's own diff and
  applies the batch as one atomic write (a failed apply keeps every
  edit and the database untouched; Discard keeps the buffer; Escape
  discards the active record's staged changes; the grid refuses input
  while a submission is in flight). Drivers without the atomic-batch
  capability and read-only sources keep the immediate per-cell path.
- core: the snapshot reports the pending write journal as a compact
  summary (85, H61) — table, token and per-kind operation counts — so
  every client observes the batch awaiting confirmation, and
  `PreviewWrite` refuses while a journal for another table is pending.
- sqlite: row-write batches apply atomically (85, H61) —
  `write_row_ops` wraps the batch in one transaction with rollback on
  the first failure; the capability reports `atomic_batch: true`,
  retiring the phase-33 one-operation-at-a-time refusal.
- ui: native file and message dialogs no longer block inside the event
  loop (81, M97) — every picker and confirmation parks a request and its
  follow-up, the native host answers on a helper thread through rfd's
  non-modal async panels, and the flow resumes on a later frame. This
  removes the documented macOS `runModal`-in-winit abort class; Cancel,
  an unavailable dialog host and a picked path stay distinguishable, and
  the queued model is pinned headlessly through the scripted test host.
  One dialog runs at a time; the queued model is unverified on macOS
  hardware (no Mac available) — `docs/COMPATIBILITY.md` names the
  follow-up reproduction pass.
- core: atomic file saves (databases, sidecars, settings) are durable and
  concurrency-safe (80, H60) — every replacement now stages into a unique
  per-writer file (process id + counter), flushes it to disk before the
  rename, syncs the parent directory best-effort on Unix, restores the
  replaced file's permissions, resolves symlinked targets through the link,
  and retries the Windows sharing/lock errors (antivirus scanners, search
  indexer) with a bounded backoff. A crash between staging and the rename
  leaves the live file byte-identical with at most one orphan staging file.
- core/csv: CSV sources open beyond UTF-8-comma (80, H59) — one shared decode
  path behind the Import File flow and the CSV driver accepts valid UTF-8
  (with or without BOM), UTF-16LE/BE byte-order-mark streams and
  Windows-1252, and sniffs the delimiter (comma, semicolon, tab) outside
  quotes; undecodable bytes still refuse precisely instead of decoding
  lossily, and the Import File preview names the detected encoding and
  delimiter (German and English). Existing UTF-8 comma files keep their
  byte-identical results.
- publish: every published crate packages its own crates.io README (90,
  L44) — a per-crate page (purpose, public surface, feature flags, MSRV,
  license, product-page link) replaces the inherited root product README,
  which stays the product entry; PUBLISHING's release checklist names the
  per-crate README verification (`cargo package --list`).

## [0.2.0] — 2026-11-14

### Added
- ui/core: the empty-shell landing and the two-step New-database flow (75,
  L41–L43) — a cold start and the close of the last database land on the File
  backstage; the workspace returns with the next open, and the Back row still
  reaches the ribbon. The New page asks what to create first (blank or a
  shipped template) and where second, keeping the choice across Back. A blank
  database chooses its family explicitly — Jet4 (`.mdb`, the default) or
  SQLite — through `CreateDatabase`; a path whose known extension contradicts
  the chosen format answers a precise `InvalidInput` before the port, for GUI,
  CLI and MCP alike.
- types/expr/query/core: the crate-family split (76, L36–L37) — three new
  published crates `datacraft-types`, `datacraft-expr` and `datacraft-query`
  now carry the value/schema vocabulary, the expression engine and the
  query layer beneath `datacraft-core`, which re-exports every existing path
  (`datacraft_core::{schema, expr, qbe, query_engine, native_exec,
  sql_types}` and the native-query port vocabulary) unchanged. The shadow
  SQL engine's rusqlite dependency moved behind `datacraft-query`'s
  `sql-engine` feature.
- import: the Access migration tool (78, M88) — `datacraft-import` reads
  native Jet4/ACE `.mdb`/`.accdb` form/report designs through the pinned
  pure-Rust `jetdb` crate, without Microsoft Access, and merges the supported
  subset (label, text box, check box, combo box, command button, subform)
  into the `.datacraft` sidecar. The database is never written; unsupported
  controls and properties are reported per object and `--strict` exits 1 on
  them. Access 97 form/report designs stay unsupported; combo row sources
  and controls whose geometry Access stored only as a default are first-cut
  limitations (reported, never guessed).

### Changed
- core: the `sql-engine` feature is **off by default** and forwards to
  `datacraft-query/sql-engine` (76, L37). Consumers that relied on the
  default build for `RunSql` must enable the feature explicitly; a
  default-feature core is now C-free (no rusqlite in its graph). This is a
  breaking change, hence the minor version.

## [0.1.3] — 2026-11-12

### Added
- sqlite/ui: SQLite depth stage 0 (73, M90–M92) — a foreign `.sqlite` file
  now shows what it contains and reports its state honestly. The file's own
  views open as read-only objects (nav-pane Views group, datasheet, SQL
  path; sidecar designs keep the writable authority), indexes and triggers
  get a read-only inventory in the table design view, and the Info page
  lists the driver facts (`sqlite_version`, page size, journal mode,
  encoding, `user_version`, busy timeout, `STRICT`/`WITHOUT ROWID` tables)
  plus a bounded `quick_check` verdict — a damaged file opens with a precise
  warning notice instead of a late, unrelated query error. The open sets an
  explicit 5000 ms `busy_timeout` so short writer contention is waited out;
  a WAL session reports its mode; `WITHOUT ROWID` row windows follow the
  declared primary key and rowid writes answer a precise `Unsupported`.

### Added
- core/jet/ui: native Access object inventory (74, M87) — the navigation
  pane lists the catalog's native forms and reports read-only (names and
  counts from the `MSysObjects` object types, a precise "definitions are
  not decoded" hint, no tab and no write path). The companion feasibility
  study (M89) established the `MSysAccessStorage` container and the
  blob/`TypeInfo` string layer against the committed SaveAsText exports,
  but the numeric property-id table has no public source; the verdict is a
  documented stop, recorded in `docs/FORMAT-JET.md`.

## [0.1.2] — 2026-11-12

### Added
- ui: the feedback & honesty sweep (ROADMAP §16 stage 0) — Ctrl+Z/Ctrl+Y
  reach the shipped undo/redo commands (an empty history stays a no-op), every
  ribbon/QAT/backstage control carries a localized tooltip from one
  spec-complete registry, the status bar keeps a bounded clickable message
  history (newest-first dialog with Clear), F1 opens the localized shortcuts
  page, and whole-file commands (open, create-from-template, save-a-copy)
  queue behind a painted busy overlay instead of freezing an unannounced
  window (68).

### Added
- core/ui/sqlite: sidecar views (71) — a named SELECT stored in the
  `.datacraft` sidecar, opened as a read-only result tab with an editor;
  Jet/ACE/CSV execute it through the shadow SQL engine, while SQLite applies
  it natively (`CREATE VIEW`/`DROP VIEW`) and rolls the sidecar change back
  on a native failure. Trigger designs are decided in ROADMAP §19 but
  deferred.

### Changed
- ui: the M42 live-review pass (69) — the relationship mapper lists user
  relationships only (Access's `MSys*` system rows are filtered at load and
  can no longer be deleted), the Relationships/SQL/Expression toggles paint
  their open state, an open returns to Home while a create activates the new
  document, a question-mark help glyph sits in the ribbon tab row, the status
  filter chip lists and removes individual filters, datasheet headers show
  their filter and sort state, and every tool window opens centered.

### Fixed
- core/ui: the second M42 review round (70) — bare wildcard filters on text
  columns (`*a*`) match through the Access matcher instead of comparing the
  literal, the message history stores messages semantically and follows a
  live language switch, and the tool windows carry an accent frame with
  stronger controls and legible secondary text in both palettes.

### Added
- core/jet/sqlite/ui: object renaming (72) — one additive `RenameObject` bus
  command renames tables, queries, forms and reports from the navigation
  pane's context menu or F2. Tables rename physically (Jet/ACE: the catalog
  row plus every `MSysRelationships` object column; SQLite: `ALTER TABLE …
  RENAME`), while open tabs, datasheet view state, form filters and the
  sidecar's structural references follow the new name. Collisions and
  invalid names answer precisely; stored view SQL text stays verbatim;
  SQLite refuses case-only renames with a precise reason; CSV reports its
  capability boundary. The vocabulary is wire-additive (`Command`/`Event`
  are `non_exhaustive`), so the CLI and MCP clients dispatch the command
  unchanged.

## [0.1.1] — 2026-11-10

### Added
- core/ui/cli: server-source readiness — `OpenDatabase` carries an additive
  `SourceSpec {Path, Uri}`: `file://` URIs open exactly like their path
  equivalent through every client (backstage Open entry, CLI argument, raw
  wire and the MCP endpoint), the snapshot reports the session's source with
  a single core-side display form (Info page row), the file-bound
  `.datacraft` sidecar rule is pinned, and every other scheme answers the
  precise no-server-driver boundary naming the scheme and ADR-7 — no server
  driver ships yet; the ADR-7 extension note lands with the first driver's
  planning (65).
- ui/core: the form runtime edits bound records the Access way — text boxes stage per record (exactly one `UpdateRow` when the record is left, Esc reverts the field then the record, the `Required`/`AllowZeroLength` zero-length rule), checkboxes toggle two-state, combos list every row of a table/query row source (duplicates included) and accept free text, command buttons dispatch one action (open form/report, run query, apply filter) and form-scoped WHERE filters narrow the record set without touching the datasheet (64).
- ui/core: the designers edit the selected control's content — the one
  property each kind carries (label text, button caption, bound source
  field or embedded form) with precise validation; `SetFormControlContent`
  / `SetReportControlContent` are ordinary bus commands and the
  runtime/preview re-renders from the refreshed snapshot (63).

## [0.1.0] — 2026-11-08

### Added
- core: `DriverCapabilities::none()` — the conservative declaration a minimal
  driver starts from; `StoragePort::driver_id`/`::capabilities` are required
  trait methods now (M22).
- core: `Support`, `PortError` and `Command` are `#[non_exhaustive]` — adding
  vocabulary variants no longer breaks out-of-tree drivers and clients (M22).
- release: tag-triggered `release.yml` — Windows/macOS/Linux archives with
  binaries, samples and third-party notices, per-artifact smoke (Jet fixture,
  samples demo, CSV directory), sha256 checksums, draft GitHub release
  reviewed by a human before publication (M13, steps 26.1–26.2).
- jet/sqlite: writes across every capable source — ACCDB update/delete/alter/
  drop with byte preservation (25); design-time schema editing, index creation
  and relationship/foreign-key creation (32).
- core: native saved-query execution over the fenced Access dialect (27);
  encrypted Save-As and password change (27); CreateDatabase and
  CreateDatabaseFromTemplate with the shipped sample registry and sidecar
  auto-load (29); undo/redo history and recent databases (28).
- core/ui: the driver model — declarative capabilities, registry and
  core-computed affordances with localized disabled reasons — proven
  out-of-tree by the CSV directory driver; registered drivers are visible in
  the snapshot (30/36).
- ui: datasheet workflow — keyboard record navigation, horizontal scrolling
  with frozen columns, row deletion, in-place cell editing (40); first-class
  column filters with a status chip and the filter dialog in a window (44/45);
  record navigation reads the cursor the arrows write and the new-record row
  stays above the status bar (48).
- ui: designer affordances — relationship create/delete, expression window,
  VBA static-analysis viewer, report grouping picker (41); runtime control
  rects on the designer canvas (46); the contextual ribbon tab follows object
  activation (47).
- ui: visual overhaul — original iconography, backstage restyle, native file
  and message dialogs, window chrome polish (38); honest failure surfacing for
  corrupted date pictures, SQL dialect divergence and unsupported QBE
  criteria (39).
- jet: page-0 tail emit/maintenance on the create path — DataCraft-created
  files open in real Access; the `recover_mdb` example re-encodes
  engine-refused files (49).
- jet: Access co-existence — a sibling Access lock file (`.ldb`/`.laccdb`)
  opens the database read-only, names the lock file in the snapshot (CLI/
  MCP data), the status bar and the Info page, and answers a precise
  refusal on every write; DataCraft never creates or deletes lock files
  (53).
- core/ui: the Import File flow — an external CSV file becomes a table in
  the open Jet4/SQLite database over one bus chain (`ImportFile`: parse,
  L7-inferred column kinds, create + row inserts over the ordinary write
  path, capability-gated); the window previews fields and leading rows and
  the source file stays byte-identical (58).
- core/ui: every dispatch failure carries a stable machine-readable code
  from the new `FailureCode` vocabulary alongside the English reason
  (wire-additive; capability refusals map onto the affordance vocabulary);
  the UI localizes known codes and keeps the English text otherwise (62).

### Changed
- workspace: `datacraft-storage` split into `datacraft-jet` (clean-room
  pure-Rust Jet/ACE backend) and `datacraft-sqlite` (rusqlite backend);
  `datacraft-storage` remains as the driver composite (detection + samples)
  (step 31.2, M19).
- core: `rusqlite` is optional behind the default-on `sql-engine` feature;
  `SqlResult`/`Source` moved to `datacraft_core::sql_types` — the state and
  wire vocabulary stay C-free, and `datacraft-jet` builds without rusqlite
  (step 31.2).
- mcp: the headless stdio JSON-RPC endpoint is the standalone
  `datacraft-mcp` binary crate (step 31.3).
- core: journal batches gate on the declared atomic_batch capability (33);
  design-time schema edits and QBE projection respect the capability gates
  and the schema whitelist (42); CSV drivers infer column types beyond text
  (36).
- jet: a crafted tri-state Boolean NULL write answers a precise boundary
  message naming column, consequence and alternative, while SQLite keeps
  tri-state NULL — the cross-driver difference is deliberate and documented
  (36).
- jet: the retired Northwind fixture is replaced by a self-created,
  hash-pinned inventory (37).

### Fixed
- jet: Jet3 fixture test renamed away from the retired engine-boundary name
  (step 31.1).
- ui: ribbon group stretch and the status-bar panel overlap (44); the
  datasheet's trailing panels — including the new-record row — stay above the
  footer (48); record-arrow reading order (45); the close-database affordance
  and the contextual design view (44).
