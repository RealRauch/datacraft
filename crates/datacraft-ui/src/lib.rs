//! DataCraft desktop shell (eframe/egui) — library root.
//!
//! The four-region shell (ribbon, navigation pane, tabbed workspace,
//! status bar) driven by the headless core (`docs/ARCHITECTURE.md` §5):
//! datasheet with the new-record row, read-only design grid, QBE grid,
//! SQL window, form/report designers with palettes, relationship mapper
//! and the sidecar actions. The `datacraft` binary is a thin composition
//! root over this library; the headless kittest suite in `tests/` drives
//! the same widgets.
//!
//! Coverage policy: this crate is rendering glue only — logic lives in
//! `datacraft-core`; the crate is excluded from the coverage gate
//! (exception registry in `docs/TESTING.md`).

pub mod app;
pub mod appearance;
pub mod availability;
pub mod backstage;
pub mod datasheet;
pub mod design;
pub mod designer_palette;
pub mod dialogs;
pub mod expression_window;
pub mod flows;
pub mod form_design;
pub mod form_runtime;
pub mod i18n;
pub mod icons;
pub mod import_window;
pub mod keys;
pub mod labels;
pub mod layout_probe;
pub mod macro_viewer;
pub mod nav_pane;
pub mod password_prompt;
pub mod qbe;
pub mod relationships;
pub mod report_design;
pub mod ribbon;
pub mod settings;
pub mod shell;
pub mod sql_window;
pub mod theme;
pub mod vba_viewer;
pub mod window;
pub mod workspace;

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod settings_tests;
