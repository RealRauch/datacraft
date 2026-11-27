//! Referential write parity (phase 104, step 104.2; item M137; roadmap
//! §13(a)): orphan inserts and updates against declared relationships
//! refuse on Jet exactly where SQLite's engine refuses them, and a
//! journal batch containing an orphan is refused in preview — the batch
//! never applies partially. The rule lives in one place (core) so the
//! port and the preview share it.

use std::collections::BTreeMap;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, Relationship};
use datacraft_core::referential::{orphan_reason, parent_rows_needed};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn customers_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger))],
    )
}

fn orders_table() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new(
                "CustomerId",
                FieldKind::Number(NumericKind::LongInteger),
            ),
            FieldDef::new("Note", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn relationship() -> Relationship {
    Relationship {
        table: "Orders".to_owned(),
        column: "CustomerId".to_owned(),
        related_table: "Customers".to_owned(),
        related_column: "Id".to_owned(),
    }
}

fn parents() -> BTreeMap<String, Vec<Vec<FieldValue>>> {
    BTreeMap::from([(
        "customers".to_owned(),
        vec![vec![FieldValue::Integer(1)]],
    )])
}

#[test]
fn parent_rows_needed_names_the_referenced_tables_once() {
    let needed = parent_rows_needed(&[relationship()], "Orders");
    assert_eq!(needed, ["Customers"]);
    // A relationship of another child table does not concern this check.
    let needed = parent_rows_needed(&[relationship()], "Customers");
    assert!(needed.is_empty());
}

#[test]
fn an_orphan_child_value_names_the_parent_and_the_value() {
    let reason = orphan_reason(
        &[relationship()],
        &orders_table(),
        &[FieldValue::Integer(99), FieldValue::Text("x".into())],
        &parents(),
    )
    .expect("the orphan is refused");
    assert!(reason.contains("Customers"), "{reason}");
    assert!(reason.contains("99"), "{reason}");
    assert!(reason.contains("CustomerId"), "{reason}");
}

#[test]
fn a_valid_child_value_and_a_null_pass() {
    assert!(
        orphan_reason(
            &[relationship()],
            &orders_table(),
            &[FieldValue::Integer(1), FieldValue::Text("x".into())],
            &parents(),
        )
        .is_none(),
        "a referenced value passes"
    );
    assert!(
        orphan_reason(
            &[relationship()],
            &orders_table(),
            &[FieldValue::Null, FieldValue::Text("x".into())],
            &parents(),
        )
        .is_none(),
        "a NULL child value passes (SQL FK semantics)"
    );
}

#[test]
fn the_preview_refuses_a_batch_with_an_orphan() {
    let mut port = MockPort::with_tables(vec![customers_table(), orders_table()]);
    port.with_relationships(vec![relationship()]);
    port.with_row_data("Customers", vec![vec![FieldValue::Integer(1)]]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "orders.mock".into(),
        password: None,
        source: None,
    });

    let events = core.dispatch(Command::PreviewWrite {
        table: "Orders".into(),
        ops: vec![datacraft_core::journal::WriteOp::Insert {
            values: vec![FieldValue::Integer(99), FieldValue::Text("x".into())],
        }],
    });
    assert!(
        matches!(&events[0], Event::Failed { reason, .. }
            if reason.contains("Customers") && reason.contains("99")),
        "{events:?}"
    );
    // Nothing is pending — the batch never reached the journal.
    let events = core.dispatch(Command::ApplyWrite { token: 1 });
    assert!(
        matches!(&events[0], Event::Failed { reason, .. }
            if reason.contains("no pending")),
        "{events:?}"
    );
}
