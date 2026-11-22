# datacraft-xlsx

Read-only Excel workbook driver for the
[DataCraft](https://github.com/RealRauch/datacraft) engine (ADR-7).

A workbook is a database; each sheet is a table. Values are read through
[calamine](https://crates.io/crates/calamine) (MIT, pure Rust) — formula
cells surface their saved cached value, macros are never read, executed
or modified. Supported families: `.xlsx`, `.xlsm`, `.xls` (BIFF8) and
`.xlsb`; every cell maps onto the DataCraft field vocabulary (text,
integer, float, date/time, boolean, empty).

The crate depends only on `datacraft-core`, the same out-of-tree driver
rule as `datacraft-csv`. It is composed by the binary roots (UI, CLI,
MCP) through `DetectingPort::with_fallback`; library crates never name
it. Every write affordance is declared unavailable, and the driver never
modifies the workbook.

See `docs/FEATURES.md` and `docs/COMPATIBILITY.md` for the verified
surface and its limits.
