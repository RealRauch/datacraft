//! Colocated mock tests (moved from `port_mock.rs` for the size gate).

use super::*;
use crate::schema::{FieldDef, FieldKind};

fn table() -> TableDef {
    TableDef::new(
        "T",
        vec![FieldDef::new(
            "n",
            FieldKind::Number(crate::schema::NumericKind::LongInteger),
        )],
    )
}

/// Every write path reports a configured failure before touching state.
#[test]
fn mock_write_failures_cover_all_write_operations() {
    let mut port = MockPort::failing_writes("disk full");
    assert!(matches!(
        port.create_table(&table()),
        Err(PortError::WriteFailed { .. })
    ));
    assert!(matches!(
        port.drop_table("T"),
        Err(PortError::WriteFailed { .. })
    ));
    assert!(matches!(
        port.insert_row("T", &[FieldValue::Integer(1)]),
        Err(PortError::WriteFailed { .. })
    ));
    assert!(matches!(
        port.update_row("T", 0, &[FieldValue::Integer(1)]),
        Err(PortError::WriteFailed { .. })
    ));
    assert!(matches!(
        port.delete_row("T", 0),
        Err(PortError::WriteFailed { .. })
    ));
}

/// The mock applies row operations by live-row index and reports
/// precise bounds and arity failures.
#[test]
fn mock_row_operations_cover_bounds_and_arity() {
    let mut port = MockPort::with_tables(vec![table()]);
    port.with_row_data("T", vec![vec![FieldValue::Integer(1)]]);
    port.update_row("t", 0, &[FieldValue::Integer(2)])
        .expect("case-insensitive update");
    port.open(std::path::Path::new("memory.mock"), None)
        .expect("open the mock");
    assert_eq!(
        port.read_rows("T", RowRange { start: 0, len: 1 })
            .expect("row stored"),
        vec![vec![FieldValue::Integer(2)]]
    );

    assert!(matches!(
        port.update_row("T", 0, &[]),
        Err(PortError::WriteFailed { .. })
    ));
    assert!(matches!(
        port.update_row("T", 3, &[FieldValue::Integer(3)]),
        Err(PortError::OutOfRange { .. })
    ));
    assert!(matches!(
        port.delete_row("T", 3),
        Err(PortError::OutOfRange { .. })
    ));
    port.delete_row("T", 0).expect("delete the only row");
    assert!(
        port.read_rows("T", RowRange { start: 0, len: 1 })
            .expect("empty table reads")
            .is_empty()
    );

    // A table without a row entry behaves like an empty table.
    let mut empty = MockPort::with_tables(vec![table()]);
    assert!(matches!(
        empty.update_row("T", 0, &[FieldValue::Integer(1)]),
        Err(PortError::OutOfRange { .. })
    ));
    assert!(matches!(
        empty.delete_row("T", 0),
        Err(PortError::OutOfRange { .. })
    ));
}

/// The mock insert validates the table and the value arity itself.
#[test]
fn mock_insert_row_checks_table_and_arity() {
    let mut port = MockPort::with_tables(vec![table()]);
    assert!(matches!(
        port.insert_row("missing", &[]),
        Err(PortError::TableNotFound(_))
    ));
    match port.insert_row("t", &[]) {
        Err(PortError::WriteFailed { detail }) => {
            assert!(detail.contains("expected 1 values"), "{detail}");
        }
        other => panic!("expected WriteFailed, got {other:?}"),
    }
    port.insert_row("t", &[FieldValue::Integer(1)])
        .expect("valid insert");
    port.open(std::path::Path::new("memory.mock"), None)
        .expect("open the mock");
    assert_eq!(
        port.read_rows("T", RowRange { start: 0, len: 1 })
            .expect("row is stored"),
        vec![vec![FieldValue::Integer(1)]]
    );
}
