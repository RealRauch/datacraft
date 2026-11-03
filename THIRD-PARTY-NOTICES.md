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
