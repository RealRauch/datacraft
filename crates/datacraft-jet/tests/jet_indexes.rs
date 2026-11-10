//! Jet index metadata and uniqueness (step 32.2, item M15).
//!
//! The writer emits TDEF index descriptors and logical blocks, keeps
//! existing index sections untouched on unrelated row writes, restores the
//! domain `IndexMode` on reopen, and enforces `NoDuplicates` with a precise
//! error on insert/update/batches. Physical index B-tree pages stay the
//! documented roadmap item.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, FieldValue, IndexMode, NumericKind, TableDef,
};
use datacraft_jet::jet::{Jet4TableReader, JetTableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-indexes-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn indexed_table() -> TableDef {
    TableDef::new(
        "People",
        vec![
            FieldDef {
                name: "id".into(),
                kind: FieldKind::Number(NumericKind::LongInteger),
                properties: FieldProperties {
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
            FieldDef {
                name: "name".into(),
                kind: FieldKind::Text { max_len: 20 },
                properties: FieldProperties {
                    indexed: IndexMode::DuplicatesOk,
                    ..FieldProperties::default()
                },
            },
        ],
    )
}

fn opened() -> (Jet4Port, Scratch) {
    let path = scratch_path("create");
    let scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create Jet4");
    port.open(&path, None).expect("open");
    port.create_table(&indexed_table()).expect("create table");
    (port, scratch)
}

fn unique_index_error(error: &PortError) -> bool {
    error.to_string().contains("unique index")
}

#[test]
fn created_indexes_survive_reopen_and_enforce_uniqueness() {
    let (mut port, _scratch) = opened();
    let fields = port.tables()[0].fields.clone();
    assert_eq!(fields[0].properties.indexed, IndexMode::NoDuplicates);
    assert_eq!(fields[1].properties.indexed, IndexMode::DuplicatesOk);

    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("alpha".into())],
    )
    .expect("first row");
    let error = port
        .insert_row(
            "People",
            &[FieldValue::Integer(1), FieldValue::Text("beta".into())],
        )
        .expect_err("duplicate id");
    assert!(unique_index_error(&error), "{error:?}");
    port.insert_row(
        "People",
        &[FieldValue::Integer(2), FieldValue::Text("alpha".into())],
    )
    .expect("duplicate non-unique value stays allowed");

    let error = port
        .update_row(
            "People",
            1,
            &[FieldValue::Integer(1), FieldValue::Text("gamma".into())],
        )
        .expect_err("duplicate id on update");
    assert!(unique_index_error(&error), "{error:?}");
    port.update_row(
        "People",
        1,
        &[FieldValue::Integer(2), FieldValue::Text("gamma".into())],
    )
    .expect("keeping the same unique value is allowed");

    port.delete_row("People", 0).expect("delete");
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("delta".into())],
    )
    .expect("a freed value can be reused");

    port.close();
    port.open(&_scratch.0, None).expect("reopen");
    let fields = port.tables()[0].fields.clone();
    assert_eq!(fields[0].properties.indexed, IndexMode::NoDuplicates);
    assert_eq!(fields[1].properties.indexed, IndexMode::DuplicatesOk);
    let error = port
        .insert_row(
            "People",
            &[FieldValue::Integer(1), FieldValue::Text("epsilon".into())],
        )
        .expect_err("uniqueness survives reopen");
    assert!(unique_index_error(&error), "{error:?}");
}

#[test]
fn a_batch_is_checked_as_a_whole() {
    let (mut port, _scratch) = opened();
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("alpha".into())],
    )
    .expect("seed");
    let error = port
        .write_row_ops(
            "People",
            &[
                datacraft_core::port::RowWriteOp::Insert(vec![
                    FieldValue::Integer(2),
                    FieldValue::Text("beta".into()),
                ]),
                datacraft_core::port::RowWriteOp::Insert(vec![
                    FieldValue::Integer(2),
                    FieldValue::Text("gamma".into()),
                ]),
            ],
        )
        .expect_err("the batch introduces a duplicate");
    assert!(unique_index_error(&error), "{error:?}");
    let rows = port
        .read_rows(
            "People",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read");
    assert_eq!(rows.len(), 1, "a rejected batch writes nothing");
}

#[test]
fn creating_a_unique_index_over_duplicates_is_refused() {
    let path = scratch_path("alter");
    let _scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create");
    port.open(&path, None).expect("open");
    let table = TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 20 })],
    );
    port.create_table(&table).expect("create table");
    for body in ["same", "same"] {
        port.insert_row("Notes", &[FieldValue::Text(body.into())])
            .expect("insert");
    }
    let error = port
        .alter_table(
            "Notes",
            datacraft_core::port::SchemaChange::SetFieldProperties {
                field: "body".into(),
                properties: FieldProperties {
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
        )
        .expect_err("duplicates block the unique index");
    assert!(error.to_string().contains("duplicate value"), "{error:?}");
    let before = port
        .read_rows(
            "Notes",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read");
    assert_eq!(before.len(), 2);
}

#[test]
fn unrelated_row_writes_keep_the_existing_index_section() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb");
    let path = scratch_path("preserve");
    let _scratch = Scratch(path.clone());
    std::fs::copy(&source, &path).expect("copy fixture");
    let before_bytes = std::fs::read(&path).expect("read");
    let reader = Jet4TableReader::new(before_bytes.clone()).expect("reader");
    let entry = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .find(|entry| entry.name == "Table4")
        .expect("Table4");
    let page = entry.tdef_page.expect("tdef");
    let before = JetTableDef::decode(&before_bytes, page).expect("decode");

    let mut port = Jet4Port::new();
    port.open(&path, None).expect("open");
    port.insert_row(
        "Table4",
        &[
            FieldValue::Text("inserted".into()),
            FieldValue::Guid([7u8; 16]),
        ],
    )
    .expect("insert with a unique primary key");
    let after_bytes = std::fs::read(&path).expect("read after");
    let after = JetTableDef::decode(&after_bytes, page).expect("decode after");
    assert_eq!(after.indexes, before.indexes, "index metadata survives");
    assert_eq!(
        after.index_section, before.index_section,
        "the raw index section is untouched"
    );
    assert!(before.indexes.iter().any(|index| index.primary));
}

#[test]
fn the_bus_reports_a_precise_unique_violation() {
    let path = scratch_path("core");
    let _scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    core.dispatch(Command::OpenDatabase {
        path: path.clone(),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateTable {
        definition: indexed_table(),
    });
    core.dispatch(Command::InsertRow {
        table: "People".into(),
        values: vec![FieldValue::Integer(1), FieldValue::Text("alpha".into())],
    });
    let events = core.dispatch(Command::InsertRow {
        table: "People".into(),
        values: vec![FieldValue::Integer(1), FieldValue::Text("beta".into())],
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("duplicate rejected");
    assert!(reason.contains("unique index"), "{reason}");
}

#[test]
fn memo_and_binary_fields_cannot_be_indexed() {
    let path = scratch_path("kinds");
    let _scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    core.dispatch(Command::OpenDatabase {
        path,
        password: None,
        source: None,
    });
    let mut memo = FieldDef::new("body", FieldKind::Memo);
    memo.properties.indexed = IndexMode::DuplicatesOk;
    let events = core.dispatch(Command::CreateTable {
        definition: TableDef::new("Notes", vec![memo]),
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("memo index rejected");
    assert!(reason.contains("indexing is not supported"), "{reason}");
}

#[test]
fn a_unique_index_added_by_alter_survives_reopen() {
    let path = scratch_path("alter-ok");
    let _scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create");
    port.open(&path, None).expect("open");
    let table = TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 20 })],
    );
    port.create_table(&table).expect("create table");
    for body in ["alpha", "beta"] {
        port.insert_row("Notes", &[FieldValue::Text(body.into())])
            .expect("insert");
    }
    port.alter_table(
        "Notes",
        datacraft_core::port::SchemaChange::SetFieldProperties {
            field: "body".into(),
            properties: FieldProperties {
                indexed: IndexMode::NoDuplicates,
                ..FieldProperties::default()
            },
        },
    )
    .expect("unique index added");
    port.close();

    port.open(&path, None).expect("reopen");
    assert_eq!(
        port.tables()[0].fields[0].properties.indexed,
        IndexMode::NoDuplicates
    );
    let error = port
        .insert_row("Notes", &[FieldValue::Text("alpha".into())])
        .expect_err("the new unique index is enforced");
    assert!(unique_index_error(&error), "{error:?}");
}

#[test]
fn adding_an_indexed_field_survives_reopen() {
    let path = scratch_path("add-field");
    let _scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 20 })],
    ))
    .expect("create table");
    port.alter_table(
        "Notes",
        datacraft_core::port::SchemaChange::AddField {
            field: FieldDef {
                name: "code".into(),
                kind: FieldKind::Number(NumericKind::LongInteger),
                properties: FieldProperties {
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
        },
    )
    .expect("add indexed field");
    port.close();
    port.open(&path, None).expect("reopen");
    let tables = port.tables();
    let field = tables[0]
        .fields
        .iter()
        .find(|field| field.name == "code")
        .expect("code field");
    assert_eq!(field.properties.indexed, IndexMode::NoDuplicates);
    port.insert_row(
        "Notes",
        &[FieldValue::Text("a".into()), FieldValue::Integer(1)],
    )
    .expect("first code");
    let error = port
        .insert_row(
            "Notes",
            &[FieldValue::Text("b".into()), FieldValue::Integer(1)],
        )
        .expect_err("duplicate code");
    assert!(unique_index_error(&error), "{error:?}");
}

#[test]
fn dropping_an_indexed_field_remaps_the_remaining_index() {
    let path = scratch_path("drop-field");
    let _scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&indexed_table()).expect("create table");
    port.alter_table(
        "People",
        datacraft_core::port::SchemaChange::DropField { field: "id".into() },
    )
    .expect("drop indexed field");
    port.close();
    port.open(&path, None).expect("reopen");
    assert_eq!(port.tables()[0].fields.len(), 1);
    assert_eq!(
        port.tables()[0].fields[0].properties.indexed,
        IndexMode::DuplicatesOk
    );
    port.insert_row("People", &[FieldValue::Text("same".into())])
        .expect("first name");
    port.insert_row("People", &[FieldValue::Text("same".into())])
        .expect("duplicate non-unique name");
    let bytes = std::fs::read(&path).expect("read");
    let reader = Jet4TableReader::new(bytes.clone()).expect("reader");
    let entry = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .find(|entry| entry.name == "People")
        .expect("People");
    let def = JetTableDef::decode(&bytes, entry.tdef_page.expect("tdef")).expect("decode");
    assert_eq!(def.indexes.len(), 1, "{:?}", def.indexes);
    assert_eq!(def.indexes[0].columns, [0]);
}

#[test]
fn removing_an_index_by_properties_survives_reopen() {
    let path = scratch_path("remove-index");
    let _scratch = Scratch(path.clone());
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create");
    port.open(&path, None).expect("open");
    port.create_table(&indexed_table()).expect("create table");
    port.alter_table(
        "People",
        datacraft_core::port::SchemaChange::SetFieldProperties {
            field: "id".into(),
            properties: FieldProperties::default(),
        },
    )
    .expect("drop the index");
    port.close();
    port.open(&path, None).expect("reopen");
    assert_eq!(
        port.tables()[0].fields[0].properties.indexed,
        IndexMode::None
    );
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("a".into())],
    )
    .expect("first row");
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("b".into())],
    )
    .expect("duplicates are allowed without the index");
}
