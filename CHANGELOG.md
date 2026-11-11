# Changelog

All notable, user-visible changes of the DataCraft crate family. The family
versions in lockstep (see `docs/PUBLISHING.md`); entries are
crate-attributed. Format based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

*Next version: reassessed at every phase close (breaking → minor, additions
and fixes → patch per `docs/PUBLISHING.md`).*

### Added
- ui: the feedback & honesty sweep (ROADMAP §16 stage 0) — Ctrl+Z/Ctrl+Y
  reach the shipped undo/redo commands (an empty history stays a no-op), every
  ribbon/QAT/backstage control carries a localized tooltip from one
  spec-complete registry, the status bar keeps a bounded clickable message
  history (newest-first dialog with Clear), F1 opens the localized shortcuts
  page, and whole-file commands (open, create-from-template, save-a-copy)
  queue behind a painted busy overlay instead of freezing an unannounced
  window (68).

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
