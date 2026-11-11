//! View design contracts (step 71.1, item M93).
//!
//! A view is a named `SELECT` stored as a sidecar design object; the
//! commands validate names and the statement shape, the snapshot carries
//! the definitions, and opening a view produces an ordinary tab.

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn open_core() -> Core {
    let table = TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![table])));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

fn views(core: &Core) -> Vec<datacraft_core::view::ViewDef> {
    core.inspect().database.expect("open").views
}

#[test]
fn views_are_created_changed_and_deleted() {
    let mut core = open_core();
    let events = core.dispatch(Command::CreateView {
        name: "Active".into(),
        sql: "SELECT 1 AS one".into(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::ViewCreated { .. })),
        "{events:?}"
    );
    let current = views(&core);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].name, "Active");
    assert_eq!(current[0].sql, "SELECT 1 AS one");

    let events = core.dispatch(Command::SetViewSql {
        name: "active".into(),
        sql: "SELECT 2 AS two".into(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::ViewChanged { .. })),
        "{events:?}"
    );
    assert_eq!(views(&core)[0].sql, "SELECT 2 AS two");

    let events = core.dispatch(Command::DeleteView {
        name: "Active".into(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::ViewDeleted { .. })),
        "{events:?}"
    );
    assert!(views(&core).is_empty());
}

#[test]
fn view_names_and_statements_are_validated() {
    let mut core = open_core();

    let events = core.dispatch(Command::CreateView {
        name: "  ".into(),
        sql: "SELECT 1".into(),
    });
    assert!(
        events.iter().any(|event| matches!(event, Event::Failed { .. })),
        "an empty name fails"
    );

    let events = core.dispatch(Command::CreateView {
        name: "Customers".into(),
        sql: "SELECT 1".into(),
    });
    assert!(
        events.iter().any(|event| matches!(event, Event::Failed { .. })),
        "a table-name collision fails"
    );

    core.dispatch(Command::CreateView {
        name: "Active".into(),
        sql: "SELECT 1".into(),
    });
    let events = core.dispatch(Command::CreateView {
        name: "active".into(),
        sql: "SELECT 2".into(),
    });
    assert!(
        events.iter().any(|event| matches!(event, Event::Failed { .. })),
        "a case-insensitive duplicate fails"
    );

    let events = core.dispatch(Command::CreateView {
        name: "Delete".into(),
        sql: "DELETE FROM Customers".into(),
    });
    assert!(
        events.iter().any(|event| matches!(event, Event::Failed { .. })),
        "a non-SELECT statement fails"
    );

    let events = core.dispatch(Command::SetViewSql {
        name: "missing".into(),
        sql: "SELECT 1".into(),
    });
    assert!(
        events.iter().any(|event| matches!(event, Event::Failed { .. })),
        "unknown views fail"
    );
}

#[test]
fn the_snapshot_carries_views_and_opens_a_tab() {
    let mut core = open_core();
    core.dispatch(Command::CreateView {
        name: "Active".into(),
        sql: "SELECT 1 AS one".into(),
    });

    let db = core.inspect().database.expect("open");
    assert_eq!(db.views.len(), 1);
    assert_eq!(db.views[0].name, "Active");
    assert_eq!(db.views[0].sql, "SELECT 1 AS one");

    let events = core.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::View, "Active")));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::TabOpened { .. })),
        "{events:?}"
    );
    let snapshot = core.inspect();
    let tab = snapshot.tabs.first().expect("a tab");
    assert_eq!(tab.object_kind, "view");
    assert_eq!(tab.object_name, "Active");
}

#[test]
fn views_round_trip_through_the_sidecar() {
    let mut core = open_core();
    core.dispatch(Command::CreateView {
        name: "Active".into(),
        sql: "SELECT 1 AS one".into(),
    });
    let path = std::env::temp_dir().join(format!("datacraft-views-{}.datacraft", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let events = core.dispatch(Command::SaveDesignDocument {
        path: path.clone().into(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DesignDocumentSaved { .. })),
        "{events:?}"
    );

    let mut reopened = open_core();
    let events = reopened.dispatch(Command::LoadDesignDocument {
        path: path.clone().into(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DesignDocumentLoaded { .. })),
        "{events:?}"
    );
    assert_eq!(views(&reopened)[0].name, "Active");
    assert_eq!(views(&reopened)[0].sql, "SELECT 1 AS one");
    let _ = std::fs::remove_file(&path);
}
