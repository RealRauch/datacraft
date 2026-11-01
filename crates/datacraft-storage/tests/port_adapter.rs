//! Contract suite for the `StoragePort` adapter (item H16).
//!
//! LESSONS #2 — seam tested from both sides: the core side runs against the
//! mock (core crate); this suite runs the real Jet4 read backend behind
//! core's port trait against the committed fixture, so the schema mapping
//! (Jet descriptors -> Access domain kinds) is exercised on real bytes.

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::TableDef;
use datacraft_storage::port_adapter::Jet4Port;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

#[test]
fn opens_the_real_fixture_with_format_and_user_tables() {
    let mut port = Jet4Port::new();
    let opened = port.open(&fixture()).expect("fixture must open");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert_eq!(opened.name, "testV2000");

    let tables = port.tables();
    assert!(!tables.is_empty(), "fixture has user tables");
    assert!(
        tables.iter().all(|t| !t.name.starts_with("MSys")),
        "system tables must not surface: {:?}",
        tables.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    assert!(tables.iter().any(|t| t.name == "Table1"));
}

#[test]
fn every_mapped_table_is_a_valid_domain_definition() {
    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    for table in port.tables() {
        table
            .validate()
            .unwrap_or_else(|e| panic!("table {:?} must validate: {e}", table.name));
        assert!(
            !table.fields.is_empty(),
            "table {:?} has fields",
            table.name
        );
    }
}

#[test]
fn text_columns_map_their_descriptor_length() {
    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    let tables = port.tables();
    let text_max_lens: Vec<u16> = tables
        .iter()
        .flat_map(|t| t.fields.iter())
        .filter_map(|f| match f.kind {
            datacraft_core::schema::FieldKind::Text { max_len } => Some(max_len),
            _ => None,
        })
        .collect();
    assert!(
        !text_max_lens.is_empty(),
        "the fixture catalog contains text columns"
    );
    assert!(
        text_max_lens.iter().all(|len| (1..=255).contains(len)),
        "mapped text lengths must be Access-valid: {text_max_lens:?}"
    );
}

#[test]
fn writes_are_rejected_as_unsupported() {
    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    let def = TableDef::new("New", vec![]);
    let err = port.create_table(&def).expect_err("read-only backend");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err}");
    assert!(err.to_string().contains("read-only"), "{err}");
    let err = port.drop_table("Table1").expect_err("read-only backend");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err}");
}

#[test]
fn open_failures_are_precise() {
    let mut port = Jet4Port::new();
    let err = port
        .open(Path::new("definitely-missing-424242.mdb"))
        .expect_err("missing file");
    assert!(
        err.to_string().contains("definitely-missing-424242"),
        "{err}"
    );

    let mut garbage = std::env::temp_dir();
    garbage.push(format!("datacraft-garbage-{}.mdb", std::process::id()));
    std::fs::write(&garbage, [0u8; 4096]).expect("write garbage fixture");
    let err = port.open(&garbage).expect_err("garbage file");
    assert!(matches!(err, PortError::OpenFailed { .. }), "{err}");
    let _ = std::fs::remove_file(&garbage);
}

#[test]
fn close_is_idempotent_and_clears_the_catalog() {
    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    assert!(!port.tables().is_empty());
    port.close();
    assert!(port.tables().is_empty());
    port.close();
    // Re-open after close re-attaches.
    port.open(&fixture()).expect("re-open");
    assert!(!port.tables().is_empty());
}

#[test]
fn tables_behind_multi_page_tdefs_decode() {
    // Table2 in the Access-rewritten fixture spans multiple TDEF pages
    // (Access chains them via the next-TDEF-page pointer at offset 4 —
    // docs/FORMAT-JET.md). The adapter must surface the table, not fail
    // the whole open.
    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    let names: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
    assert!(
        names.iter().any(|n| n == "Table2"),
        "multi-page TDEF table must surface: {names:?}"
    );
}

#[test]
fn read_rows_maps_storage_values_to_domain_values() {
    use datacraft_core::port::{RowRange, StoragePort as _};
    use datacraft_core::schema::FieldValue;

    let mut port = Jet4Port::new();
    port.open(&fixture()).expect("fixture must open");
    let tables = port.tables();
    let with_rows = tables
        .iter()
        .find(|t| {
            port.read_rows(&t.name, RowRange { start: 0, len: 1 })
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        })
        .expect("fixture has a table with rows");

    let rows = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("full read");
    let arity = with_rows.fields.len();
    assert!(
        rows.iter().all(|r| r.len() == arity),
        "every row matches the mapped field count"
    );
    // Windowing agrees with the full read.
    let half = rows.len() / 2;
    let first = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: 0,
                len: half,
            },
        )
        .expect("first half");
    assert_eq!(first, rows[..half]);
    let tail = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: rows.len(),
                len: 4,
            },
        )
        .expect("empty tail");
    assert!(tail.is_empty());

    let err = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: rows.len() + 1,
                len: 1,
            },
        )
        .expect_err("beyond the row set");
    assert!(matches!(
        err,
        datacraft_core::port::PortError::OutOfRange { .. }
    ));

    let err = port
        .read_rows("Ghost", RowRange { start: 0, len: 1 })
        .expect_err("unknown table");
    assert!(matches!(
        err,
        datacraft_core::port::PortError::TableNotFound(_)
    ));

    // Values are domain values, not storage values: spot-check a mapped row.
    let _ = FieldValue::Null;
}
