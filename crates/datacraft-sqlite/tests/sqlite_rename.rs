//! Table renames on the SQLite backend (phase 72, step 72.3; item M81).
//!
//! SQLite renames natively (`ALTER TABLE … RENAME TO …`); SQLite rewrites
//! references in triggers, views and foreign-key clauses itself. Case-only
//! renames are refused precisely — SQLite compares names case-insensitively
//! and reports a collision with the table itself.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, Relationship, RowRange, StoragePort};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-rename-{}-{n}-{stem}-{stamp}.sqlite",
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
        source: None,
    });
    core
}

fn created(stem: &str) -> (Core, Scratch) {
    let path = scratch_path(stem);
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    let mut core = open(&path);
    for definition in [
        TableDef::new(
            "parents",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new("label", FieldKind::Text { max_len: 20 }),
            ],
        ),
        TableDef::new(
            "children",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new("parent", FieldKind::Number(NumericKind::LongInteger)),
            ],
        ),
    ] {
        let events = core.dispatch(Command::CreateTable { definition });
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. })),
            "{events:?}"
        );
    }
    for (id, label) in [(1, "Ada"), (2, "Grace")] {
        core.dispatch(Command::InsertRow {
            table: "parents".into(),
            values: vec![FieldValue::Integer(id), FieldValue::Text(label.into())],
        });
    }
    core.dispatch(Command::InsertRow {
        table: "children".into(),
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

fn failure(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("a Failed event")
}

#[test]
fn renaming_a_table_keeps_rows_and_survives_reopen() {
    let (mut core, scratch) = created("basic");
    let events = rename_table(&mut core, "PARENTS", "guardians");
    assert_eq!(
        events,
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Table,
            old_name: "parents".into(),
            new_name: "guardians".into(),
        }]
    );
    assert_eq!(
        rows(&core, "guardians")[0],
        vec![FieldValue::Integer(1), FieldValue::Text("Ada".into()),]
    );
    assert!(
        core.read_rows("parents", RowRange { start: 0, len: 10 })
            .is_err()
    );
    core.dispatch(Command::CloseDatabase);

    let reopened = open(&scratch.0);
    let database = reopened.inspect().database.expect("reopen");
    assert!(
        database
            .tables
            .iter()
            .any(|table| table.name == "guardians"),
        "{:?}",
        database.tables
    );
    assert_eq!(rows(&reopened, "guardians").len(), 2);
}

#[test]
fn renaming_a_table_follows_foreign_keys_on_disk() {
    let (mut core, scratch) = created("fk");
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(
        !events.iter().any(|e| matches!(e, Event::Failed { .. })),
        "{events:?}"
    );

    let events = rename_table(&mut core, "parents", "guardians");
    assert!(
        !events.iter().any(|e| matches!(e, Event::Failed { .. })),
        "{events:?}"
    );
    let snapshot = core.inspect().database.expect("open");
    assert_eq!(snapshot.relationships.len(), 1);
    assert_eq!(snapshot.relationships[0].related_table, "guardians");
    core.dispatch(Command::CloseDatabase);

    let reopened = open(&scratch.0);
    let snapshot = reopened.inspect().database.expect("reopen");
    assert_eq!(
        snapshot.relationships.len(),
        1,
        "{:?}",
        snapshot.relationships
    );
    assert_eq!(snapshot.relationships[0].related_table, "guardians");
    assert_eq!(snapshot.relationships[0].table, "children");
}

#[test]
fn case_only_renames_are_refused_precisely() {
    let (mut core, _scratch) = created("case");
    let reason = failure(&rename_table(&mut core, "parents", "PARENTS"));
    assert!(reason.contains("case"), "{reason}");
    let database = core.inspect().database.expect("open");
    assert!(database.tables.iter().any(|table| table.name == "parents"));
}

#[test]
fn collisions_and_unknown_tables_are_precise() {
    let (mut core, _scratch) = created("clash");
    let reason = failure(&rename_table(&mut core, "parents", "children"));
    assert!(reason.contains("duplicate"), "{reason}");
    let reason = failure(&rename_table(&mut core, "ghost", "x"));
    assert!(reason.contains("not found"), "{reason}");
}

#[test]
fn the_rename_capability_follows_the_session() {
    let (core, scratch) = created("caps");
    drop(core);
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    assert!(port.capabilities().rename_table);
    assert!(
        !port.capabilities().alter_table,
        "field-level alterations stay a separate stage"
    );
}
