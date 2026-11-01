//! ACE complex values: attachments, multi-value and version-history columns
//! (step 16.1, item M4).
//!
//! The `MSysComplexColumns` catalog maps a conceptual table's complex column
//! to its `f_<guid>_*` flat table; the parent row stores a row-scoped complex
//! id in the complex column, and the flat table's `_<name>` link column
//! carries that id. The decoder groups the flat rows per parent row and hides
//! the flat tables from the user catalog.

use std::path::{Path, PathBuf};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{BinaryKind, ComplexRecord, FieldKind, FieldValue, MultiValueKind};
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn open(name: &str) -> DetectingPort {
    let mut port = DetectingPort::new();
    port.open(&fixture(name))
        .unwrap_or_else(|e| panic!("open {name}: {e}"));
    port
}

fn rows(port: &DetectingPort, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .unwrap_or_else(|e| panic!("read {table}: {e}"))
}

fn kind_of(port: &DetectingPort, table: &str, field: &str) -> FieldKind {
    let definition = port
        .tables()
        .into_iter()
        .find(|t| t.name == table)
        .unwrap_or_else(|| panic!("table {table} is visible"));
    definition
        .fields
        .iter()
        .find(|f| f.name == field)
        .unwrap_or_else(|| panic!("field {field} exists"))
        .kind
}

fn complex(value: &FieldValue) -> &[ComplexRecord] {
    match value {
        FieldValue::Complex(records) => records,
        other => panic!("expected a complex value, found {other:?}"),
    }
}

fn scalar_text(record: &ComplexRecord) -> String {
    match record.scalar() {
        Some(FieldValue::Text(text)) => text.clone(),
        other => panic!("expected a text scalar, found {other:?} in {record:?}"),
    }
}

fn text_field(record: &ComplexRecord, name: &str) -> String {
    match record.field(name) {
        Some(FieldValue::Text(text)) => text.clone(),
        other => panic!("expected text field {name}, found {other:?} in {record:?}"),
    }
}

#[test]
fn attachment_columns_resolve_to_their_flat_rows() {
    let port = open("attachmentsV2016.accdb");
    assert_eq!(
        kind_of(&port, "Table1", "Attachments"),
        FieldKind::Binary(BinaryKind::Attachment)
    );
    let rows = rows(&port, "Table1");
    assert_eq!(rows.len(), 3);

    let first = complex(&rows[0][2]);
    assert!(first.is_empty(), "row 1 carries no attachments: {first:?}");

    let second = complex(&rows[1][2]);
    assert_eq!(second.len(), 1);
    assert_eq!(text_field(&second[0], "FileName"), "attachment1.txt");
    assert_eq!(text_field(&second[0], "FileType"), "txt");
    assert!(
        matches!(second[0].field("FileData"), Some(FieldValue::Binary(bytes)) if !bytes.is_empty()),
        "the attachment payload is decoded: {:?}",
        second[0]
    );

    let third = complex(&rows[2][2]);
    let names: Vec<String> = third
        .iter()
        .map(|record| text_field(record, "FileName"))
        .collect();
    assert_eq!(
        names,
        ["attachment1.txt", "attachment2.txt", "attachment3.txt"]
    );
}

#[test]
fn multi_value_columns_resolve_to_their_flat_rows() {
    let port = open("complexDataV2007.accdb");
    assert_eq!(
        kind_of(&port, "Table1", "multi-value-data"),
        FieldKind::MultiValue(MultiValueKind::Text { max_len: 510 })
    );
    let rows = rows(&port, "Table1");
    assert_eq!(rows.len(), 4);

    let values: Vec<Vec<String>> = rows
        .iter()
        .map(|row| complex(&row[4]).iter().map(scalar_text).collect())
        .collect();
    assert_eq!(
        values,
        vec![
            Vec::<String>::new(),
            vec!["value1".to_owned(), "value4".to_owned()],
            vec![
                "value1".to_owned(),
                "value2".to_owned(),
                "value3".to_owned(),
                "value4".to_owned()
            ],
            Vec::<String>::new(),
        ]
    );
}

#[test]
fn attachment_and_version_history_columns_resolve_per_row() {
    let port = open("complexDataV2007.accdb");
    assert_eq!(
        kind_of(&port, "Table1", "attach-data"),
        FieldKind::Binary(BinaryKind::Attachment)
    );
    let version_history = port
        .tables()
        .into_iter()
        .find(|t| t.name == "Table1")
        .expect("Table1")
        .fields
        .into_iter()
        .find(|f| f.name.starts_with("VersionHistory_"))
        .expect("version-history field");
    assert_eq!(version_history.kind, FieldKind::Complex);

    let rows = rows(&port, "Table1");
    let attachments: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            complex(&row[5])
                .iter()
                .map(|record| text_field(record, "FileName"))
                .collect()
        })
        .collect();
    assert_eq!(
        attachments,
        vec![
            Vec::<String>::new(),
            vec!["test_data.txt".to_owned(), "test_data2.txt".to_owned()],
            Vec::<String>::new(),
            vec!["test_data2.txt".to_owned()],
        ]
    );

    let history: Vec<Vec<String>> = rows
        .iter()
        .map(|row| complex(&row[1]).iter().map(scalar_text).collect())
        .collect();
    assert_eq!(
        history,
        vec![
            Vec::<String>::new(),
            vec!["row2-memo".to_owned()],
            vec![
                "row3-memo".to_owned(),
                "row3-memo-revised".to_owned(),
                "row3-memo-again".to_owned()
            ],
            vec!["row4-memo".to_owned()],
        ]
    );
}

#[test]
fn complex_backing_tables_are_hidden_from_the_user_catalog() {
    for name in ["attachmentsV2016.accdb", "complexDataV2007.accdb"] {
        let port = open(name);
        let visible: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
        assert!(
            visible.iter().all(|table| !table.starts_with("f_")),
            "{name} hides its flat tables: {visible:?}"
        );
        assert!(visible.iter().any(|table| table == "Table1"));
    }
}

#[test]
fn addressing_a_hidden_flat_table_is_a_precise_table_not_found() {
    let port = open("attachmentsV2016.accdb");
    let err = port
        .read_rows(
            "f_F42896D510474144890B9076E2CD27D9_Field1",
            RowRange { start: 0, len: 1 },
        )
        .expect_err("the backing table is not addressable");
    assert!(matches!(err, PortError::TableNotFound(_)), "{err:?}");
}

#[test]
fn complex_values_stay_null_free_empty_lists() {
    // An empty complex cell is an empty list, never Null — the datasheet
    // distinguishes "no attachments" from "NULL" (the parent row always
    // carries a complex id).
    let port = open("attachmentsV2016.accdb");
    let rows = rows(&port, "Table1");
    assert_eq!(rows[0][2], FieldValue::Complex(Vec::new()));
}
