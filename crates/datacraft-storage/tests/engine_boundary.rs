//! Boundary contract tests for the [`StorageEngine`] trait (step 2.1, item H4).
//!
//! The mock backend is the reference implementation of the contract: whatever
//! holds for `InMemoryEngine` must hold for every real backend (jet-rs, oracle).
//! These tests are written test-first — RED until the boundary exists.

use std::path::Path;

use datacraft_storage::{
    ColumnSchema, DatabaseMeta, FormatVersion, InMemoryEngine, RowOp, RowRange, StorageDataType,
    StorageEngine, StorageError, StorageValue, TableSchema,
};

fn customers_table() -> TableSchema {
    TableSchema::new(
        "Customers",
        vec![
            ColumnSchema::new("Id", StorageDataType::Int4, false),
            ColumnSchema::new("Name", StorageDataType::Text, true),
            ColumnSchema::new("Active", StorageDataType::Bit, true),
        ],
    )
}

fn customers_rows() -> Vec<Vec<StorageValue>> {
    vec![
        vec![
            StorageValue::Int(1),
            StorageValue::Text("Ada".into()),
            StorageValue::Bit(true),
        ],
        vec![
            StorageValue::Int(2),
            StorageValue::Null,
            StorageValue::Bit(false),
        ],
        vec![
            StorageValue::Int(3),
            StorageValue::Text("Grace".into()),
            StorageValue::Null,
        ],
    ]
}

fn engine_with_customers() -> InMemoryEngine {
    let mut engine = InMemoryEngine::new(FormatVersion::Jet4);
    engine.create_table(customers_table());
    for row in customers_rows() {
        engine
            .write_rows("Customers", &[RowOp::Insert(row)])
            .expect("insert into mock");
    }
    engine
}

#[test]
fn open_reports_format_version() {
    let mut engine = engine_with_customers();
    let meta = engine.open(Path::new("")).expect("open mock");
    assert_eq!(meta, DatabaseMeta::new(FormatVersion::Jet4));
}

#[test]
fn schema_lists_tables_in_creation_order() {
    let mut engine = engine_with_customers();
    engine.create_table(TableSchema::new(
        "Orders",
        vec![ColumnSchema::new("Total", StorageDataType::Currency, true)],
    ));
    let schema = engine.schema().expect("schema");
    let names: Vec<&str> = schema.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Customers", "Orders"]);
    assert_eq!(schema[0].columns[1].name, "Name");
    assert_eq!(schema[0].columns[1].data_type, StorageDataType::Text);
    assert!(schema[0].columns[0].nullable == false);
}

#[test]
fn read_rows_returns_requested_window_with_nulls() {
    let engine = engine_with_customers();
    let batch = engine
        .read_rows("Customers", RowRange::at(1, 2))
        .expect("read window");
    assert_eq!(batch.start_row, 1);
    assert_eq!(batch.rows.len(), 2);
    assert_eq!(batch.rows[0][1], StorageValue::Null);
    assert_eq!(batch.rows[1][1], StorageValue::Text("Grace".into()));
}

#[test]
fn read_rows_out_of_range_is_an_error() {
    let engine = engine_with_customers();
    let err = engine
        .read_rows("Customers", RowRange::at(5, 2))
        .expect_err("out of range must fail");
    assert!(matches!(err, StorageError::OutOfRange { .. }));
}

#[test]
fn read_rows_unknown_table_is_an_error() {
    let engine = engine_with_customers();
    let err = engine
        .read_rows("Missing", RowRange::at(0, 1))
        .expect_err("unknown table must fail");
    assert!(matches!(err, StorageError::TableNotFound(ref t) if t == "Missing"));
}

#[test]
fn write_rows_update_and_delete_report_counts() {
    let mut engine = engine_with_customers();
    let report = engine
        .write_rows(
            "Customers",
            &[
                RowOp::Update {
                    row: 0,
                    values: vec![
                        StorageValue::Int(1),
                        StorageValue::Text("Adalovelace".into()),
                        StorageValue::Bit(true),
                    ],
                },
                RowOp::Delete { row: 2 },
            ],
        )
        .expect("write ops");
    assert_eq!((report.updated, report.deleted), (1, 1));
    let batch = engine
        .read_rows("Customers", RowRange::at(0, 10))
        .expect("read back");
    assert_eq!(batch.rows.len(), 2);
    assert_eq!(batch.rows[0][1], StorageValue::Text("Adalovelace".into()));
}

#[test]
fn flush_is_idempotent_on_the_mock() {
    let mut engine = engine_with_customers();
    engine.flush().expect("flush once");
    engine.flush().expect("flush twice");
}

#[test]
fn error_display_is_precise_and_locatable() {
    let corrupt = StorageError::Corrupt {
        context: "page header".into(),
        detail: "magic bytes mismatch at offset 0".into(),
    };
    let display = corrupt.to_string();
    assert!(display.contains("page header"), "context in display: {display}");
    assert!(
        display.contains("offset 0"),
        "detail in display: {display}"
    );
    let missing = StorageError::TableNotFound("Foo".into()).to_string();
    assert!(missing.contains("Foo"), "table name in display: {missing}");
}

#[test]
fn trait_is_object_safe_and_dispatches_dynamically() {
    let mut engine: Box<dyn StorageEngine> = Box::new(engine_with_customers());
    let meta = engine.open(Path::new("")).expect("open via trait object");
    assert_eq!(meta.format, FormatVersion::Jet4);
    let batch = engine
        .read_rows("Customers", RowRange::at(0, 1))
        .expect("read via trait object");
    assert_eq!(batch.rows.len(), 1);
}
