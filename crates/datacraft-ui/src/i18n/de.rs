//! German catalog (step 18.1 seed; completed in step 18.3, item I3).
//!
//! Every entry is `(English source, German translation)`. The English side
//! is the lookup key used by `t()`/`tr()`; the parity tests of step 18.3
//! keep the table complete, non-empty and in sync with the UI sources.

/// German translations; the English column is the key.
pub static TABLE: &[(&str, &str)] = &[
    ("File", "Datei"),
    ("{count} row", "{count} Zeile"),
    ("{count} rows", "{count} Zeilen"),
];

/// German translation of an English source string.
pub fn lookup(key: &str) -> Option<&'static str> {
    TABLE
        .iter()
        .find(|(english, _)| *english == key)
        .map(|(_, german)| *german)
}
