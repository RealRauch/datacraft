//! Datasheet in-place cell editing suite (step 40.4, item L23).
//!
//! Clicking a cell of the selected record arms an in-cell editor; Enter
//! commits through exactly one `UpdateRow` dispatch over the real write
//! path, Escape cancels, and a mistyped cell keeps the editor with a
//! precise failure. Read-only sources never arm the editor. Assertions
//! stay on the cached rows, the undo depth and the semantic tree.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with the Customers tab open over four records; names carry their
/// index so cells stay uniquely addressable by label.
fn edit_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

fn edit_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

/// Selects the record at zero-based `row` and arms the editor on the
/// second click into its name cell (the selector label is row + 1).
fn arm_editor(harness: &mut Harness<'static, App>, row: usize) {
    harness.get_by_label(&format!("{:>6}", row + 1)).click();
    harness.run();
    harness.get_by_label(&format!("alpha{row}")).click();
    harness.run();
}

fn cached_name(harness: &Harness<'static, App>, tab: TabId, row: usize) -> String {
    harness
        .state()
        .rows_for(tab)
        .and_then(|rows| rows.get(row))
        .and_then(|values| values.get(1))
        .map(datacraft_ui::labels::value_label)
        .unwrap_or_default()
}

/// An edit on the selected record commits through exactly one `UpdateRow`
/// dispatch: the cache and the datasheet show the new text afterwards.
#[test]
fn editing_a_cell_commits_one_update_row() {
    let (app, tab) = edit_app(4);
    let mut harness = edit_harness(app);
    harness.run();

    arm_editor(&mut harness, 2);
    harness.event(egui::Event::Text("X".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2X",
        "the committed edit reaches the row cache"
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "exactly one write command was dispatched"
    );
    assert!(
        harness.state().last_error().is_none(),
        "a valid edit leaves no failure behind"
    );
    // Cell text rides the AccessKit value (the painted label's info);
    // assert the committed text is on the grid.
    let values: Vec<String> = harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().value().unwrap_or_default().to_owned())
        .collect();
    assert!(
        values.iter().any(|value| value == "alpha2X"),
        "the datasheet paints the committed text: {values:?}"
    );
}

/// Escape discards the buffer: no dispatch, the cached value stands.
#[test]
fn escape_cancels_the_cell_edit() {
    let (app, tab) = edit_app(4);
    let mut harness = edit_harness(app);
    harness.run();

    arm_editor(&mut harness, 3);
    harness.event(egui::Event::Text("ZZ".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    assert_eq!(
        cached_name(&harness, tab, 3),
        "alpha3",
        "a cancelled edit never touches the row cache"
    );
    assert_eq!(harness.state().snapshot().undo_depth, 0);
}

/// A mistyped cell keeps the editor open with the buffer intact and
/// reports the precise failure; nothing is dispatched.
#[test]
fn a_mistyped_cell_keeps_the_editor_and_reports_the_failure() {
    let (app, tab) = edit_app(4);
    let mut harness = edit_harness(app);
    harness.run();

    // Select record 3, then arm the editor on its Number cell.
    harness.get_by_label("     3").click();
    harness.run();
    harness.get_by_label("2").click();
    harness.run();
    harness.event(egui::Event::Text("xyz".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();

    let error = harness
        .state()
        .last_error()
        .expect("the failed parse reports a precise error")
        .to_owned();
    assert!(
        error.contains("id"),
        "the failure names the offending field: {error}"
    );
    assert_eq!(
        cached_name(&harness, tab, 3),
        "alpha3",
        "the row cache is untouched by a failed commit"
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "a failed commit dispatches nothing"
    );
    let values: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let n = node.accesskit_node();
            let value = n.value().unwrap_or_default();
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect();
    assert!(
        values.iter().any(|value| value == "2xyz"),
        "the editor stays open with the buffer intact: {values:?}"
    );
}

/// On a read-only source the cells stay inert: clicking one never arms an
/// editor and nothing is dispatched.
#[test]
fn read_only_sources_never_arm_the_cell_editor() {
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(caps);
    port.with_row_data(
        "Customers",
        (0..4)
            .map(|index| {
                vec![
                    datacraft_core::schema::FieldValue::Integer(index as i64),
                    datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
                ]
            })
            .collect(),
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    let mut harness = edit_harness(app);
    harness.run();

    // Select the record via the selector, then click the name cell twice.
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("alpha2").click();
    harness.run();
    harness.get_by_label("alpha2").click();
    harness.run();

    assert_eq!(
        cached_name(&harness, tab, 2),
        "alpha2",
        "a read-only source never edits through the cell"
    );
    assert_eq!(harness.state().snapshot().undo_depth, 0);
    // An armed editor is a TextInput widget; the painted cells are
    // Labels and never appear as one.
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
