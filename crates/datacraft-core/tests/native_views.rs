//! Foreign SQLite views in the core state (phase 73, step 73.1; item M90).
//!
//! The port reports the file's own views; the core projects them into the
//! snapshot, opens them as read-only view tabs and materializes them for
//! the SQL window. The sidecar stays the design authority: a sidecar view
//! with the same name wins over the foreign entry.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, NativeView, RowRange};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn people() -> TableDef {
    TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn adults() -> NativeView {
    NativeView {
        name: "adults".into(),
        sql: "CREATE VIEW adults AS SELECT id, name FROM people".into(),
        fields: vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    }
}

fn opened() -> Core {
    let mut port = MockPort::with_tables(vec![people()]);
    port.with_native_views(vec![adults()]);
    port.with_row_data(
        "people",
        vec![vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())]],
    );
    port.with_row_data(
        "adults",
        vec![vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())]],
    );
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.sqlite"),
        password: None,
        source: None,
    });
    core
}

#[test]
fn the_snapshot_lists_foreign_views() {
    let core = opened();
    let snapshot = core.inspect();
    let database = snapshot.database.expect("open");
    assert_eq!(database.native_views.len(), 1);
    assert_eq!(database.native_views[0].name, "adults");
    assert!(database.native_views[0].sql.contains("CREATE VIEW"));
    assert!(database.native_views[0].ui_test_id.contains("view.adults"));
    // The view does not appear as a table.
    assert_eq!(database.tables.len(), 1);
}

#[test]
fn a_foreign_view_opens_a_tab_and_reads_rows() {
    let mut core = opened();
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::View,
        "adults",
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs[0].object_kind, "view");
    assert_eq!(snapshot.tabs[0].object_name, "adults");

    let result = core.native_view_result("adults").expect("view rows");
    assert_eq!(result.columns, ["id", "name"]);
    assert_eq!(result.rows[0][1], FieldValue::Text("Ada".into()));
    let rows = core
        .read_rows("adults", RowRange { start: 0, len: 10 })
        .expect("view read through the port");
    assert_eq!(rows.len(), 1);
}

#[test]
fn the_sql_window_sees_foreign_views_as_sources() {
    let core = opened();
    let result = core.run_sql("SELECT name FROM adults").expect("sql run");
    assert_eq!(result.columns, ["name"]);
    assert_eq!(result.rows[0][0], FieldValue::Text("Ada".into()));
}

#[test]
fn the_sidecar_view_wins_over_the_foreign_entry() {
    let mut core = opened();
    let path = std::env::temp_dir().join(format!(
        "datacraft-native-view-{}.datacraft",
        std::process::id()
    ));
    let mut document = datacraft_core::schema::SchemaDocument::new("shop");
    document.views.push(datacraft_core::view::ViewDef::new(
        "adults",
        "SELECT id, name FROM people",
    ));
    document.save(&path).expect("write sidecar");
    let events = core.dispatch(Command::LoadDesignDocument { path: path.clone() });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let snapshot = core.inspect();
    let database = snapshot.database.expect("open");
    assert!(
        database.native_views.is_empty(),
        "the sidecar design authority filters the foreign entry: {:?}",
        database.native_views
    );
    assert_eq!(database.views.len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn drivers_without_a_view_inventory_report_none() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![people()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    assert!(
        core.inspect()
            .database
            .expect("open")
            .native_views
            .is_empty()
    );
}
