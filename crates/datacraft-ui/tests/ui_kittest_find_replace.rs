//! Find and replace over the visible rows (phase 101, step 101.3; item
//! M132).
//!
//! The window stages every replacement into the tab's edit buffer: the
//! grid paints the pending image, nothing is written until the batch
//! applies, and a filtered-out row never takes part.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::find_replace::FindReplaceDraft;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn sample_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Text("Ada Lovelace".into()),
            FieldValue::Integer(1),
        ],
        vec![
            FieldValue::Text("Grace Hopper".into()),
            FieldValue::Integer(2),
        ],
        vec![FieldValue::Null, FieldValue::Integer(3)],
    ]
}

fn ready_app(rows: Vec<Vec<FieldValue>>) -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", rows);
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
    app
}

fn datasheet_harness(app: App) -> Harness<'static, App> {
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

fn draft(find: &str, replace: &str) -> FindReplaceDraft {
    FindReplaceDraft {
        find: find.into(),
        replace: replace.into(),
        ..FindReplaceDraft::default()
    }
}

#[test]
fn replace_all_stages_every_match_once() {
    let app = ready_app(sample_rows());
    let mut harness = datasheet_harness(app);
    harness.run();
    // The window opens from the datasheet's affordance.
    harness.get_by_label("Find and Replace…").click();
    harness.run();
    assert!(harness.query_by_label("Replace with").is_some());
    let tab_id = harness.state().snapshot().tabs[0].id;
    harness.state_mut().find_replace = Some(draft("ada", "Grace"));
    harness.run();
    harness.get_by_label("Replace All").click();
    harness.run();
    // One cell staged; the grid paints the pending image.
    let buffer = harness
        .state()
        .staged_rows(tab_id)
        .expect("the batch carries the replacement");
    assert_eq!(buffer.row_count(), 3);
    let image = buffer.row_image(0).expect("the staged row");
    assert_eq!(image[0], FieldValue::Text("Grace Lovelace".into()));
    assert!(
        harness.query_by_label_contains("Grace Lovelace").is_some(),
        "the datasheet renders the staged image"
    );
    // Nothing was written: the database has no pending journal.
    assert!(harness.state().snapshot().pending_write.is_none());
}

#[test]
fn a_numeric_column_is_never_a_replace_target() {
    let mut app = ready_app(sample_rows());
    let mut window = draft("1", "9");
    window.column = Some("qty".into());
    app.find_replace = Some(window);
    let mut harness = datasheet_harness(app);
    harness.run();
    harness.get_by_label("Replace All").click();
    harness.run();
    let draft = harness
        .state()
        .find_replace
        .clone()
        .expect("the window stays");
    assert!(
        draft
            .error
            .as_deref()
            .is_some_and(|reason| reason.contains("text")),
        "{draft:?}"
    );
    assert_eq!(draft.staged, None);
}

#[test]
fn a_filtered_out_row_never_takes_part() {
    let mut app = ready_app(sample_rows());
    app.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "name".into(),
        value: Some("Grace Hopper".into()),
    });
    app.find_replace = Some(draft("Grace", "Ada"));
    let mut harness = datasheet_harness(app);
    harness.run();
    harness.get_by_label("Replace All").click();
    harness.run();
    let draft = harness
        .state()
        .find_replace
        .clone()
        .expect("the window stays");
    assert_eq!(draft.staged, Some(1), "{draft:?}");
    let tab_id = harness.state().snapshot().tabs[0].id;
    let buffer = harness.state().staged_rows(tab_id).expect("a batch");
    assert_eq!(
        buffer.row_image(1).expect("row 1")[0],
        FieldValue::Text("Ada Hopper".into())
    );
    // The filtered-out Ada row is untouched — its image is the cache row.
    assert_eq!(
        buffer.row_image(0).expect("row 0")[0],
        FieldValue::Text("Ada Lovelace".into())
    );
}
