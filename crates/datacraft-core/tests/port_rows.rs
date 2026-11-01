//! Contract suite for row reads through the port (step 6.2, item H23).
//!
//! Locks the domain row vocabulary: `StoragePort::read_rows` windows rows
//! as `FieldValue` vectors, mirrors the engine boundary's window semantics
//! (empty window at `row_count`, precise error beyond), and `Core` exposes
//! it as a passthrough for the UI's virtualized datasheet.

use std::path::{Path, PathBuf};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::{MockPort, PortError, RowRange, StoragePort as _};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn table(name: &str) -> TableDef {
    TableDef::new(
        name,
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn two_column_row(a: i64, b: &str) -> Vec<FieldValue> {
    vec![FieldValue::Integer(a), FieldValue::Text(b.to_owned())]
}

fn seeded_core() -> Core {
    let mut port = MockPort::new();
    port.with_row_data(
        "Customers",
        vec![
            two_column_row(1, "Ada"),
            two_column_row(2, "Brahe"),
            two_column_row(3, "Curie"),
        ],
    );
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateTable {
        definition: table("Customers"),
    });
    core
}

#[test]
fn read_rows_windows_rows_positionally() {
    let core = seeded_core();
    let rows = core
        .read_rows("Customers", RowRange { start: 1, len: 2 })
        .expect("windowed read");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], two_column_row(2, "Brahe"));
    assert_eq!(rows[1], two_column_row(3, "Curie"));
}

#[test]
fn a_window_starting_at_row_count_is_empty() {
    let core = seeded_core();
    let rows = core
        .read_rows("Customers", RowRange { start: 3, len: 5 })
        .expect("empty tail window");
    assert!(rows.is_empty());
}

#[test]
fn a_window_beyond_row_count_is_a_precise_error() {
    let core = seeded_core();
    let err = core
        .read_rows("Customers", RowRange { start: 4, len: 2 })
        .expect_err("beyond the row set");
    let msg = err.to_string();
    assert!(msg.contains("4"), "names the start: {msg}");
    assert!(
        msg.to_lowercase().contains("range"),
        "names the problem: {msg}"
    );
    assert!(matches!(err, PortError::OutOfRange { .. }));
}

#[test]
fn unknown_tables_are_table_not_found() {
    let core = seeded_core();
    let err = core
        .read_rows("Ghost", RowRange { start: 0, len: 1 })
        .expect_err("unknown table");
    assert!(matches!(err, PortError::TableNotFound(_)), "{err}");
}

#[test]
fn reads_work_without_an_open_database_fail_precisely() {
    let core = Core::new(Box::new(MockPort::new()));
    let err = core
        .read_rows("Customers", RowRange { start: 0, len: 1 })
        .expect_err("no database");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err}");
}

#[test]
fn null_and_typed_values_pass_through_unchanged() {
    let mut port = MockPort::with_tables(vec![table("T")]);
    port.with_row_data(
        "T",
        vec![vec![
            FieldValue::Null,
            FieldValue::Boolean(true),
            FieldValue::Float(1.5),
            FieldValue::Currency(-12_345),
            FieldValue::DateTime(45_000.5),
            FieldValue::Decimal(-9),
            FieldValue::Binary(vec![1, 2]),
            FieldValue::Guid([9u8; 16]),
        ]],
    );
    port.open(Path::new("m.mdb"), None).expect("mock opens");
    let rows = port
        .read_rows("T", RowRange { start: 0, len: 10 })
        .expect("read");
    assert_eq!(rows[0][0], FieldValue::Null);
    assert_eq!(rows[0][1], FieldValue::Boolean(true));
    assert_eq!(rows[0][7], FieldValue::Guid([9u8; 16]));
}
