//! German catalog modules (step 18.3, item I3; ADR-4, ARCHITECTURE §15).
//!
//! Every entry is `(English source, German translation)`. The English side
//! is the lookup key used by `t()`/`tr()`; the parity tests in
//! `i18n/parity.rs` keep the modules complete, non-empty and in sync with
//! the UI sources, and enforce that every template keeps the English
//! placeholder set.
//!
//! The catalog is split by UI area (`ui_chrome` for ribbon/nav/shell,
//! `views` for documents and their grids) to stay below the file-size gate.

mod ui_chrome;
mod views;

/// All catalog modules; lookups scan each in order.
static MODULES: &[&[(&str, &str)]] = &[ui_chrome::TABLE, views::TABLE];

/// German translation of an English source string.
pub fn lookup(key: &str) -> Option<&'static str> {
    entries()
        .find(|(english, _)| *english == key)
        .map(|(_, german)| *german)
}

/// Every catalog entry (the parity tests consume this iterator).
pub fn entries() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    MODULES.iter().flat_map(|table| table.iter())
}
