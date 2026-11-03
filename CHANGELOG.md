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

### Fixed
- jet: Jet3 fixture test renamed away from the retired engine-boundary name
  (step 31.1).
