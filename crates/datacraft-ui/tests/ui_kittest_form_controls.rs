//! Form runtime checkbox and combo suite (phase 64, M70; ADR-8).
//!
//! The checkbox toggles two-state through the record buffer (Null shows as
//! No); the combo lists every row of its table/query row source and accepts
//! free text like Access `LimitToList = No`.

#[path = "support/form_edit.rs"]
mod form_edit;

use datacraft_core::schema::FieldValue;
use eframe::egui;
use egui_kittest::kittest::Queryable;
use form_edit::*;

#[test]
fn a_checkbox_click_toggles_true_to_false_and_writes_with_the_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "ActiveBox")
        .click();
    harness.run();
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "the click stages the value, it does not write"
    );

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached_value(&harness, tab, 0, 3),
        Some(FieldValue::Boolean(false)),
        "leaving the record writes the toggled value"
    );
}

#[test]
fn a_null_checkbox_shows_unchecked_and_a_click_writes_true() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Next >").click();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "ActiveBox")
        .click();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness.get_by_label("< Prev").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached_value(&harness, tab, 1, 3),
        Some(FieldValue::Boolean(true)),
        "a Null checkbox displays as No and the click writes Yes (Access TripleState=False)"
    );
}

#[test]
fn a_combo_selection_comes_from_the_row_source_rows() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "NameCombo values")
        .click();
    harness.run();
    harness.get_by_label("mika@example.com").click();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached(&harness, tab, 0, 1),
        "mika@example.com",
        "the selected display value is written to the bound field"
    );
}

#[test]
fn a_combo_accepts_free_text() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameCombo");
    type_into(&mut harness, "X");
    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(cached(&harness, tab, 0, 1), "LenaX");
}
