//! German catalog: dialogs and read-only source-object surfaces
//! (phase 72/73 — split from `ui_chrome.rs` for the file-size gate).

/// German translations; the English column is the key.
pub static TABLE: &[(&str, &str)] = &[
    ("New name", "Neuer Name"),
    ("Read-only native view", "Schreibgeschützte native View"),
    (
        "File objects (read-only)",
        "Dateiobjekte (schreibgeschützt)",
    ),
    ("Index", "Index"),
    ("Trigger", "Trigger"),
    ("unique", "eindeutig"),
    ("automatic", "automatisch"),
    (
        "The native view could not be read: {reason}",
        "Die native View konnte nicht gelesen werden: {reason}",
    ),
    ("Rename {name}", "„{name}“ umbenennen"),
    ("Current name: {name}", "Aktueller Name: {name}"),
    (
        "Rename the active table, query, form or report.",
        "Benennt die aktive Tabelle, Abfrage, Formular oder Bericht um.",
    ),
];
