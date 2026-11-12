//! Table renames on the Jet4/ACE writer (phase 72, step 72.2; item M81).
//!
//! The catalog row owns the table name (the TDEF carries none), so a
//! rename rewrites that cell and every `MSysRelationships` object column.
//! Rows, indexes and relationship edges survive; the file reopens
//! consistently; collisions leave the catalog untouched.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, Relationship, RowRange, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_jet::port_adapter::Jet4Port;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-jet-rename-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
        ],
    )
}

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn relationship() -> Relationship {
    Relationship {
        table: "Orders".into(),
        column: "customer_id".into(),
        related_table: "Customers".into(),
        related_column: "id".into(),
    }
}

fn open(path: &std::path::Path) -> Core {
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::OpenDatabase {
        path: path.to_path_buf(),
        password: None,
        source: None,
    });
    core
}

fn created(format: DatabaseFormat, stem: &str) -> (Core, Scratch) {
    let path = scratch_path(stem);
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format,
    });
    let mut core = open(&path);
    for definition in [customers(), orders()] {
        let events = core.dispatch(Command::CreateTable { definition });
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. })),
            "{events:?}"
        );
    }
    for (id, name) in [(1, "Ada"), (2, "Grace")] {
        core.dispatch(Command::InsertRow {
            table: "Customers".into(),
            values: vec![FieldValue::Integer(id), FieldValue::Text(name.into())],
        });
    }
    core.dispatch(Command::InsertRow {
        table: "Orders".into(),
        values: vec![FieldValue::Integer(10), FieldValue::Integer(1)],
    });
    (core, scratch)
}

fn rename_table(core: &mut Core, name: &str, new_name: &str) -> Vec<Event> {
    core.dispatch(Command::RenameObject {
        object: ObjectId::new(ObjectKind::Table, name),
        new_name: new_name.to_owned(),
    })
}

fn rows(core: &Core, table: &str) -> Vec<Vec<FieldValue>> {
    core.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("rows read")
}

fn failures(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::Failed { reason, .. } => Some(reason.clone()),
        _ => None,
    })
}

#[test]
fn renaming_a_table_keeps_rows_and_survives_reopen() {
    let (mut core, scratch) = created(DatabaseFormat::Jet4, "jet4");
    let events = rename_table(&mut core, "customers", "Clients");
    assert_eq!(
        events,
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Table,
            old_name: "Customers".into(),
            new_name: "Clients".into(),
        }]
    );
    assert_eq!(
        rows(&core, "Clients")[0],
        vec![FieldValue::Integer(1), FieldValue::Text("Ada".into()),]
    );
    assert!(
        core.read_rows("Customers", RowRange { start: 0, len: 10 })
            .is_err()
    );
    core.dispatch(Command::CloseDatabase);

    let reopened = open(&scratch.0);
    let database = reopened.inspect().database.expect("reopen");
    assert!(
        database.tables.iter().any(|table| table.name == "Clients"),
        "{:?}",
        database.tables
    );
    assert!(
        !database
            .tables
            .iter()
            .any(|table| table.name == "Customers")
    );
    assert_eq!(
        rows(&reopened, "Clients")[1],
        vec![FieldValue::Integer(2), FieldValue::Text("Grace".into()),]
    );
}

#[test]
fn renaming_a_table_follows_relationships_on_disk() {
    let (mut core, scratch) = created(DatabaseFormat::Jet4, "rel");
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(failures(&events).is_none(), "{events:?}");

    let events = rename_table(&mut core, "Customers", "Clients");
    assert!(failures(&events).is_none(), "{events:?}");
    let snapshot = core.inspect().database.expect("open");
    assert_eq!(snapshot.relationships.len(), 1);
    assert_eq!(snapshot.relationships[0].related_table, "Clients");
    core.dispatch(Command::CloseDatabase);

    let reopened = open(&scratch.0);
    let snapshot = reopened.inspect().database.expect("reopen");
    assert_eq!(
        snapshot.relationships.len(),
        1,
        "{:?}",
        snapshot.relationships
    );
    assert_eq!(snapshot.relationships[0].related_table, "Clients");
    assert_eq!(snapshot.relationships[0].table, "Orders");
    assert_eq!(rows(&reopened, "Orders").len(), 1);
}

#[test]
fn collisions_and_unknown_tables_leave_the_catalog_untouched() {
    let (mut core, _scratch) = created(DatabaseFormat::Jet4, "clash");
    let reason = failures(&rename_table(&mut core, "Customers", "orders")).expect("duplicate");
    assert!(reason.contains("duplicate"), "{reason}");
    let reason = failures(&rename_table(&mut core, "Ghost", "Clients")).expect("unknown");
    assert!(reason.contains("not found"), "{reason}");
    let database = core.inspect().database.expect("open");
    assert!(
        database
            .tables
            .iter()
            .any(|table| table.name == "Customers")
    );
    assert!(database.tables.iter().any(|table| table.name == "Orders"));
}

#[test]
fn case_only_renames_survive_reopen() {
    let (mut core, scratch) = created(DatabaseFormat::Jet4, "case");
    let events = rename_table(&mut core, "Customers", "CUSTOMERS");
    assert_eq!(
        events,
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Table,
            old_name: "Customers".into(),
            new_name: "CUSTOMERS".into(),
        }]
    );
    core.dispatch(Command::CloseDatabase);
    let reopened = open(&scratch.0);
    let database = reopened.inspect().database.expect("reopen");
    assert!(
        database
            .tables
            .iter()
            .any(|table| table.name == "CUSTOMERS"),
        "{:?}",
        database.tables
    );
    assert_eq!(rows(&reopened, "CUSTOMERS").len(), 2);
}

#[test]
fn renaming_a_table_in_an_ace_database_persists() {
    let (mut core, scratch) = created(DatabaseFormat::Ace, "ace");
    let events = rename_table(&mut core, "Customers", "Clients");
    assert!(failures(&events).is_none(), "{events:?}");
    core.dispatch(Command::CloseDatabase);

    let reopened = open(&scratch.0);
    let database = reopened.inspect().database.expect("reopen");
    assert_eq!(database.format, DatabaseFormat::Ace);
    assert!(database.tables.iter().any(|table| table.name == "Clients"));
    assert_eq!(rows(&reopened, "Clients").len(), 2);
}
