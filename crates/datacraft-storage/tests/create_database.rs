//! Fresh-file creation contracts (phase 29, step 29.1, item H44).
//!
//! A brand-new database is created from nothing, reopens through the same
//! boundaries every other file uses, and takes the full write path. The
//! Jet4 builder is byte-deterministic for a fixed creation date (LESSONS
//! #6). RED state: `create_jet4_file` and `StoragePort::create_database`
//! do not exist yet — the suite fails to compile until they land.

use std::path::{Path, PathBuf};
use std::process::id;

use datacraft_core::port::{DatabaseFormat, StoragePort};
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_storage::jet::{FileHeader, Jet4TableReader};
use datacraft_storage::port_adapter::Jet4Port;
use datacraft_storage::sqlite::SqlitePort;
use datacraft_storage::{FormatVersion, StorageError};

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-storage-create-{}-{name}",
        id()
    ))
}

fn jet_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("the created file exists")
}

#[test]
fn a_fresh_jet4_file_opens_with_an_empty_catalog() {
    let path = scratch("fresh.mdb");
    datacraft_storage::jet::create::create_jet4_file(&path, 45_000.0).expect("create");
    let bytes = jet_bytes(&path);

    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("valid header");
    assert_eq!(header.format, FormatVersion::Jet4);
    assert_eq!(header.page_count, 4, "header, free map, catalog, usage page");
    assert_eq!(bytes.len(), 4 * 4096);

    let reader = Jet4TableReader::new(bytes).expect("the fresh file reopens");
    assert!(reader.catalog().expect("catalog decodes").is_empty());

    let mut port = Jet4Port::new();
    let opened = port.open(&path, None).expect("the port opens the fresh file");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert!(port.tables().is_empty(), "no user tables yet");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creation_is_deterministic_for_a_fixed_creation_date() {
    let first = scratch("det-a.mdb");
    let second = scratch("det-b.mdb");
    datacraft_storage::jet::create::create_jet4_file(&first, 45_000.0).expect("create a");
    datacraft_storage::jet::create::create_jet4_file(&second, 45_000.0).expect("create b");
    assert_eq!(jet_bytes(&first), jet_bytes(&second), "same input, same bytes");
    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

#[test]
fn a_fresh_database_takes_the_full_write_path() {
    let path = scratch("lifecycle.mdb");
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4).expect("create through the port");
    port.open(&path, None).expect("open the fresh file");

    let people = TableDef::new(
        "People",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    );
    port.create_table(&people).expect("create a table in the fresh file");
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("Ada".into())],
    )
    .expect("insert a row");
    port.close();

    let mut reopened = Jet4Port::new();
    reopened.open(&path, None).expect("reopen");
    let rows = reopened
        .read_rows(
            "People",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("the row persisted");
    assert_eq!(rows, [vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())]]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creating_over_an_existing_file_is_refused_and_leaves_it_intact() {
    let path = scratch("exists.mdb");
    datacraft_storage::jet::create::create_jet4_file(&path, 45_000.0).expect("first create");
    let before = jet_bytes(&path);

    let error =
        datacraft_storage::jet::create::create_jet4_file(&path, 46_000.0).expect_err("refused");
    match error {
        StorageError::Unsupported { reason } => {
            assert!(reason.contains("already exists"), "{reason}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    assert_eq!(jet_bytes(&path), before, "the refused create touched nothing");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_jet_backend_refuses_every_other_format_precisely() {
    let mut port = Jet4Port::new();
    for (format, needle) in [
        (DatabaseFormat::Ace, "ACE"),
        (DatabaseFormat::Jet3, "Jet3"),
        (DatabaseFormat::Sqlite, "SQLite"),
    ] {
        let error = port
            .create_database(&scratch("wrong.mdb"), format)
            .expect_err("{format:?} is not a Jet4 creation target");
        match error {
            datacraft_core::port::PortError::Unsupported { reason } => {
                assert!(reason.contains(needle), "{format:?}: {reason}");
            }
            other => panic!("{format:?}: expected Unsupported, got {other:?}"),
        }
    }
}

#[test]
fn the_sqlite_backend_creates_an_openable_database() {
    let path = scratch("fresh.sqlite");
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite).expect("create");

    let bytes = jet_bytes(&path);
    assert!(bytes.starts_with(b"SQLite format 3\0"), "real SQLite header");

    let opened = port.open(&path, None).expect("reopen the fresh database");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert!(port.tables().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_sqlite_backend_refuses_jet4_targets() {
    let mut port = SqlitePort::new();
    let error = port
        .create_database(&scratch("wrong.sqlite"), DatabaseFormat::Jet4)
        .expect_err("SQLite does not create Jet files");
    assert!(
        matches!(error, datacraft_core::port::PortError::Unsupported { .. }),
        "{error:?}"
    );
}
