//! SQLite foreign-key creation (step 32.3, item M16).
//!
//! SQLite cannot add a foreign key in place, so the port rebuilds the
//! referencing table in one transaction. The FK lands in the DDL, SQLite
//! enforces it on later inserts, existing orphans block the creation, and
//! reopening/removing the relationship keeps the file consistent.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, Relationship};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_storage::sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-rel-{}-{n}-{stem}.sqlite",
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
        table: "children".into(),
        column: "parent".into(),
        related_table: "parents".into(),
        related_column: "id".into(),
    }
}

fn open(path: &std::path::Path) -> Core {
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::OpenDatabase {
        path: path.to_path_buf(),
        password: None,
    });
    core
}

fn create_tables(core: &mut Core) {
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "parents",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new("label", FieldKind::Text { max_len: 20 }),
            ],
        ),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "children",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new(
                    "parent",
                    FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
                ),
            ],
        ),
    });
}

fn insert(core: &mut Core, table: &str, values: Vec<FieldValue>) -> Vec<Event> {
    core.dispatch(Command::InsertRow {
        table: table.to_owned(),
        values,
    })
}

fn failure(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected Failed: {events:?}"))
}

fn setup() -> (Core, Scratch) {
    let path = scratch_path("create");
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    core = open(&path);
    create_tables(&mut core);
    for label in ["a", "b"] {
        let events = insert(
            &mut core,
            "parents",
            vec![FieldValue::Null, FieldValue::Text(label.into())],
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. })),
            "{events:?}"
        );
    }
    for parent in [1, 2] {
        let events = insert(
            &mut core,
            "children",
            vec![FieldValue::Null, FieldValue::Integer(parent)],
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. })),
            "{events:?}"
        );
    }
    (core, scratch)
}

#[test]
fn foreign_keys_are_enforced_and_survive_reopen() {
    let (mut core, scratch) = setup();
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(
        matches!(events.first(), Some(Event::RelationshipCreated { .. })),
        "{events:?}"
    );
    assert_eq!(
        core.inspect().database.expect("open").relationships,
        vec![relationship()]
    );
    let events = insert(
        &mut core,
        "children",
        vec![FieldValue::Null, FieldValue::Integer(1)],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "a valid child inserts: {events:?}"
    );
    let reason = failure(&insert(
        &mut core,
        "children",
        vec![FieldValue::Null, FieldValue::Integer(99)],
    ));
    assert!(reason.to_uppercase().contains("FOREIGN KEY"), "{reason}");
    core.dispatch(Command::CloseDatabase);

    let mut reopened = open(&scratch.0);
    assert_eq!(
        reopened.inspect().database.expect("open").relationships,
        vec![relationship()]
    );
    let reason = failure(&insert(
        &mut reopened,
        "children",
        vec![FieldValue::Null, FieldValue::Integer(99)],
    ));
    assert!(reason.to_uppercase().contains("FOREIGN KEY"), "{reason}");
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
    let events = insert(
        &mut reopened,
        "children",
        vec![FieldValue::Null, FieldValue::Integer(99)],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "without the FK the orphan is allowed again: {events:?}"
    );
    reopened.dispatch(Command::CloseDatabase);
    let again = open(&scratch.0);
    assert!(
        again
            .inspect()
            .database
            .expect("open")
            .relationships
            .is_empty()
    );
}

#[test]
fn existing_orphans_block_the_relationship() {
    let path = scratch_path("orphan");
    let _scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    core = open(&path);
    create_tables(&mut core);
    let events = insert(
        &mut core,
        "parents",
        vec![FieldValue::Null, FieldValue::Text("a".into())],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    );
    let events = insert(
        &mut core,
        "children",
        vec![FieldValue::Null, FieldValue::Integer(99)],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    );
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    }));
    assert!(
        reason.contains("existing") && reason.contains("missing"),
        "{reason}"
    );
    assert!(
        core.inspect()
            .database
            .expect("open")
            .relationships
            .is_empty()
    );
}
