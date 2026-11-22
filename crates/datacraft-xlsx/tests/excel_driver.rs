//! Excel read-driver contract over the pinned fixtures (phase 96, step
//! 96.2; item M127).
//!
//! The same workbook is committed in all four families (`.xlsx`, `.xlsm`,
//! `.xls`, `.xlsb`, built by `tools/xlsx-fixtures/build.ps1`). Every
//! family must list the same sheets as tables, infer the same column
//! kinds and deliver the same typed values — the format matrix boundary
//! is the reader library's in-memory model, never a per-format branch in
//! DataCraft.
//!
//! Fixture content: `Items` (Name, Qty, Price, Added, Active, Note) with
//! two rows — the first has an empty Note — and `Summary` (Region, Total)
//! whose last Total is a formula with a saved cached value.

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{FieldKind, FieldValue, NumericKind};
use datacraft_xlsx::ExcelPort;

fn fixture(extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(format!("excelFixture.{extension}"))
}

/// Excel serial days for 2024-01-15 and 2024-02-20 (OLE automation days;
/// identical for dates from 1900-03-01 on).
const JANUARY: f64 = 45_306.0;
const FEBRUARY: f64 = 45_342.0;

fn open(extension: &str) -> ExcelPort {
    let mut port = ExcelPort::new();
    let opened = port
        .open(&fixture(extension), None)
        .unwrap_or_else(|error| panic!("{extension} must open: {error}"));
    assert_eq!(opened.format, DatabaseFormat::Excel);
    assert_eq!(opened.name, "excelFixture");
    port
}

fn rows(port: &ExcelPort, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read table")
}

#[test]
fn every_family_lists_the_sheets_as_tables_with_inferred_kinds() {
    for extension in ["xlsx", "xlsm", "xls", "xlsb"] {
        let port = open(extension);
        let tables = port.tables();
        assert_eq!(
            tables.iter().map(|table| &table.name).collect::<Vec<_>>(),
            vec!["Items", "Summary"],
            "{extension}: sheets become tables in workbook order"
        );
        let items = tables
            .iter()
            .find(|table| table.name == "Items")
            .expect("Items");
        assert_eq!(
            items
                .fields
                .iter()
                .map(|field| field.kind)
                .collect::<Vec<_>>(),
            vec![
                FieldKind::Text { max_len: 255 },
                FieldKind::Number(NumericKind::LongInteger),
                FieldKind::Number(NumericKind::Double),
                FieldKind::DateTime,
                FieldKind::Boolean { tri_state: false },
                FieldKind::Text { max_len: 255 },
            ],
            "{extension}: column kinds are inferred from the cells"
        );
    }
}

#[test]
fn every_family_delivers_the_same_typed_values() {
    for extension in ["xlsx", "xlsm", "xls", "xlsb"] {
        let port = open(extension);
        assert_eq!(
            rows(&port, "Items"),
            vec![
                vec![
                    FieldValue::Text("Ada".into()),
                    FieldValue::Integer(3),
                    FieldValue::Float(12.5),
                    FieldValue::DateTime(JANUARY),
                    FieldValue::Boolean(true),
                    FieldValue::Null,
                ],
                vec![
                    FieldValue::Text("Grace".into()),
                    FieldValue::Integer(7),
                    FieldValue::Float(4.25),
                    FieldValue::DateTime(FEBRUARY),
                    FieldValue::Boolean(false),
                    FieldValue::Text("hello".into()),
                ],
            ],
            "{extension}: typed cells, empty cell as Null"
        );
        assert_eq!(
            rows(&port, "Summary"),
            vec![
                vec![FieldValue::Text("West".into()), FieldValue::Integer(5)],
                vec![FieldValue::Text("East".into()), FieldValue::Integer(7)],
                vec![FieldValue::Text("All".into()), FieldValue::Integer(12)],
            ],
            "{extension}: the formula's saved cached value is read"
        );
    }
}

#[test]
fn row_windows_follow_the_datasheet_contract() {
    let port = open("xlsx");
    let window = port
        .read_rows("Items", RowRange { start: 1, len: 1 })
        .expect("window");
    assert_eq!(window.len(), 1);
    assert_eq!(window[0][0], FieldValue::Text("Grace".into()));
    let error = port
        .read_rows("Items", RowRange { start: 3, len: 1 })
        .expect_err("beyond the rows");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");
    assert!(matches!(
        port.read_rows("Missing", RowRange { start: 0, len: 1 }),
        Err(PortError::TableNotFound(_))
    ));
}

#[test]
fn workbooks_are_read_only() {
    let mut port = open("xlsx");
    let error = port
        .insert_row("Items", &[FieldValue::Text("x".into())])
        .expect_err("read-only");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(
        port.tables()
            .iter()
            .all(|table| !table.name.starts_with("MSys"))
    );
}

#[test]
fn an_ods_document_is_a_precise_boundary() {
    let mut port = ExcelPort::new();
    let path =
        std::env::temp_dir().join(format!("datacraft-ods-boundary-{}.ods", std::process::id()));
    // A tiny zip-shaped ODS is enough: the driver must refuse by content,
    // not by crashing on the malformed archive.
    std::fs::write(&path, b"PK\x03\x04not-an-ods").expect("write probe");
    let result = port.open(&path, None);
    let _ = std::fs::remove_file(&path);
    match result {
        Err(PortError::Unsupported { reason }) => {
            assert!(reason.contains("ODF"), "{reason}");
        }
        Err(PortError::OpenFailed { detail }) => {
            // calamine declines the malformed archive before the family
            // check; the refusal must still be locatable.
            assert!(detail.contains("ods"), "{detail}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
}
