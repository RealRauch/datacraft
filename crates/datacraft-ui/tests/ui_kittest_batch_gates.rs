//! Capability-gating pins of the pending-edit batch (step 85.3, item
//! H61): non-atomic drivers keep the immediate single-cell path, cursor
//! moves inside a batch leave the staging alone, read-only sources never
//! stage. Assertions stay on the cached rows, the snapshot and the
//! semantic tree.

#[path = "batch_support.rs"]
mod batch_support;

use batch_support::*;
use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};

/// On a driver that cannot apply atomic batches (atomic_batch=false, the
/// mock's honest default and the read-only CSV-like shapes) the datasheet
/// keeps the immediate single-cell path: every edit dispatches at once
/// and nothing stages (step 85.3, item H61).
#[test]
fn non_atomic_drivers_keep_the_immediate_cell_path() {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..4)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    port.with_capabilities(DriverCapabilities {
        atomic_batch: false,
        ..DriverCapabilities::full()
    });
    let mut app = App::new(Box::new(port));
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    app.dispatch(Command::OpenDatabase {
        path: db_path().into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2X",
        "the immediate path writes through"
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "exactly one write command was dispatched"
    );
    assert!(
        !harness.state().has_staged_edits(tab),
        "nothing stages on a non-atomic driver"
    );
    assert!(harness.state().snapshot().pending_write.is_none());
}

/// Cursor moves inside a batch never disturb the staging: the staged
/// value stays put, and Escape discards the record under the NEW cursor
/// (step 85.3, item H61's cursor-moves edge case).
#[test]
fn the_cursor_moves_inside_a_batch_without_disturbing_staging() {
    let (app, tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");

    // Move the cursor to another record (arrow-down navigation from the
    // grid anchor; a click works too, so use the selector).
    harness.get_by_label("     1").click();
    harness.run();

    assert!(
        staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X"),
        "the staged value survives the cursor move: {:?}",
        staged_texts(&harness)
    );
    assert!(
        harness.state().has_staged_edits(tab),
        "the batch stays pending"
    );

    // Escape discards the record under the new cursor (row 0) — nothing
    // staged there, so the buffer keeps the row-2 edit.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    assert!(
        staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X"),
        "the row-2 edit is untouched by the row-0 discard"
    );
    assert!(harness.state().has_staged_edits(tab));
}

/// A read-only source never arms the editor and never stages, whatever
/// its batch capability claims (step 85.3, item H61).
#[test]
fn read_only_sources_never_stage() {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..4)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    // Batch-capable but not writable: batch_writes() checks both flags.
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
        write_rows: false,
        ..DriverCapabilities::full()
    });
    let mut app = App::new(Box::new(port));
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    app.dispatch(Command::OpenDatabase {
        path: db_path().into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    let mut harness = batch_harness(app);
    harness.run();

    // Two clicks into a cell never arm the editor on a read-only source.
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("alpha1").click();
    harness.run();
    harness.get_by_label("alpha1").click();
    harness.run();

    assert!(
        !harness.state().has_staged_edits(tab),
        "nothing stages on a read-only source"
    );
    assert_eq!(harness.state().snapshot().undo_depth, 0);
    let editors: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let n = node.accesskit_node();
            (format!("{:?}", n.role()) == "TextInput" && !n.is_disabled())
                .then(|| n.value().unwrap_or_default().to_owned())
        })
        .collect();
    assert!(
        editors.is_empty(),
        "no cell editor is armed on a read-only source: {editors:?}"
    );
}
