# Publishing — crates.io order, Semver/MSRV policy, checklist

> Step 31.4 (M19). This file is the binding publishing policy for the
> DataCraft crate family. The tag/tagging side lives in `docs/PLAYBOOK.md`
> §9; the release build lives in `.github/workflows/release.yml`.

## Published crates and order

Dependency order defines the publish order — a crate is published only after
the versions it compiles against exist on the registry:

1. `datacraft-core` — the headless engine (command bus, port trait, state,
   snapshots, JSON-RPC transport). The `sql-engine` feature is optional with
   `default = on`.
2. `datacraft-jet` — clean-room pure-Rust Jet/ACE backend. Declares
   `datacraft-core` with `default-features = false`: the crate is C-free by
   construction (no rusqlite in its dependency graph). Two surfaces are
   public and semver-covered: the port-facing API (`Jet4Port`,
   `StorageError`, the `types` vocabulary) and the raw format layer under
   `datacraft_jet::jet` (readers, writers, page/TDEF/row helpers), which
   exists for consumers that need format-level access and for the crate's
   integration tests. The raw layer follows the same semver rules; a
   refactor of its signatures is a minor bump pre-1.0.
3. `datacraft-sqlite` — SQLite backend (rusqlite, statically bundled).
4. `datacraft-csv` — CSV directory driver (ADR-7 out-of-tree proof).
5. `datacraft-storage` — driver composite (format-detecting registry plus the
   samples generator) for consumers that want the shipped backends bundled.
6. `datacraft-mcp` — headless stdio JSON-RPC 2.0 endpoint
   (`cargo install datacraft-mcp`).

**Testing surface:** `datacraft_core::port::MockPort` is public on purpose —
it lets out-of-crate clients drive `Core` over the command bus in their own
tests without a database file. Its behaviour is part of the supported API
(the same contract the real drivers implement where they overlap); it is not
a production backend. The modules that only host `impl Core` blocks are
private, so docs.rs shows no empty modules.

**Not published:** `datacraft-ui`. The desktop shell ships only through the
release archives — its value is the application, its dependency tree
(eframe/wgpu/fonts) is large, and it has no library consumer. This exclusion
is a decision, not an oversight; revisit only if a library consumer appears.

## Versioning (Semver)

- **Pre-1.0 the family versions in lockstep:** the workspace
  `[workspace.package] version` is the single source of truth (currently
  `0.1.0`). A breaking change bumps the minor (`0.y` → `0.y+1.0`), additions
  and fixes bump the patch. From 1.0 on, standard Semver applies (breaking =
  major). Independent per-crate versions are deliberately postponed until
  after 1.0 — the family is small and releases are coordinated anyway.
- **MSRV:** the workspace `rust-version` (currently 1.98) is binding for
  every published crate. `rust-toolchain.toml` pins the *development*
  toolchain and is not the MSRV contract. An MSRV bump is a minor-version
  change (pre-1.0) and is called out in the changelog.
- **Vocabulary stability (M22/M32):** `Support`, `PortError`, `Command`,
  `Event`, `DisabledReason`, `DatabaseFormat` and `SchemaChange` are
  `#[non_exhaustive]` — adding variants is additive for out-of-tree drivers
  and clients (wildcard match arms). Snapshot structs (`UiStateSnapshot`,
  `DatabaseSnapshot` and the entry types) are **read-only**: field access is
  the contract; constructing or exhaustively destructuring them is not
  (adding a field is an additive change in practice).
- **`DriverCapabilities` field additions (M22 / ADR-7 item 6c):** a new
  capability field is additive **only if** both reference constructors
  (`DriverCapabilities::none()` and `::full()`) carry a value for it.
  Out-of-tree drivers must build their declaration from a reference
  constructor (struct literals re-compile against new fields by design —
  that is the honest price of naming your surface directly).

## Changelog policy

- The root [`CHANGELOG.md`](../CHANGELOG.md) follows the "Keep a Changelog"
  format with an `Unreleased` section between releases. Entries are
  crate-attributed (`core:`, `jet:`, …) because the family releases in
  lockstep.
- Every user-visible change lands in `Unreleased` when committed: wire and
  snapshot changes, behavior changes, capability changes, compatibility-matrix
  claims, MSRV bumps. Internal refactors without observable surface do not
  get an entry.
- The release step turns `Unreleased` into the version heading with the
  release date; nothing ships without its changelog section.

## Release checklist (per release)

1. Canonical verification set green on CI `main` (fmt, clippy, tests,
   coverage ≥ 95 %, file-size, hygiene, oracle).
2. Bump the workspace version, date the changelog section.
3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` is clean
   (docs.rs is part of the published surface).
4. `cargo publish --dry-run` per crate **in the order above**; every package
   must build cleanly (`cargo package --list` shows no stray files).
5. Publish in order; verify `cargo install datacraft-mcp` resolves.
6. Push the annotated `v<version>` tag — `release.yml` builds the three
   platform archives (binaries + samples + third-party notices), runs the
   packaged smoke, collects sha256 checksums and drafts the GitHub release;
   a human reviews and publishes the draft (PLAYBOOK §9).
7. Re-verify the README compatibility matrix and date-stamp it.
