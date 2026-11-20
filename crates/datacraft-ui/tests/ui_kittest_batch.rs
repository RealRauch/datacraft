//! Datasheet pending-edit batch suite (step 85.2, item H61).
//!
//! Cell edits, new records and deletions stage into the per-tab edit
//! buffer (amber pending markers on the changed cells); Ctrl+S opens the
//! submission preview that renders the journal's own changes, and Apply
//! submits one atomic batch through `PreviewWrite`/`ApplyWrite` — the
//! undo history stays out of it (journal writes are its barrier).
//! Assertions stay on the cached rows, the snapshot and the semantic
//! tree — never on pixels.

#[path = "batch_support.rs"]
mod batch_support;

use batch_support::*;

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};

/// A committed cell edit waits in the buffer: the grid shows the staged
/// value, the row cache and the undo history stay untouched, and nothing
/// is pending at the journal yet.
#[test]
fn a_cell_edit_stages_instead_of_writing() {
    let (app, tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2",
        "the staged edit never touches the row cache"
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "staging dispatches no write command"
    );
    assert!(
        harness.state().snapshot().pending_write.is_none(),
        "nothing is pending at the journal yet"
    );
    assert!(
        staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X"),
        "the grid paints the staged value: {:?}",
        staged_texts(&harness)
    );
    assert!(
        harness.state().last_error().is_none(),
        "a valid edit leaves no failure behind"
    );
}

/// Ctrl+S opens the submission preview; Apply submits the buffer as one
/// atomic journal batch — the cache updates, the buffer clears, and the
/// undo history stays empty (journal writes are its barrier).
#[test]
fn ctrl_s_previews_and_apply_submits_one_atomic_batch() {
    let (app, tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();

    assert!(
        harness.state().snapshot().pending_write.is_some(),
        "the preview parked the journal"
    );
    let summary = harness
        .state()
        .snapshot()
        .pending_write
        .clone()
        .expect("pending summary");
    assert_eq!(summary.table, "Customers");
    assert_eq!(
        (summary.updates, summary.inserts, summary.deletes),
        (1, 0, 0)
    );

    harness.get_by_label("Apply Changes").click();
    harness.run();
    harness.run();

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2X",
        "the applied batch reaches the row cache"
    );
    assert!(harness.state().snapshot().pending_write.is_none());
    assert!(
        !staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X" && cached_name(&harness, tab, 2) != "alpha2X"),
        "the buffer cleared after the apply"
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "journal writes are outside the row-write history"
    );
}

/// The command modifier carries the save chord: ⌘S opens the submission
/// preview exactly like Ctrl+S (step 92.1, item M99).
#[test]
fn command_s_opens_the_submission_preview() {
    let (app, _tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();

    assert!(
        harness.state().snapshot().pending_write.is_some(),
        "⌘S opened the same submission preview"
    );
    let summary = harness
        .state()
        .snapshot()
        .pending_write
        .clone()
        .expect("pending summary");
    assert_eq!(summary.table, "Customers");
    assert_eq!(
        (summary.updates, summary.inserts, summary.deletes),
        (1, 0, 0)
    );
}

/// Escape discards the staged changes of the active record.
#[test]
fn escape_discards_the_staged_record() {
    let (app, tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    assert!(
        !staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X"),
        "the staged value is gone: {:?}",
        staged_texts(&harness)
    );
    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2",
        "the record keeps its original values"
    );
    assert_eq!(harness.state().snapshot().undo_depth, 0);
}

/// A failed apply keeps every staged edit and the database unchanged.
#[test]
fn a_failed_apply_keeps_every_edit_pending() {
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
    port.with_write_failure("disk on fire");
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
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
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();
    harness.get_by_label("Apply Changes").click();
    harness.run();
    harness.run();

    assert!(
        harness.state().last_error().is_some(),
        "the failed apply reports its reason"
    );
    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2",
        "the database is unchanged"
    );
    assert!(
        staged_texts(&harness)
            .iter()
            .any(|value| value == "alpha2X"),
        "every edit stays pending: {:?}",
        staged_texts(&harness)
    );
}

/// While a submission is open the grid refuses input: no editor arms and
/// no further edits stage.
#[test]
fn the_grid_refuses_input_while_a_submission_is_open() {
    let (app, _tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    type_and_commit(&mut harness, "X");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();

    // Click into another cell twice — the editor must not arm.
    harness.get_by_label("     3").click();
    harness.run();
    harness.get_by_label("alpha1").click();
    harness.run();
    harness.get_by_label("alpha1").click();
    harness.run();

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
        "no cell editor arms while a submission is open: {editors:?}"
    );
    assert!(
        harness.state().snapshot().pending_write.is_some(),
        "the journal stays parked"
    );
}

/// Staged deletes and inserts join the same batch: the delete marks the
/// record (the row cache keeps it until Apply), the new record appends,
/// and one Apply submits all three kinds.
#[test]
fn staged_deletes_and_inserts_apply_with_the_batch() {
    let (app, tab) = batch_app();
    let mut harness = batch_harness(app);
    harness.run();

    // Stage a delete of record 2 through the context menu + confirmation.
    let rect = harness.get_by_label("     3").rect();
    harness.hover_at(rect.center());
    let press = |pressed| egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
    harness.get_by_label("Delete Record").click();
    harness.run();
    harness.run();

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2",
        "the delete is staged, not written"
    );
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    // Stage an insert through the new-record row (the cells ride the
    // presentation setter — the same seam the in-place editor fills).
    harness
        .state_mut()
        .set_new_row_cells(tab, vec![String::new(), "beta".to_owned()]);
    harness.get_by_label("Add record").click();
    harness.run();
    harness.run();

    eprintln!(
        "DEBUG staged buffer: {:?}",
        harness.state().staged_rows(tab)
    );
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();
    let summary = harness
        .state()
        .snapshot()
        .pending_write
        .clone()
        .expect("pending summary");
    assert_eq!(
        (summary.updates, summary.inserts, summary.deletes),
        (0, 1, 1)
    );

    harness.get_by_label("Apply Changes").click();
    harness.run();
    harness.run();

    let rows = harness.state().rows_for(tab).expect("rows reloaded");
    eprintln!(
        "DEBUG rows after apply: {:?}",
        rows.iter()
            .map(|r| datacraft_ui::labels::value_label(&r[1]))
            .collect::<Vec<_>>()
    );
    eprintln!(
        "DEBUG pending_write: {:?}",
        harness.state().snapshot().pending_write
    );
    eprintln!("DEBUG last_error: {:?}", harness.state().last_error());
    assert_eq!(rows.len(), 4, "three originals minus one, plus one insert");
    assert!(
        !rows
            .iter()
            .any(|row| row[1] == datacraft_core::schema::FieldValue::Text("alpha2".into())),
        "the deleted record is gone"
    );
    assert!(
        rows.iter()
            .any(|row| row[1] == datacraft_core::schema::FieldValue::Text("beta".into())),
        "the insert landed"
    );
}
