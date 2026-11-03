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
use datacraft_core::port::{DatabaseFormat, Relationship, StoragePort};
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

#[test]
fn port_level_errors_and_read_only_sources_are_precise() {
    let path = scratch_path("port-errors");
    let _scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&TableDef::new(
        "parents",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("label", FieldKind::Text { max_len: 20 }),
        ],
    ))
    .expect("parents");
    port.create_table(&TableDef::new(
        "children",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new(
                "parent",
                FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
            ),
        ],
    ))
    .expect("children");
    let relationship = relationship();
    let error = port
        .delete_relationship(&relationship)
        .expect_err("no foreign key yet");
    assert!(error.to_string().contains("not found"), "{error:?}");
    port.create_relationship(&relationship).expect("create");
    let error = port
        .create_relationship(&relationship)
        .expect_err("duplicate");
    assert!(error.to_string().contains("duplicate"), "{error:?}");
    let missing = Relationship {
        table: "children".into(),
        column: "ghost".into(),
        related_table: "parents".into(),
        related_column: "id".into(),
    };
    let error = port
        .create_relationship(&missing)
        .expect_err("missing child field");
    assert!(error.to_string().contains("ghost"), "{error:?}");
    let absent = Relationship {
        table: "children".into(),
        column: "parent".into(),
        related_table: "parents".into(),
        related_column: "ghost".into(),
    };
    let error = port
        .delete_relationship(&absent)
        .expect_err("four names must match");
    assert!(error.to_string().contains("not found"), "{error:?}");
    port.close();

    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).expect("read-only");
    let mut read_only = SqlitePort::new();
    read_only.open(&path, None).expect("open read-only");
    let error = read_only
        .create_relationship(&relationship)
        .expect_err("read-only source");
    assert!(error.to_string().contains("read-only"), "{error:?}");
    let error = read_only
        .delete_relationship(&relationship)
        .expect_err("read-only source");
    assert!(error.to_string().contains("read-only"), "{error:?}");
    read_only.close();
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).expect("writable again");
}

#[test]
fn indexed_columns_survive_the_rebuild() {
    let path = scratch_path("indexed");
    let _scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&TableDef::new(
        "parents",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("label", FieldKind::Text { max_len: 20 }),
        ],
    ))
    .expect("parents");
    port.create_table(&TableDef::new(
        "children",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef {
                name: "parent".into(),
                kind: FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
                properties: datacraft_core::schema::FieldProperties {
                    indexed: datacraft_core::schema::IndexMode::DuplicatesOk,
                    ..Default::default()
                },
            },
            FieldDef {
                name: "tag".into(),
                kind: FieldKind::Text { max_len: 20 },
                properties: datacraft_core::schema::FieldProperties {
                    indexed: datacraft_core::schema::IndexMode::NoDuplicates,
                    ..Default::default()
                },
            },
        ],
    ))
    .expect("children");
    port.create_relationship(&relationship()).expect("create");
    port.close();

    let mut reopened = SqlitePort::new();
    reopened.open(&path, None).expect("reopen");
    reopened
        .insert_row("parents", &[FieldValue::Null, FieldValue::Text("a".into())])
        .expect("parent");
    reopened
        .insert_row(
            "children",
            &[
                FieldValue::Null,
                FieldValue::Integer(1),
                FieldValue::Text("tag".into()),
            ],
        )
        .expect("child");
    let error = reopened
        .insert_row(
            "children",
            &[
                FieldValue::Null,
                FieldValue::Integer(1),
                FieldValue::Text("tag".into()),
            ],
        )
        .expect_err("the unique tag index survives the rebuild");
    assert!(
        error.to_string().to_uppercase().contains("UNIQUE"),
        "{error:?}"
    );
}

#[test]
fn a_rebuild_failure_rolls_back() {
    let path = scratch_path("rollback");
    let _scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&TableDef::new(
        "parents",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("code", FieldKind::Text { max_len: 20 }),
        ],
    ))
    .expect("parents");
    port.create_table(&TableDef::new(
        "children",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("parent_code", FieldKind::Text { max_len: 20 }),
        ],
    ))
    .expect("children");
    for _ in 0..2 {
        port.insert_row(
            "parents",
            &[FieldValue::Null, FieldValue::Text("same".into())],
        )
        .expect("duplicate parent code");
    }
    port.insert_row(
        "children",
        &[FieldValue::Null, FieldValue::Text("same".into())],
    )
    .expect("child");
    let relationship = Relationship {
        table: "children".into(),
        column: "parent_code".into(),
        related_table: "parents".into(),
        related_column: "code".into(),
    };
    let error = port
        .create_relationship(&relationship)
        .expect_err("duplicate parent codes block the unique index");
    assert!(error.to_string().contains("rebuilding table"), "{error:?}");
    let rows = port
        .read_rows(
            "children",
            datacraft_core::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("child rows survive the rollback");
    assert_eq!(rows.len(), 1);
}
