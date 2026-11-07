# Changelog

All notable, user-visible changes of the DataCraft crate family. The family
versions in lockstep (see `docs/PUBLISHING.md`); entries are
crate-attributed. Format based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

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
