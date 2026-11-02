//! Contract suite for the ribbon specification (step 5.2, item H18;
//! contextual tabs in step 21.1, item U5).
//!
//! ARCHITECTURE §5 test-ID convention: `ribbon.<tab>.<group>.<action>`.
//! The spec is static data in core (tested here); the UI renders buttons
//! from it and maps enabled actions to commands — no dispatch logic in
//! paint code beyond that mapping.

use datacraft_core::ribbon::{ActionState, rendered_tabs, ribbon_spec};

const DISPATCHABLE_ACTIONS: [&str; 8] = [
    "ribbon.home.view.toggle_nav_pane",
    "ribbon.home.views.view_mode",
    "ribbon.create.queries.query_design",
    "ribbon.create.forms.form_design",
    "ribbon.create.reports.report_design",
    "ribbon.create.tables.table",
    "ribbon.database_tools.relationships.relationships",
    "ribbon.database_tools.queries.query_sql",
];

#[test]
fn the_spec_carries_the_five_access_tabs_in_order() {
    let spec = ribbon_spec();
    let ids: Vec<&str> = spec.tabs.iter().map(|t| t.id).collect();
    assert_eq!(
        ids,
        ["file", "home", "create", "external_data", "database_tools"]
    );
    let labels: Vec<&str> = spec.tabs.iter().map(|t| t.label).collect();
    assert_eq!(
        labels,
        ["File", "Home", "Create", "External Data", "Database Tools"]
    );
}

#[test]
fn the_file_tab_delegates_to_the_backstage() {
    let file = &ribbon_spec().tabs[0];
    assert_eq!(file.id, "file");
    assert!(
        file.groups.is_empty(),
        "File is a full-page backstage, not a ribbon tab with inline inputs: {:?}",
        file.groups
    );
}

#[test]
fn contextual_tabs_cover_every_kind_with_a_design_view() {
    let spec = ribbon_spec();
    let contexts: Vec<&str> = spec
        .contextual_tabs
        .iter()
        .map(|entry| entry.context)
        .collect();
    assert_eq!(contexts, ["table", "query", "form", "report"]);
    let ids: Vec<&str> = spec
        .contextual_tabs
        .iter()
        .map(|entry| entry.tab.id)
        .collect();
    assert_eq!(
        ids,
        ["table_tools", "query_tools", "form_tools", "report_tools"]
    );
    let labels: Vec<&str> = spec
        .contextual_tabs
        .iter()
        .map(|entry| entry.tab.label)
        .collect();
    assert_eq!(
        labels,
        ["Table Tools", "Query Tools", "Form Tools", "Report Tools"]
    );
}

#[test]
fn rendered_tabs_appends_the_matching_contextual_tab_last() {
    let base: Vec<&str> = rendered_tabs(None).iter().map(|tab| tab.id).collect();
    assert_eq!(
        base,
        ["file", "home", "create", "external_data", "database_tools"]
    );
    let table: Vec<&str> = rendered_tabs(Some("table"))
        .iter()
        .map(|tab| tab.id)
        .collect();
    assert_eq!(
        table,
        [
            "file",
            "home",
            "create",
            "external_data",
            "database_tools",
            "table_tools"
        ]
    );
    let read_only: Vec<&str> = rendered_tabs(Some("macro"))
        .iter()
        .map(|tab| tab.id)
        .collect();
    assert_eq!(
        read_only,
        ["file", "home", "create", "external_data", "database_tools"],
        "kinds without a design view get no contextual tab"
    );
    for kind in ["query", "form", "report"] {
        assert_eq!(
            rendered_tabs(Some(kind)).last().map(|tab| tab.id),
            Some(match kind {
                "query" => "query_tools",
                "form" => "form_tools",
                _ => "report_tools",
            })
        );
    }
}

#[test]
fn every_action_has_a_unique_convention_test_id() {
    let spec = ribbon_spec();
    let mut ids: Vec<String> = Vec::new();
    let tabs = spec
        .tabs
        .iter()
        .chain(spec.contextual_tabs.iter().map(|entry| &entry.tab));
    for tab in tabs {
        for group in &tab.groups {
            for action in &group.actions {
                let id = format!("ribbon.{}.{}.{}", tab.id, group.id, action.id);
                assert!(!ids.contains(&id), "duplicate ribbon action id {id}");
                ids.push(id);
            }
            assert!(!group.label.is_empty());
        }
    }
    assert!(ids.len() >= 8, "the skeleton carries {ids:?}");
    for id in &ids {
        let parts: Vec<&str> = id.split('.').collect();
        assert_eq!(parts.len(), 4, "convention shape violated: {id}");
        assert!(
            parts
                .iter()
                .skip(1)
                .all(|p| p.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
            "segments must be snake_case: {id}"
        );
    }
}

#[test]
fn enabled_actions_exactly_match_the_ui_dispatch_mapping() {
    let spec = ribbon_spec();
    let mut enabled: Vec<String> = Vec::new();
    for tab in &spec.tabs {
        for group in &tab.groups {
            for action in &group.actions {
                if matches!(action.state, ActionState::Enabled) {
                    enabled.push(format!("ribbon.{}.{}.{}", tab.id, group.id, action.id));
                }
            }
        }
    }
    let mut expected = DISPATCHABLE_ACTIONS.map(str::to_owned).to_vec();
    expected.sort();
    enabled.sort();
    assert_eq!(enabled, expected, "UI mapping and spec must agree exactly");
}

#[test]
fn contextual_tab_actions_are_dispatchable() {
    for entry in &ribbon_spec().contextual_tabs {
        for group in &entry.tab.groups {
            for action in &group.actions {
                assert!(
                    matches!(action.state, ActionState::Enabled),
                    "{} arrives in a later phase, not as a contextual command",
                    action.id
                );
            }
        }
    }
}

#[test]
fn quick_access_entries_are_declared_in_order() {
    let quick_access = &ribbon_spec().quick_access;
    let ids: Vec<&str> = quick_access.iter().map(|entry| entry.id).collect();
    assert_eq!(ids, ["save_design", "undo", "redo"]);
    let labels: Vec<&str> = quick_access.iter().map(|entry| entry.label).collect();
    assert_eq!(labels, ["Save", "Undo", "Redo"]);
}

#[test]
fn every_quick_access_entry_is_declared_enabled() {
    let quick_access = &ribbon_spec().quick_access;
    for entry in quick_access {
        assert!(
            matches!(entry.state, ActionState::Enabled),
            "{} is dispatchable today — the QAT gates Undo/Redo from the snapshot depth",
            entry.id
        );
    }
}

#[test]
fn quick_access_save_is_a_stable_command_id() {
    let quick_access = &ribbon_spec().quick_access;
    assert!(
        matches!(quick_access[0].state, ActionState::Enabled),
        "Save is dispatchable today"
    );
    assert_eq!(
        quick_access[0].id, "save_design",
        "the QAT keeps the command id shared with the backstage Save page"
    );
}

#[test]
fn disabled_actions_declare_their_arriving_phase() {
    let spec = ribbon_spec();
    let tabs = spec
        .tabs
        .iter()
        .chain(spec.contextual_tabs.iter().map(|entry| &entry.tab));
    for tab in tabs {
        for group in &tab.groups {
            for action in &group.actions {
                match &action.state {
                    ActionState::DisabledIn { phase } => {
                        assert!(
                            phase.starts_with("Phase "),
                            "{} arrives in a named phase: {phase}",
                            action.id
                        );
                    }
                    ActionState::Enabled => {}
                }
            }
        }
    }
}

#[test]
fn labels_are_non_empty_everywhere() {
    let spec = ribbon_spec();
    let tabs = spec
        .tabs
        .iter()
        .chain(spec.contextual_tabs.iter().map(|entry| &entry.tab));
    for tab in tabs {
        assert!(!tab.label.is_empty());
        for group in &tab.groups {
            assert!(!group.label.is_empty());
            for action in &group.actions {
                assert!(!action.label.is_empty());
            }
        }
    }
}
