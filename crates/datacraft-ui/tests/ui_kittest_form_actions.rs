//! Form runtime filter and button suite (phase 64, M69/M71; ADR-8).
//!
//! A form filter narrows the record set and keeps the physical record when
//! it changes; command buttons dispatch exactly one action, and an empty
//! action answers precisely instead of doing nothing.

#[path = "support/form_edit.rs"]
mod form_edit;

use datacraft_core::command::Command;
use datacraft_core::form::FormFilter;
use egui_kittest::kittest::Queryable;
use form_edit::*;

#[test]
fn a_form_filter_narrows_the_record_set_and_clears() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();
    assert_eq!(displayed(&harness, "NameBox"), "Lena");

    harness
        .state_mut()
        .dispatch(Command::SetFormFilter(FormFilter {
            form: "Form1".into(),
            criteria: Some("[Name] Like 'L*'".into()),
        }));
    harness.run();
    assert!(
        tree_contains(&harness, "Record 1 of 2"),
        "the filtered count drives the navigation box"
    );
    assert_eq!(displayed(&harness, "NameBox"), "Lena");

    harness.get_by_label("Next >").click();
    harness.run();
    assert!(tree_contains(&harness, "Record 2 of 2"));
    assert_eq!(
        displayed(&harness, "NameBox"),
        "Lars",
        "navigation stays inside the filtered set"
    );

    harness.get_by_label("Clear filter").click();
    harness.run();
    assert!(
        tree_contains(&harness, "Record 3 of 3"),
        "clearing keeps the current record visible (Access semantics)"
    );
    assert_eq!(displayed(&harness, "NameBox"), "Lars");
}

#[test]
fn a_button_with_an_action_dispatches_one_command() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();
    assert_eq!(harness.state().snapshot().tabs.len(), 1);

    harness.get_by_label("Open").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    assert_eq!(
        snapshot.tabs.len(),
        2,
        "one OpenObject dispatch opened Second"
    );
    assert_eq!(snapshot.tabs[1].object_name, "Second");
}

#[test]
fn a_button_without_an_action_reports_precisely() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Empty").click();
    harness.run();

    assert_eq!(
        harness.state().snapshot().tabs.len(),
        1,
        "the empty action dispatches nothing"
    );
    let error = harness
        .state()
        .last_error()
        .expect("the empty action reports");
    assert!(error.contains("action"), "{error}");
}

#[test]
fn a_filter_button_narrows_the_form() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Filter").click();
    harness.run();
    harness.run();
    assert!(
        tree_contains(&harness, "Record 1 of 2"),
        "the button applied the form filter"
    );
}
