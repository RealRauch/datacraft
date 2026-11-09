//! Form runtime bound-field editing suite (phase 64, M68; ADR-8).
//!
//! Typing edits a per-record buffer; exactly one `UpdateRow` is dispatched
//! when the record is left, never per keystroke or focus move. `Esc` once
//! reverts the field, `Esc` twice the record. A mistyped value keeps the
//! buffer and reports precisely.

#[path = "support/form_edit.rs"]
mod form_edit;

use datacraft_core::command::Command;
use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};
use form_edit::*;

#[test]
fn an_edit_is_buffered_until_the_record_is_left() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "X");
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "Enter updates the control, not the database"
    );

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "leaving the record dispatches exactly one UpdateRow"
    );
    assert_eq!(
        cached(&harness, tab, 0, 1),
        "LenaX",
        "the record change writes the buffered value"
    );
}

#[test]
fn escape_once_reverts_the_field_and_twice_the_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "X");
    click_input(&mut harness, "EmailBox");
    type_into(&mut harness, "Y");

    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "Esc twice discards the whole record"
    );
    assert_eq!(cached(&harness, tab, 0, 1), "Lena");
    assert_eq!(cached(&harness, tab, 0, 2), "lena@example.com");
}

#[test]
fn a_mistyped_value_keeps_the_buffer_and_reports_precisely() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "IdBox");
    type_into(&mut harness, "abc");

    let error = harness
        .state()
        .last_error()
        .expect("the failed parse reports")
        .to_owned();
    assert!(error.contains("Id"), "the failure names the field: {error}");
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    let values: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let value = node.accesskit_node().value().unwrap_or_default();
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect();
    assert!(
        values.iter().any(|value| value == "1abc"),
        "the buffered text stays visible: {values:?}"
    );
    assert_eq!(cached(&harness, tab, 0, 0), "1");
}

#[test]
fn closing_the_tab_writes_the_buffered_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "Z");
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness
        .state_mut()
        .dispatch(Command::CloseTab { tab_id: tab });
    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "closing the tab saves the pending record"
    );
    assert!(
        harness.state().last_error().is_none(),
        "the save on close succeeds"
    );
}
