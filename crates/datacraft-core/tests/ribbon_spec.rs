//! Contract suite for the ribbon specification (step 5.2, item H18).
//!
//! ARCHITECTURE §5 test-ID convention: `ribbon.<tab>.<group>.<action>`.
//! The spec is static data in core (tested here); the UI renders buttons
//! from it and maps enabled actions to commands — no dispatch logic in
//! paint code beyond that mapping.

use datacraft_core::ribbon::{ActionState, ribbon_spec};

const DISPATCHABLE_ACTIONS: [&str; 6] = [
    "ribbon.file.open.open_database",
    "ribbon.file.database.close_database",
    "ribbon.home.view.toggle_nav_pane",
    "ribbon.create.queries.query_design",
    "ribbon.create.forms.form_design",
    "ribbon.database_tools.relationships.relationships",
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
fn every_action_has_a_unique_convention_test_id() {
    let spec = ribbon_spec();
    let mut ids: Vec<String> = Vec::new();
    for tab in &spec.tabs {
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
fn disabled_actions_declare_their_arriving_phase() {
    let spec = ribbon_spec();
    for tab in &spec.tabs {
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
    for tab in &spec.tabs {
        assert!(!tab.label.is_empty());
        for group in &tab.groups {
            assert!(!group.label.is_empty());
            for action in &group.actions {
                assert!(!action.label.is_empty());
            }
        }
    }
}
