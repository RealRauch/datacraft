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
