//! German catalog: dialogs and read-only source-object surfaces
//! (phase 72/73 — split from `ui_chrome.rs` for the file-size gate).

/// German translations; the English column is the key.
pub static TABLE: &[(&str, &str)] = &[
    ("New name", "Neuer Name"),
    ("Read-only native view", "Schreibgeschützte native View"),
    ("SQLite version", "SQLite-Version"),
    ("Page size", "Seitengröße"),
    ("Journal mode", "Journal-Modus"),
    ("Encoding", "Kodierung"),
    ("User version", "Benutzerversion"),
    ("Integrity check", "Integritätsprüfung"),
    ("ok", "ok"),
    (
        "The database's own integrity check reported: {reason}",
        "Die Integritätsprüfung der Datenbank meldet: {reason}",
    ),
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
