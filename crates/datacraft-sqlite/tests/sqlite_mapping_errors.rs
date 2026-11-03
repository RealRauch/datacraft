//! SQLite mapping error arms (phase 24 coverage gate).
//!
//! Non-SQLite content, truncated headers and unsupported kinds answer
//! precise errors instead of being misdetected.

use std::path::PathBuf;

use datacraft_core::port::{PortError, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_sqlite::SqlitePort;

fn scratch(stem: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-errors-{}-{stem}",
        std::process::id()
    ))
}

#[test]
fn non_sqlite_content_is_rejected_precisely() {
    let path = scratch("garbage.db");
    std::fs::write(&path, b"not a database at all").expect("write");
    let mut port = SqlitePort::new();
    let error = port.open(&path, None).expect_err("garbage is rejected");
    assert!(matches!(error, PortError::OpenFailed { .. }), "{error:?}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn truncated_magic_is_rejected_precisely() {
    let path = scratch("short.db");
    std::fs::write(&path, b"SQL").expect("write");
    let mut port = SqlitePort::new();
    let error = port.open(&path, None).expect_err("short magic is rejected");
    assert!(matches!(error, PortError::OpenFailed { .. }), "{error:?}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_missing_file_is_rejected_precisely() {
    let mut port = SqlitePort::new();
    let error = port
        .open(&scratch("missing.db"), None)
        .expect_err("missing file");
    assert!(matches!(error, PortError::OpenFailed { .. }), "{error:?}");
}

#[test]
fn complex_kinds_stay_unsupported_on_create() {
    let path = scratch("complex.db");
    {
        let _ = std::fs::remove_file(&path);
    }
    rusqlite::Connection::open(&path)
        .expect("fresh SQLite file")
        .execute_batch("CREATE TABLE seed (id INTEGER);")
        .expect("write the SQLite header");
    let mut port = SqlitePort::new();
    port.open(&path, None).expect("opens the fresh database");
    let definition = TableDef::new(
        "ComplexT",
        vec![
            FieldDef::new(
                "id",
                FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
            ),
            FieldDef::new("data", FieldKind::Complex),
        ],
    );
    let error = port
        .create_table(&definition)
        .expect_err("complex values are not representable");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(error.to_string().contains("complex"), "{error}");
    port.close();
    let _ = std::fs::remove_file(&path);
}
