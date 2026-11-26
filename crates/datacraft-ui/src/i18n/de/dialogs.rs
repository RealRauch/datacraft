//! German catalog: dialogs and read-only source-object surfaces
//! (phase 72/73 — split from `ui_chrome.rs` for the file-size gate).

/// German translations; the English column is the key.
pub static TABLE: &[(&str, &str)] = &[
    ("New name", "Neuer Name"),
    ("Read-only native view", "Schreibgeschützte native View"),
    (
        "Native forms and reports ({count})",
        "Native Formulare und Berichte ({count})",
    ),
    (
        "Definitions are not decoded — these objects are listed read-only.",
        "Definitionen werden nicht dekodiert — diese Objekte sind schreibgeschützt aufgelistet.",
    ),
    ("SQLite version", "SQLite-Version"),
    ("Page size", "Seitengröße"),
    ("Journal mode", "Journal-Modus"),
    ("Encoding", "Kodierung"),
    ("User version", "Benutzerversion"),
    ("Integrity check", "Integritätsprüfung"),
    ("Compact Database", "Datenbank komprimieren"),
    (
        "the open source cannot be compacted",
        "Die geöffnete Quelle kann nicht komprimiert werden",
    ),
    ("Database compacted", "Datenbank komprimiert"),
    ("Export PDF…", "PDF exportieren…"),
    (
        "Export the report's print preview to a PDF file.",
        "Exportiert die Druckvorschau des Berichts als PDF-Datei.",
    ),
    (
        "This build does not include the PDF writer.",
        "Dieser Build enthält keinen PDF-Writer.",
    ),
    (
        "Report exported to {path} ({pages} pages)",
        "Bericht exportiert nach {path} ({pages} Seiten)",
    ),
    ("Export CSV…", "CSV exportieren…"),
    ("Export JSON…", "JSON exportieren…"),
    (
        "Export the table to a CSV file.",
        "Exportiert die Tabelle als CSV-Datei.",
    ),
    (
        "Export the table to a JSON document (schema and rows).",
        "Exportiert die Tabelle als JSON-Dokument (Schema und Datensätze).",
    ),
    (
        "Table exported to {path} ({rows} rows)",
        "Tabelle exportiert nach {path} ({rows} Datensätze)",
    ),
    (
        "native SQLite · read-only",
        "natives SQLite · schreibgeschützt",
    ),
    ("Query plan", "Abfrageplan"),
    (
        "Native SQLite dialect: indexes, views and EXPLAIN QUERY PLAN run on the file itself.",
        "Natives SQLite-Dialekt: Indizes, Views und EXPLAIN QUERY PLAN laufen direkt auf der Datei.",
    ),
    ("Busy timeout", "Busy-Timeout"),
    ("STRICT tables", "STRICT-Tabellen"),
    ("WITHOUT ROWID tables", "WITHOUT-ROWID-Tabellen"),
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
