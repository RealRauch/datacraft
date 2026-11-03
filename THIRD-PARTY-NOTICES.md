# Third-Party Notices

This file lists the third-party components shipped inside the DataCraft
release archives (the `datacraft`, `datacraft-cli` and `datacraft-mcp`
binaries plus the `samples/` folder). DataCraft itself is licensed under the
MIT OR Apache-2.0 license; this file only covers what we bundle on top of our
own code.

## Rust crates bundled into the binaries

The full transitive dependency graph of the shipped binaries is pinned in
`Cargo.lock`; every crate on crates.io carries its own license text in the
published package. The notable components:

| Component | License | Notes |
| --- | --- | --- |
| `eframe` / `egui` (incl. its windowing/rendering stack) | MIT OR Apache-2.0 | desktop shell of the `datacraft` binary |
| `serde` / `serde_json` | MIT OR Apache-2.0 | command/event and sidecar serialization |
| `image` | MIT OR Apache-2.0 | app icon loading (PNG) |
| `sys-locale` | MIT OR Apache-2.0 | default UI language detection |
| `csv` | Unlicense OR MIT | CSV directory driver, bundled via the composite fallback (ADR-7) |
| `rusqlite` / `libsqlite3-sys` | MIT | SQLite backend, statically linked (`bundled` feature) |
| SQLite (amalgamation bundled by `libsqlite3-sys`) | Public domain | no attribution required; upstream: https://www.sqlite.org |

## Fonts embedded in the `datacraft` binary

| Font | License | License text |
| --- | --- | --- |
| Inter | SIL Open Font License 1.1 | `assets/fonts/LICENSE-Inter.txt` |
| JetBrains Mono | SIL Open Font License 1.1 | `assets/fonts/LICENSE-JetBrainsMono.txt` |

Both fonts are embedded unmodified and remain under their original licenses.

## Committed test fixtures (repository test evidence, not shipped)

The databases under `fixtures/` are commit-time test evidence for the parser
suites; they are **not** part of the release archives. Every fixture carries
its `repo@commit` provenance and sha256 in `fixtures/manifest`, verified
offline by CI. Licenses and modifications:

| Fixture | Source | License | Modifications |
| --- | --- | --- | --- |
| `testV2000.mdb` (pristine) | `jahlborn/jackcess@be0a2139` (`src/test/data/V2000/testV2000.mdb`) | Apache-2.0 | none |
| `testV2000-access-touched.mdb` | descendant of the pristine jackcess file, rewritten once by real Microsoft Access on open (2026-01) | Apache-2.0 | system-table area rewritten by Access (documented in `docs/FIXTURE-SOURCES.md` §4) |
| `testV2007.accdb`, `testV2010.accdb` | `jahlborn/jackcess@be0a2139` | Apache-2.0 | none |
| `queryTestV2000.mdb` | `jahlborn/jackcess@be0a2139` | Apache-2.0 | none |
| `complexDataV2007.accdb` | `jahlborn/jackcess@be0a2139` | Apache-2.0 | none |
| `testV1997.mdb`, `test2V1997.mdb` | `jahlborn/jackcess@be0a2139` | Apache-2.0 | none |
| `attachmentsV2016.accdb` | `andipaetzold/mdb-reader@3dc637a` (`test/data/V2016/attachments.accdb`) | MIT | none |
| `passwordV2000.mdb`, `passwordV2000-none.mdb`, `passwordV2010.accdb`, `officeAgile42.accdb` | `andipaetzold/mdb-reader@3dc637a` | MIT | none (password-protected test data, password `"password"`) |
| `programmability.accdb` | self-created with Microsoft Access 16.0.20527.20112 by DataCraft contributors (recipe: `docs/FIXTURE-SOURCES.md`) | MIT OR Apache-2.0 (project's own) | is the original |
