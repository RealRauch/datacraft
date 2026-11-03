//! Jet relationship creation (step 32.3, item M16).
//!
//! `CreateRelationship` lazily creates `MSysRelationships`, adds the parent
//! unique and child plain indexes through the M15 TDEF metadata, writes the
//! row in one writer session and survives reopen. `DeleteRelationship`
//! removes the row; duplicate names are refused.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, Relationship};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, IndexMode, NumericKind, TableDef};
use datacraft_storage::port_adapter::Jet4Port;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-jet-rel-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
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
    });
    core
}

fn create_tables(core: &mut Core) {
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "Customers",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("name", FieldKind::Text { max_len: 20 }),
            ],
        ),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "Orders",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
            ],
        ),
    });
}

fn insert(core: &mut Core, table: &str, values: Vec<FieldValue>) {
    let events = core.dispatch(Command::InsertRow {
        table: table.to_owned(),
        values,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
}

fn setup() -> (Core, Scratch) {
    let path = scratch_path("create");
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    core = open(&path);
    create_tables(&mut core);
    for (id, name) in [(1, "Ada"), (2, "Grace")] {
        insert(
            &mut core,
            "Customers",
            vec![FieldValue::Integer(id), FieldValue::Text(name.into())],
        );
    }
    for (id, customer) in [(10, 1), (11, 2), (12, 9)] {
        insert(
            &mut core,
            "Orders",
            vec![FieldValue::Integer(id), FieldValue::Integer(customer)],
        );
    }
    (core, scratch)
}

#[test]
fn created_relationships_and_indexes_survive_reopen() {
    let (mut core, scratch) = setup();
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(
        matches!(events.first(), Some(Event::RelationshipCreated { .. })),
        "{events:?}"
    );
    let database = core.inspect().database.expect("open");
    assert_eq!(database.relationships, vec![relationship()]);
    assert_eq!(
        database.tables[0].fields[0].properties.indexed,
        IndexMode::NoDuplicates,
        "the parent column gains a unique index"
    );
    assert_eq!(
        database.tables[1].fields[1].properties.indexed,
        IndexMode::DuplicatesOk,
        "the child column gains a plain index"
    );
    core.dispatch(Command::CloseDatabase);

    let mut reopened = open(&scratch.0);
    let database = reopened.inspect().database.expect("open");
    assert_eq!(database.relationships, vec![relationship()]);
    assert_eq!(
        database.tables[0].fields[0].properties.indexed,
        IndexMode::NoDuplicates
    );
    let duplicate = reopened.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(matches!(duplicate.first(), Some(Event::Failed { .. })));

    let events = reopened.dispatch(Command::DeleteRelationship {
        relationship: relationship(),
    });
    assert!(
        matches!(events.first(), Some(Event::RelationshipDeleted { .. })),
        "{events:?}"
    );
    assert!(
        reopened
            .inspect()
            .database
            .expect("open")
            .relationships
            .is_empty()
    );
    reopened.dispatch(Command::CloseDatabase);
    let again = open(&scratch.0);
    assert!(
        again
            .inspect()
            .database
            .expect("open")
            .relationships
            .is_empty(),
        "the deleted relationship stays gone"
    );
}

#[test]
fn a_unique_parent_index_over_duplicates_is_refused() {
    let path = scratch_path("duplicates");
    let _scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    core = open(&path);
    create_tables(&mut core);
    insert(
        &mut core,
        "Customers",
        vec![FieldValue::Integer(1), FieldValue::Text("a".into())],
    );
    insert(
        &mut core,
        "Customers",
        vec![FieldValue::Integer(1), FieldValue::Text("b".into())],
    );
    insert(
        &mut core,
        "Orders",
        vec![FieldValue::Integer(10), FieldValue::Integer(1)],
    );
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        })
        .expect("duplicate parent values block the unique index");
    assert!(reason.contains("duplicate value"), "{reason}");
    assert!(
        core.inspect()
            .database
            .expect("open")
            .relationships
            .is_empty()
    );
}
