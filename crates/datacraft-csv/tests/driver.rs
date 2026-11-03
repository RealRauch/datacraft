//! CSV directory driver contracts (phase 30, item M18; ADR-7): a
//! directory of `*.csv` files opens as one database — tables are files,
//! the header row names the columns, every column is Short Text, and any
//! write rewrites the file once (temp + rename), so `atomic_batch` is an
//! honest `Yes`. RED state: `CsvPort` does not exist yet — the suite
//! fails to compile until the driver lands.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{
    DatabaseFormat, DriverId, PortError, RowRange, SchemaChange, StoragePort, Support,
};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_csv::CsvPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("datacraft-csv-{}-{n}-{stem}", std::process::id()))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.0);
    }
}

fn csv_database() -> Scratch {
    let dir = scratch_path("data");
    std::fs::create_dir_all(&dir).expect("create the directory");
    std::fs::write(
        dir.join("people.csv"),
        "name,city\nada,london\ngrace,\"new york\"\n",
    )
    .expect("write the csv file");
    Scratch(dir)
}

/// The driver names itself and reports the honest set: tables and rows
/// are fully served (one rewrite per call = atomic), everything beyond
/// file-shaped text storage is `No`.
#[test]
fn the_csv_driver_reports_its_honest_capabilities() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    assert_eq!(port.driver_id(), DriverId::new("csv"));
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.write_rows);
    assert!(caps.create_table);
    assert!(caps.drop_table);
    assert!(caps.create_database);
    assert!(!caps.alter_table, "CSV has no alter path");
    assert!(caps.atomic_batch, "one rewrite per call is atomic");
    assert!(!caps.passwords);
    assert_eq!(caps.autonumber, Support::No);
    assert_eq!(caps.relationships, Support::No);
    assert_eq!(caps.native_queries, Support::No);
    assert_eq!(caps.macros, Support::No);
    assert_eq!(caps.vba, Support::No);
    assert_eq!(caps.triggers, Support::No);
    assert_eq!(caps.stored_procedures, Support::No);
}

/// Tables are the directory's `*.csv` files in sorted order; the header
/// row names Short Text columns; quoted fields parse; rows read in file
/// order.
#[test]
fn csv_tables_come_from_the_directory_listing() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    let tables = port.tables();
    assert_eq!(tables.len(), 1, "{tables:?}");
    assert_eq!(tables[0].name, "people");
    assert_eq!(tables[0].fields.len(), 2);
    assert!(matches!(tables[0].fields[0].kind, FieldKind::Text { .. }));
    assert_eq!(tables[0].fields[0].name, "name");
    assert_eq!(tables[0].fields[1].name, "city");
    let rows = port
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read the rows");
    assert_eq!(
        rows,
        vec![
            vec![
                FieldValue::Text("ada".to_owned()),
                FieldValue::Text("london".to_owned()),
            ],
            vec![
                FieldValue::Text("grace".to_owned()),
                FieldValue::Text("new york".to_owned()),
            ],
        ]
    );
}

/// Row writes rewrite the file once and survive a reopen; the arity is
/// checked against the header.
#[test]
fn csv_writes_survive_a_reopen() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    port.insert_row(
        "people",
        &[
            FieldValue::Text("alan".to_owned()),
            FieldValue::Text("wilmslow".to_owned()),
        ],
    )
    .expect("append the row");
    port.update_row(
        "people",
        0,
        &[
            FieldValue::Text("ada l".to_owned()),
            FieldValue::Text("london".to_owned()),
        ],
    )
    .expect("update the first row");
    port.delete_row("people", 1).expect("delete the second row");

    let mut reopened = CsvPort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    let rows = reopened
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read after reopen");
    // 2 initial rows + 1 insert − 1 delete.
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], FieldValue::Text("ada l".to_owned()));
    assert_eq!(rows[1][0], FieldValue::Text("alan".to_owned()));

    let error = reopened
        .insert_row("people", &[FieldValue::Text("one".to_owned())])
        .expect_err("arity is checked against the header");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
}

/// Creation makes the directory (never overwriting), creation of foreign
/// formats is refused, and table creation writes a header-only file.
#[test]
fn csv_create_database_and_create_table_behave() {
    let scratch = Scratch(scratch_path("fresh"));
    let mut port = CsvPort::new();
    port.create_database(&scratch.0, DatabaseFormat::Csv)
        .expect("create the directory database");
    assert!(scratch.0.is_dir());
    let error = port
        .create_database(&scratch.0, DatabaseFormat::Csv)
        .expect_err("creating twice is refused");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
    let error = port
        .create_database(&scratch.0, DatabaseFormat::Jet4)
        .expect_err("foreign formats are refused");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    port.open(&scratch.0, None)
        .expect("open the empty directory");
    port.create_table(&TableDef::new(
        "places",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 255 }),
            FieldDef::new("country", FieldKind::Text { max_len: 255 }),
        ],
    ))
    .expect("create the header-only file");
    assert!(scratch.0.join("places.csv").exists());
    assert_eq!(port.tables()[0].name, "places");
}

/// Reported gaps answer precisely: no alter path, no relationships, and
/// a non-directory path fails the open with a locatable reason.
#[test]
fn csv_capability_gaps_answer_precisely() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");

    let error = port
        .alter_table(
            "people",
            SchemaChange::RenameField {
                field: "name".to_owned(),
                new_name: "who".to_owned(),
            },
        )
        .expect_err("alter_table is unsupported on CSV");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(port.relationships().expect("none").is_empty());

    let file = scratch_path("plain.txt");
    std::fs::write(&file, "not a database").expect("write the file");
    let mut stranger = CsvPort::new();
    let error = stranger
        .open(&file, None)
        .expect_err("a plain file is no CSV directory");
    assert!(
        matches!(
            error,
            PortError::OpenFailed { .. } | PortError::Unsupported { .. }
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains(&file.display().to_string()));
    let _ = std::fs::remove_file(&file);
}

/// Hostile table names are rejected precisely; ragged rows fail the open
/// with a locatable reason (LESSONS #1/#4).
#[test]
fn csv_rejects_hostile_names_and_ragged_rows() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    let error = port
        .create_table(&TableDef::new(
            "../escape",
            vec![FieldDef::new("a", FieldKind::Text { max_len: 255 })],
        ))
        .expect_err("path separators are rejected");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");

    std::fs::write(scratch.0.join("ragged.csv"), "a,b\n1\n").expect("write ragged");
    let mut stranger = CsvPort::new();
    let error = stranger
        .open(&scratch.0, None)
        .expect_err("a ragged row fails the open");
    assert!(error.to_string().contains("ragged"), "{error}");
    let _ = std::fs::remove_file(scratch.0.join("ragged.csv"));
}

/// L12 regression: an invalid definition is refused before any file is
/// written (the Jet and SQLite drivers already validate).
#[test]
fn create_table_validates_the_definition_before_writing() {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    let invalid = TableDef::new(
        "dupes",
        vec![
            FieldDef::new("a", FieldKind::Text { max_len: 10 }),
            FieldDef::new("A", FieldKind::Text { max_len: 10 }),
        ],
    );
    let err = port
        .create_table(&invalid)
        .expect_err("duplicate field names must be refused");
    assert!(
        err.to_string().to_lowercase().contains("duplicate"),
        "{err}"
    );
    assert!(
        !scratch.0.join("dupes.csv").exists(),
        "no file may be written before validation"
    );
    let mut reopened = CsvPort::new();
    reopened.open(&scratch.0, None).expect("directory still opens");
    assert_eq!(reopened.tables().len(), 1);
}
