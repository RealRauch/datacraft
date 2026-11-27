//! German catalog of the migration-readiness window (phase 103, step
//! 103.2; item M134). `Close`, `Macros`, `VBA modules`, `form` and
//! `report` reuse the existing catalog entries.

pub(super) const TABLE: &[(&str, &str)] = &[
    ("Migration readiness", "Migrationsbereitschaft"),
    ("Refresh", "Aktualisieren"),
    ("Effort summary", "Aufwandsübersicht"),
    (
        "{running} of {total} objects run",
        "{running} von {total} Objekten laufen",
    ),
    ("not covered: {n}", "nicht abgedeckt: {n}"),
    ("Linked tables", "Verknüpfte Tabellen"),
    ("Native designs to migrate", "Nativ zu migrierende Entwürfe"),
    ("Native queries", "Native Abfragen"),
    (
        "{n} of {total} native queries are outside the executable subset",
        "{n} von {total} nativen Abfragen liegen außerhalb der ausführbaren Teilmenge",
    ),
    ("Unsupported features", "Nicht unterstützte Funktionen"),
    (
        "no unsupported features",
        "keine nicht unterstützten Funktionen",
    ),
    ("Macro execution", "Makro-Ausführung"),
    ("VBA execution", "VBA-Ausführung"),
    ("Native definitions", "Native Definitionen"),
    ("Decode error", "Dekodierfehler"),
    ("VBA estimate", "VBA-Abschätzung"),
    ("no VBA project", "kein VBA-Projekt"),
    (
        "{percent}% of {code_lines} code lines are candidates",
        "{percent}% von {code_lines} Codezeilen sind Kandidaten",
    ),
    (
        "{name}: {candidates} of {lines} candidate lines",
        "{name}: {candidates} von {lines} Kandidatenzeilen",
    ),
    ("SQL literals", "SQL-Literale"),
    ("markers", "Marker"),
    ("no linked tables", "keine verknüpften Tabellen"),
    ("target unknown", "Ziel unbekannt"),
    ("Objects", "Objekte"),
    ("table", "Tabelle"),
    ("linked table", "verknüpfte Tabelle"),
    ("query", "Abfrage"),
    ("native query", "native Abfrage"),
    ("view", "Ansicht"),
    ("native view", "native Ansicht"),
    ("native form", "natives Formular"),
    ("native report", "nativer Bericht"),
    ("macro", "Makro"),
    ("runs", "läuft"),
    ("not covered", "nicht abgedeckt"),
];
