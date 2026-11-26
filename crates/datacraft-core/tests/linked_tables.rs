//! Link inventory on the command bus (phase 100, step 100.2; item M131;
//! roadmap §9 stage 0).
//!
//! The port's link records reach the snapshot (and therefore the wire), a
//! link-free database reports an empty list, and capability honesty holds:
//! a linked table cannot be opened and cannot be written — the inventory
//! is where roadmap §9 stops.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{LinkKind, LinkedTable, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
    )
}

fn link() -> LinkedTable {
    LinkedTable {
        name: "Orders_Remote".into(),
        kind: LinkKind::Odbc,
        source: None,
        connect: Some("DSN=billing;UID=reader;PWD=***;DATABASE=sales".into()),
        foreign_name: Some("dbo.Orders".into()),
        flags: 537_919_488,
    }
}

fn ready_core(links: Vec<LinkedTable>) -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", vec![vec![FieldValue::Text("Ada".into())]]);
    port.with_linked_tables(links);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

#[test]
fn the_snapshot_carries_the_link_inventory() {
    let core = ready_core(vec![link()]);
    let snapshot = core.inspect();
    let db = snapshot.database.as_ref().expect("a database is open");
    assert_eq!(db.linked_tables.len(), 1, "{:?}", db.linked_tables);
    let entry = &db.linked_tables[0];
    assert_eq!(entry.name, "Orders_Remote");
    assert_eq!(entry.kind, LinkKind::Odbc);
    assert_eq!(entry.source, None);
    assert_eq!(
        entry.connect.as_deref(),
        Some("DSN=billing;UID=reader;PWD=***;DATABASE=sales"),
        "the port's redacted string is passed through unchanged"
    );
    assert_eq!(entry.foreign_name.as_deref(), Some("dbo.Orders"));
    assert_eq!(entry.flags, 537_919_488);
    assert_eq!(
        entry.ui_test_id, "navpane.expanded.linked.odbc.orders_remote",
        "the nav-pane id follows the native-object convention"
    );
    // The wire carries the same shape (kind as a snake_case token).
    let wire = serde_json::to_value(&snapshot).expect("the snapshot serialises");
    assert_eq!(wire["database"]["linked_tables"][0]["kind"], "odbc");
    assert_eq!(
        wire["database"]["linked_tables"][0]["name"],
        "Orders_Remote"
    );
}

#[test]
fn a_file_link_reports_its_source_path() {
    let mut stored = link();
    stored.kind = LinkKind::AccessFile;
    stored.connect = None;
    stored.source = Some("Z:\\data\\linkee.accdb".into());
    let core = ready_core(vec![stored]);
    let db = core.inspect().database.expect("a database is open");
    let entry = &db.linked_tables[0];
    assert_eq!(entry.kind, LinkKind::AccessFile);
    assert_eq!(entry.source.as_deref(), Some("Z:\\data\\linkee.accdb"));
    assert_eq!(entry.connect, None);
    assert_eq!(
        entry.ui_test_id,
        "navpane.expanded.linked.access_file.orders_remote"
    );
}

#[test]
fn the_snapshot_names_a_one_line_target_per_link_kind() {
    let mut file = link();
    file.kind = LinkKind::AccessFile;
    file.connect = None;
    file.source = Some("Z:\\data\\linkee.accdb".into());
    let odbc = link();
    let core = ready_core(vec![file, odbc]);
    let db = core.inspect().database.expect("a database is open");
    assert_eq!(
        db.linked_tables[0].target.as_deref(),
        Some("Z:\\data\\linkee.accdb"),
        "an Access-file link shows its external database"
    );
    assert_eq!(
        db.linked_tables[1].target.as_deref(),
        Some("DSN=billing"),
        "an ODBC link shows its first connect fragment — credential-free"
    );
}

#[test]
fn a_link_free_database_reports_an_empty_inventory() {
    let core = ready_core(Vec::new());
    let db = core.inspect().database.expect("a database is open");
    assert_eq!(db.linked_tables, Vec::new());
    let wire = serde_json::to_value(core.inspect()).expect("serialises");
    assert_eq!(wire["database"]["linked_tables"], serde_json::json!([]));
}

#[test]
fn opening_a_linked_table_is_refused_precisely() {
    let mut core = ready_core(vec![link()]);
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "orders_remote",
    )));
    match events.as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Unsupported);
            assert!(reason.contains("linked table"), "{reason}");
            assert!(reason.contains("orders_remote"), "{reason}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
    let snapshot = core.inspect();
    assert!(snapshot.tabs.is_empty(), "no tab is opened for a link");
    assert_eq!(snapshot.active_tab, None);
}

#[test]
fn a_link_is_never_a_write_target() {
    let mut core = ready_core(vec![link()]);
    for command in [
        Command::InsertRow {
            table: "Orders_Remote".into(),
            values: vec![FieldValue::Text("x".into())],
        },
        Command::UpdateRow {
            table: "Orders_Remote".into(),
            row: 0,
            values: vec![FieldValue::Text("x".into())],
        },
        Command::DeleteRow {
            table: "Orders_Remote".into(),
            row: 0,
        },
    ] {
        let events = core.dispatch(command);
        match events.as_slice() {
            [Event::Failed { code, reason }] => {
                // The link is not in the table catalog, so every
                // table-addressed command answers the lookup refusal
                // before the port is reached — nothing resolves, nothing
                // dials, nothing is written.
                assert_eq!(code, &FailureCode::NotFound, "{events:?}");
                assert!(reason.contains("Orders_Remote"), "{reason}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
    // The real table next to the link stays the only table and keeps its
    // name — the link never enters the table catalog.
    let db = core.state().database.as_ref().expect("a database is open");
    let names: Vec<&str> = db.tables.iter().map(|def| def.name.as_str()).collect();
    assert_eq!(names, ["Customers"]);
}

#[test]
fn the_inventory_does_not_depend_on_the_target_existing() {
    // The path is data: a link whose target is gone still reports it.
    let mut stored = link();
    stored.kind = LinkKind::AccessFile;
    stored.connect = None;
    stored.source = Some("Z:\\gone\\linkee.accdb".into());
    let core = ready_core(vec![stored]);
    let db = core.inspect().database.expect("a database is open");
    assert_eq!(
        db.linked_tables[0].source.as_deref(),
        Some("Z:\\gone\\linkee.accdb")
    );
}
