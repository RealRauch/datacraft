//! Read-only index/trigger inventory in the core state (phase 73, step
//! 73.2; item M90).
//!
//! The port reports the open file's indexes and triggers; the snapshot
//! carries them for every client. Nothing here is writable: DataCraft
//! never creates or executes a trigger (inventory only).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::{MockPort, SourceIndex, SourceTrigger};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn people() -> TableDef {
    TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("city", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn opened() -> Core {
    let mut port = MockPort::with_tables(vec![people()]);
    port.with_indexes(vec![SourceIndex {
        table: "people".into(),
        name: "idx_people_city".into(),
        columns: vec!["city".into()],
        unique: false,
        automatic: false,
    }]);
    port.with_triggers(vec![SourceTrigger {
        table: "people".into(),
        name: "people_guard".into(),
        sql: "CREATE TRIGGER people_guard AFTER INSERT ON people BEGIN SELECT 1; END".into(),
    }]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.sqlite"),
        password: None,
        source: None,
    });
    core
}

#[test]
fn the_snapshot_lists_indexes_and_triggers() {
    let core = opened();
    let database = core.inspect().database.expect("open");
    assert_eq!(database.indexes.len(), 1);
    assert_eq!(database.indexes[0].name, "idx_people_city");
    assert_eq!(database.indexes[0].columns, ["city"]);
    assert!(!database.indexes[0].unique);
    assert_eq!(database.triggers.len(), 1);
    assert_eq!(database.triggers[0].name, "people_guard");
    assert_eq!(database.triggers[0].table, "people");
    assert!(database.triggers[0].sql.contains("CREATE TRIGGER"));
}

#[test]
fn drivers_without_the_inventory_report_empty_lists() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![people()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let database = core.inspect().database.expect("open");
    assert!(database.indexes.is_empty());
    assert!(database.triggers.is_empty());
}
