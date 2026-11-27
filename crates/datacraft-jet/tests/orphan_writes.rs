//! Jet orphan writes refuse against declared relationships (phase 104,
//! step 104.2; item M137; roadmap §13(a)). SQLite's engine refuses
//! orphans; the Jet path carried `MSysRelationships` metadata only — an
//! orphan insert succeeded there where SQLite failed. The writer now
//! checks every insert/update (and every batch operation, before a
//! single byte is written) against the parent tables. Deletes stay
//! allowed — §13(b) cascades are a separate, later decision.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{Relationship, RowRange, RowWriteOp, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-jet-orphan-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn parent_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

fn child_table() -> TableDef {
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

/// A fresh database with both tables, one parent row (`Id` 1) and the
/// declared relationship.
fn opened() -> (Scratch, Jet4Port) {
    let file = Scratch(scratch_path("db"));
    let mut port = Jet4Port::new();
    port.create_database(&file.0, datacraft_core::port::DatabaseFormat::Jet4)
        .expect("create the scratch database");
    port.open(&file.0, None).expect("open the scratch database");
    port.create_table(&parent_table())
        .expect("create the parent table");
    port.create_table(&child_table())
        .expect("create the child table");
    port.insert_row("Customers", &[FieldValue::Integer(1)])
        .expect("seed the parent row");
    port.create_relationship(&relationship())
        .expect("declare the relationship");
    (file, port)
}

#[test]
fn an_orphan_insert_names_the_parent_and_the_value() {
    let (_file, mut port) = opened();
    let error = port
        .insert_row(
            "Orders",
            &[FieldValue::Integer(99), FieldValue::Text("x".into())],
        )
        .expect_err("the orphan insert must fail");
    let detail = error.to_string();
    assert!(detail.contains("Customers"), "{detail}");
    assert!(detail.contains("99"), "{detail}");
    assert!(detail.contains("CustomerId"), "{detail}");
    // Nothing was written.
    let rows = port
        .read_rows("Orders", RowRange { start: 0, len: 100 })
        .expect("read the child table");
    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn a_valid_child_insert_writes() {
    let (_file, mut port) = opened();
    port.insert_row(
        "Orders",
        &[FieldValue::Integer(1), FieldValue::Text("x".into())],
    )
    .expect("the referenced value writes");
    let rows = port
        .read_rows("Orders", RowRange { start: 0, len: 100 })
        .expect("read the child table");
    assert_eq!(rows.len(), 1);
}

#[test]
fn a_null_child_value_passes_sql_fk_semantics() {
    let (_file, mut port) = opened();
    port.insert_row("Orders", &[FieldValue::Null, FieldValue::Text("x".into())])
        .expect("a NULL child value passes");
}

#[test]
fn an_orphan_update_refuses_and_keeps_the_old_value() {
    let (_file, mut port) = opened();
    port.insert_row(
        "Orders",
        &[FieldValue::Integer(1), FieldValue::Text("x".into())],
    )
    .expect("seed the child row");
    let error = port
        .update_row(
            "Orders",
            0,
            &[FieldValue::Integer(99), FieldValue::Text("y".into())],
        )
        .expect_err("the orphan update must fail");
    assert!(error.to_string().contains("99"), "{error}");
    let rows = port
        .read_rows("Orders", RowRange { start: 0, len: 100 })
        .expect("read the child table");
    assert_eq!(rows[0][0], FieldValue::Integer(1), "{rows:?}");
}

#[test]
fn a_batch_with_an_orphan_refuses_before_any_byte_is_written() {
    let (_file, mut port) = opened();
    let error = port
        .write_row_ops(
            "Orders",
            &[
                RowWriteOp::Insert(vec![FieldValue::Integer(1), FieldValue::Text("x".into())]),
                RowWriteOp::Insert(vec![FieldValue::Integer(99), FieldValue::Text("y".into())]),
            ],
        )
        .expect_err("the batch must refuse the orphan");
    assert!(error.to_string().contains("99"), "{error}");
    // The valid first operation never landed.
    let rows = port
        .read_rows("Orders", RowRange { start: 0, len: 100 })
        .expect("read the child table");
    assert!(rows.is_empty(), "no partial application: {rows:?}");
}

#[test]
fn deleting_a_parent_stays_allowed_documented_13b() {
    let (_file, mut port) = opened();
    port.delete_row("Customers", 0)
        .expect("deletes stay allowed — §13(b) cascades are a separate step");
}
