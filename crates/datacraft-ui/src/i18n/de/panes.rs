//! German catalog of the pane and shell additions from phases 90-92
//! (split from `ui_chrome.rs` for the file-size gate, item L30).

pub(super) const TABLE: &[(&str, &str)] = &[
    // Task-pane host (step 92.2, item M108).
    ("Dock", "Andocken"),
    ("Dock Right", "Rechts andocken"),
    ("Dock Bottom", "Unten andocken"),
    ("Float", "Schwebend"),
    // Nav-pane search (step 92.3, item L48).
    ("Search objects", "Objekte suchen"),
    (
        "No objects match {query}",
        "Keine Objekte passen zu {query}",
    ),
    // Nav-pane selection (step 92.4, item L48).
    (
        "Open {count} selected",
        "{count} ausgewählte Objekte öffnen",
    ),
    // Nav-pane linked tables (phase 100, step 100.3; item M131).
    ("Linked tables ({count})", "Verknüpfte Tabellen ({count})"),
    ("Linked to {target}", "Verknüpft mit {target}"),
    (
        "Link records only — the targets are not resolved.",
        "Nur Verknüpfungsdatensätze — die Ziele werden nicht aufgelöst.",
    ),
    // Nav-pane groups and details view (step 92.5, item L48).
    ("Object Details", "Objektdetails"),
    ("Custom Groups", "Eigene Gruppen"),
    (
        "New group from {count} selected",
        "Neue Gruppe aus {count} ausgewählten",
    ),
    ("New Group", "Neue Gruppe"),
    ("Group name", "Gruppenname"),
    ("{count} selected objects", "{count} ausgewählte Objekte"),
    ("Remove from group", "Aus Gruppe entfernen"),
    ("Delete group", "Gruppe löschen"),
    ("Colour", "Farbe"),
    ("Red", "Rot"),
    ("Orange", "Orange"),
    ("Yellow", "Gelb"),
    ("Green", "Grün"),
    ("Blue", "Blau"),
    ("Purple", "Lila"),
    ("Teal", "Türkis"),
    ("Grey", "Grau"),
    // Low-power rendering (step 92.6, item L51).
    ("Low-power rendering", "Energiearme Darstellung"),
    (
        "Low-power rendering ({ms} ms)",
        "Energiearme Darstellung ({ms} ms)",
    ),
    ("On", "Ein"),
    ("Off", "Aus"),
];

/// German catalog: the datasheet value list (phase 101, step 101.2; item M132).
pub(super) const VALUE_LIST: &[(&str, &str)] = &[
    ("Filter by Value…", "Nach Wert filtern…"),
    ("Filter by Value: {column}", "Nach Wert filtern: {column}"),
    ("(Blanks)", "(Leer)"),
    (
        "More values exist — narrow the column with a criteria.",
        "Es gibt weitere Werte — schränken Sie die Spalte mit einem Kriterium ein.",
    ),
];

/// German catalog: the datasheet find/replace window (phase 101, step
/// 101.3; item M132).
pub(super) const FIND_REPLACE: &[(&str, &str)] = &[
    ("Find and Replace…", "Suchen und Ersetzen…"),
    ("Find and Replace", "Suchen und Ersetzen"),
    ("Find", "Suchen"),
    ("Replace with", "Ersetzen durch"),
    ("Match case", "Groß-/Kleinschreibung beachten"),
    ("Whole field", "Ganzes Feld"),
    ("All text columns", "Alle Textspalten"),
    ("Column", "Spalte"),
    ("Replace All", "Alle ersetzen"),
    (
        "Replacements staged: {count}",
        "Ersetzungen vorgemerkt: {count}",
    ),
    (
        "Search the visible rows and stage replacements.",
        "Durchsucht die sichtbaren Zeilen und merkt Ersetzungen vor.",
    ),
];
