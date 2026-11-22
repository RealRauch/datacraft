//! Compact for Jet4/ACE (phase 95, step 95.2; item M126).
//!
//! Compact reads everything, writes a fresh verified file of the same
//! family, swaps it atomically at the original path and re-opens the
//! session. Tables, rows, AutoNumber identities, indexes and user
//! relationships must survive; identical input must produce identical
//! bytes; password-protected and read-only sessions refuse precisely
//! without touching the file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn scratch_path(stem: &str, extension: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-jet-compact-{}-{n}-{stem}.{extension}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn copy_of(name: &str, stem: &str) -> Scratch {
    let path = scratch_path(stem, name.rsplit('.').next().unwrap_or("mdb"));
    std::fs::copy(fixture(name), &path).expect("copy the fixture");
    Scratch(path)
}

fn snapshot(port: &Jet4Port) -> Vec<(String, Vec<Vec<FieldValue>>)> {
    port.tables()
        .into_iter()
        .map(|table| {
            let rows = port
                .read_rows(
                    &table.name,
                    datacraft_core::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .expect("read table");
            (table.name, rows)
        })
        .collect()
}

#[test]
fn compacts_a_jet4_database_in_place_preserving_tables_rows_and_relationships() {
    let path = scratch_path("built", "mdb");
    let scratch = Scratch(path.clone());
    {
        let mut port = Jet4Port::new();
        port.create_database(&path, DatabaseFormat::Jet4)
            .expect("create");
        port.open(&path, None).expect("open");
        port.create_table(&TableDef::new(
            "Customers",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("name", FieldKind::Text { max_len: 20 }),
            ],
        ))
        .expect("customers");
        port.create_table(&TableDef::new(
            "Orders",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
            ],
        ))
        .expect("orders");
        port.create_table(&TableDef::new(
            "Notes",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new("body", FieldKind::Text { max_len: 20 }),
            ],
        ))
        .expect("notes");
        port.insert_row(
            "Customers",
            &[FieldValue::Integer(1), FieldValue::Text("Ada".into())],
        )
        .expect("customer row");
        port.insert_row("Orders", &[FieldValue::Integer(10), FieldValue::Integer(1)])
            .expect("order row");
        port.insert_row(
            "Notes",
            &[FieldValue::Null, FieldValue::Text("first".into())],
        )
        .expect("note row");
        port.insert_row(
            "Notes",
            &[FieldValue::Null, FieldValue::Text("second".into())],
        )
        .expect("note row");
        port.create_relationship(&datacraft_core::port::Relationship {
            table: "Orders".into(),
            column: "customer_id".into(),
            related_table: "Customers".into(),
            related_column: "id".into(),
        })
        .expect("relationship");
    }

    let mut port = Jet4Port::new();
    port.open(&scratch.0, None).expect("open before compact");
    let before = snapshot(&port);
    let before_relationships = port.relationships().expect("relationships");
    port.compact().expect("compact");

    assert_eq!(port.tables().len(), 3, "tables stay");
    assert_eq!(snapshot(&port), before, "rows and order stay");
    assert_eq!(
        port.relationships().expect("relationships"),
        before_relationships,
        "relationships stay"
    );
    assert_eq!(
        port.source_facts().health,
        None,
        "the compacted file is healthy"
    );

    // AutoNumber identities stay and the counter continues after them.
    port.insert_row(
        "Notes",
        &[FieldValue::Null, FieldValue::Text("third".into())],
    )
    .expect("write after compact");
    let notes = snapshot(&port)
        .into_iter()
        .find(|(name, _)| name == "Notes")
        .expect("notes");
    assert_eq!(
        notes.1.last().and_then(|row| row.first()),
        Some(&FieldValue::Integer(3)),
        "the AutoNumber counter continues after the restored identities"
    );

    // A new row still lands after the swap (the session is writable).
    port.insert_row(
        "Customers",
        &[FieldValue::Integer(2), FieldValue::Text("Grace".into())],
    )
    .expect("write after compact");
    port.close();

    let mut reopened = Jet4Port::new();
    reopened
        .open(&scratch.0, None)
        .expect("reopen the compacted file");
    assert_eq!(reopened.tables().len(), 3);
    let customers = snapshot(&reopened)
        .into_iter()
        .find(|(name, _)| name == "Customers")
        .expect("customers");
    assert_eq!(customers.1.len(), 2, "both rows persist");
}

#[test]
fn compacts_the_pinned_jet4_fixture_and_reopens_clean() {
    let scratch = copy_of("testV2000.mdb", "fixture");
    let mut port = Jet4Port::new();
    port.open(&scratch.0, None).expect("open");
    let before = snapshot(&port);
    assert!(!before.is_empty());
    port.compact().expect("compact");
    assert_eq!(snapshot(&port), before);
    port.close();

    let mut reopened = Jet4Port::new();
    reopened.open(&scratch.0, None).expect("reopen");
    assert_eq!(snapshot(&reopened), before);
    assert_eq!(reopened.source_facts().health, None);
}

#[test]
fn compact_preserves_the_ace_format() {
    let scratch = copy_of("testV2010.accdb", "ace");
    let mut port = Jet4Port::new();
    let opened = port.open(&scratch.0, None).expect("open");
    assert_eq!(opened.format, DatabaseFormat::Ace);
    let before = snapshot(&port);
    port.compact().expect("compact");
    assert_eq!(port.tables().len(), before.len());
    assert_eq!(snapshot(&port), before);
    port.close();

    let mut reopened = Jet4Port::new();
    let opened = reopened.open(&scratch.0, None).expect("reopen");
    assert_eq!(opened.format, DatabaseFormat::Ace, "the family stays ACE");
    assert_eq!(snapshot(&reopened), before);
}

#[test]
fn compact_is_byte_deterministic_for_identical_input() {
    let first = copy_of("testV2000.mdb", "det-a");
    let second = copy_of("testV2000.mdb", "det-b");
    for scratch in [&first, &second] {
        let mut port = Jet4Port::new();
        port.open(&scratch.0, None).expect("open");
        port.compact().expect("compact");
    }
    let a = std::fs::read(&first.0).expect("first bytes");
    let b = std::fs::read(&second.0).expect("second bytes");
    assert_eq!(a, b, "same input state must produce the same bytes");
}

#[test]
fn compact_refuses_jet3_read_only_sources() {
    let scratch = copy_of("testV1997.mdb", "jet3");
    let mut port = Jet4Port::new();
    port.open(&scratch.0, None).expect("open Jet3");
    let before = std::fs::read(&scratch.0).expect("bytes");
    let error = port.compact().expect_err("Jet3 is read-only");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert_eq!(
        std::fs::read(&scratch.0).expect("bytes"),
        before,
        "the refusal leaves the file untouched"
    );
}

#[test]
fn compact_refuses_password_protected_sources() {
    let scratch = copy_of("passwordV2000.mdb", "protected");
    let mut port = Jet4Port::new();
    port.open(&scratch.0, Some("password"))
        .expect("open protected");
    let before = std::fs::read(&scratch.0).expect("bytes");
    let error = port
        .compact()
        .expect_err("password protection is preserved by refusal");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert_eq!(
        std::fs::read(&scratch.0).expect("bytes"),
        before,
        "the refusal leaves the file untouched"
    );
}
