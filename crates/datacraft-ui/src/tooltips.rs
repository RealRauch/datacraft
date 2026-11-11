//! Tooltip registry (step 68.2, item M74).
//!
//! One lookup per action/control id; the English sentence is the i18n key
//! (the German catalog resolves it). The registry is spec-complete by
//! construction: the tests enumerate the ribbon spec, the quick access
//! strip and the backstage inventory, so a new control cannot ship without
//! its explanation. Dynamic surfaces (template cards, recent entries) name
//! themselves and stay outside the static inventory.

use eframe::egui;

use crate::i18n::t;

/// Ribbon action tooltips, keyed by `(tab, group, action)` — the same
/// triple that composes the AccessKit test id.
const RIBBON_TOOLTIPS: &[(&str, &str, &str, &str)] = &[
    (
        "home",
        "view",
        "toggle_nav_pane",
        "Show or hide the navigation pane.",
    ),
    (
        "home",
        "views",
        "view_mode",
        "Switch between Datasheet and Design view.",
    ),
    (
        "create",
        "tables",
        "table",
        "Create a new table and open it in the datasheet.",
    ),
    (
        "create",
        "tables",
        "table_design",
        "Create a new table and open its design view.",
    ),
    (
        "create",
        "queries",
        "query_design",
        "Create a new query and open the query designer.",
    ),
    (
        "create",
        "forms",
        "form_design",
        "Create a new form and open the form designer.",
    ),
    (
        "create",
        "reports",
        "report_design",
        "Create a new report and open the report designer.",
    ),
    (
        "external_data",
        "import",
        "import_file",
        "Import a CSV file as a new table of the open database.",
    ),
    (
        "database_tools",
        "relationships",
        "relationships",
        "Show or hide the relationship mapper.",
    ),
    (
        "database_tools",
        "queries",
        "query_sql",
        "Open the SQL window.",
    ),
    (
        "database_tools",
        "queries",
        "query_expression",
        "Open the expression window.",
    ),
    (
        "table_tools",
        "views",
        "view_mode",
        "Switch between Datasheet and Design view.",
    ),
    (
        "query_tools",
        "views",
        "view_mode",
        "Switch between Datasheet and Design view.",
    ),
    (
        "form_tools",
        "views",
        "view_mode",
        "Switch between Datasheet and Design view.",
    ),
    (
        "report_tools",
        "views",
        "view_mode",
        "Switch between Datasheet and Design view.",
    ),
];

/// Quick Access entries reuse the ribbon action ids (step 21.2, U6).
const QAT_TOOLTIPS: &[(&str, &str)] = &[
    ("save_design", "Save the design sidecar (.datacraft)."),
    ("undo", "Undo the last row write (Ctrl+Z)."),
    ("redo", "Redo the last undone row write (Ctrl+Y)."),
];

/// Backstage control tooltips, keyed by the `backstage.<page>.<control>`
/// id the paint code already carries.
const BACKSTAGE_TOOLTIPS: &[(&str, &str)] = &[
    (
        "backstage.home.open_page",
        "Open the Open page to pick a database.",
    ),
    ("backstage.home.close_database", "Close the open database."),
    (
        "backstage.home.new_table",
        "Create a new table and open it in the datasheet.",
    ),
    (
        "backstage.home.save_page",
        "Open the Save page for the design sidecar and copies.",
    ),
    (
        "backstage.save.browse",
        "Choose the target file for the copy.",
    ),
    (
        "backstage.save.save_copy",
        "Write a copy of the open database to the target path.",
    ),
    (
        "backstage.save.change_password",
        "Protect the open database with a new password.",
    ),
    (
        "backstage.save.remove_password",
        "Remove the database password.",
    ),
    (
        "backstage.save.save_design",
        "Save the design sidecar (.datacraft).",
    ),
    (
        "backstage.save.load_design",
        "Load the design sidecar into the open database.",
    ),
    ("backstage.save.close_database", "Close the open database."),
    (
        "backstage.new.browse",
        "Choose the path for the new database.",
    ),
    (
        "backstage.new.create",
        "Create the database from the selected template and open it.",
    ),
    ("backstage.open.browse", "Choose a database file to open."),
    (
        "backstage.open.browse_folder",
        "Open a folder of CSV tables (or a database inside it).",
    ),
    (
        "backstage.open.open_database",
        "Open the entered path or file:// URI.",
    ),
];

/// The static backstage control inventory: every dispatchable Save/New/Open
/// control. Template cards and recent entries are dynamic and name
/// themselves.
pub const BACKSTAGE_CONTROLS: &[&str] = &[
    "backstage.home.open_page",
    "backstage.home.close_database",
    "backstage.home.new_table",
    "backstage.home.save_page",
    "backstage.save.browse",
    "backstage.save.save_copy",
    "backstage.save.change_password",
    "backstage.save.remove_password",
    "backstage.save.save_design",
    "backstage.save.load_design",
    "backstage.save.close_database",
    "backstage.new.browse",
    "backstage.new.create",
    "backstage.open.browse",
    "backstage.open.browse_folder",
    "backstage.open.open_database",
];

/// The tooltip of one ribbon action (already the English i18n key).
pub fn ribbon_tooltip(tab: &str, group: &str, action: &str) -> Option<&'static str> {
    RIBBON_TOOLTIPS
        .iter()
        .find(|(t, g, a, _)| *t == tab && *g == group && *a == action)
        .map(|(_, _, _, tip)| *tip)
}

/// The tooltip of one quick access action.
pub fn qat_tooltip(action: &str) -> Option<&'static str> {
    QAT_TOOLTIPS
        .iter()
        .find(|(id, _)| *id == action)
        .map(|(_, tip)| *tip)
}

/// The tooltip of one static backstage control.
pub fn backstage_tooltip(control: &str) -> Option<&'static str> {
    BACKSTAGE_TOOLTIPS
        .iter()
        .find(|(id, _)| *id == control)
        .map(|(_, tip)| *tip)
}

/// Every registry key, for the i18n coverage test (dynamic keys).
pub fn all_keys() -> impl Iterator<Item = &'static str> {
    RIBBON_TOOLTIPS
        .iter()
        .map(|(_, _, _, tip)| *tip)
        .chain(QAT_TOOLTIPS.iter().map(|(_, tip)| *tip))
        .chain(BACKSTAGE_TOOLTIPS.iter().map(|(_, tip)| *tip))
}

/// Decorates a ribbon response with its action tooltip, if the registry
/// carries one. The coverage test guarantees it does.
pub fn ribbon_response(
    response: egui::Response,
    tab: &str,
    group: &str,
    action: &str,
) -> egui::Response {
    match ribbon_tooltip(tab, group, action) {
        Some(tip) => response.on_hover_text(t(tip)),
        None => response,
    }
}

/// Decorates a quick access response with its entry tooltip.
pub fn qat_response(response: egui::Response, action: &str) -> egui::Response {
    match qat_tooltip(action) {
        Some(tip) => response.on_hover_text(t(tip)),
        None => response,
    }
}

/// Decorates a backstage response with its control tooltip.
pub fn backstage_response(response: egui::Response, control: &str) -> egui::Response {
    match backstage_tooltip(control) {
        Some(tip) => response.on_hover_text(t(tip)),
        None => response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datacraft_core::ribbon::{RibbonTab, rendered_tabs, ribbon_spec};

    fn all_tabs() -> impl Iterator<Item = &'static RibbonTab> {
        rendered_tabs(None)
            .into_iter()
            .chain(ribbon_spec().contextual_tabs.iter().map(|entry| &entry.tab))
    }

    #[test]
    fn every_ribbon_action_resolves_a_tooltip() {
        for tab in all_tabs() {
            for group in &tab.groups {
                for action in &group.actions {
                    assert!(
                        ribbon_tooltip(tab.id, group.id, action.id).is_some(),
                        "missing tooltip for ribbon.{}.{}.{}",
                        tab.id,
                        group.id,
                        action.id
                    );
                }
            }
        }
    }

    #[test]
    fn every_quick_access_entry_resolves_a_tooltip() {
        for entry in &ribbon_spec().quick_access {
            assert!(
                qat_tooltip(entry.id).is_some(),
                "missing QAT tooltip for {}",
                entry.id
            );
        }
    }

    #[test]
    fn every_inventoried_backstage_control_resolves_a_tooltip() {
        for control in BACKSTAGE_CONTROLS {
            assert!(
                backstage_tooltip(control).is_some(),
                "missing backstage tooltip for {control}"
            );
        }
    }

    #[test]
    fn unknown_ids_answer_none() {
        assert_eq!(ribbon_tooltip("home", "view", "nope"), None);
        assert_eq!(qat_tooltip("nope"), None);
        assert_eq!(backstage_tooltip("nope"), None);
    }

    /// The registry holds no entry the spec and the inventory do not name
    /// (a typo would otherwise sit dead while the coverage test passes).
    #[test]
    fn no_registry_entry_is_orphaned() {
        for (tab, group, action, _) in RIBBON_TOOLTIPS {
            let known = all_tabs().any(|candidate| {
                candidate.id == *tab
                    && candidate
                        .groups
                        .iter()
                        .any(|g| g.id == *group && g.actions.iter().any(|a| a.id == *action))
            });
            assert!(known, "orphaned ribbon tooltip {tab}.{group}.{action}");
        }
        for (action, _) in QAT_TOOLTIPS {
            assert!(
                ribbon_spec().quick_access.iter().any(|e| e.id == *action),
                "orphaned QAT tooltip {action}"
            );
        }
        for (control, _) in BACKSTAGE_TOOLTIPS {
            assert!(
                BACKSTAGE_CONTROLS.contains(control),
                "orphaned backstage tooltip {control}"
            );
        }
    }
}
