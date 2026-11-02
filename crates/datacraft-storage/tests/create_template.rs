//! Template materialization contracts (phase 29, step 29.2, item H44).
//!
//! `materialize_jet4_template` writes a template's tables and rows into a
//! brand-new file with every timestamp pinned to the `created` argument —
//! same inputs produce byte-identical files (LESSONS #6), which is what
//! the sample generator's `--check` (step 29.3) relies on. RED state: the
//! function does not exist yet — the suite fails to compile until it
//! lands.

use std::path::PathBuf;
use std::process::id;

use datacraft_core::port::StoragePort;
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_storage::jet::create::materialize_jet4_template;
use datacraft_storage::port_adapter::Jet4Port;

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("datacraft-storage-template-{}-{name}", id()))
}

/// A template with an autonumbered table and a plain table.
fn inventory() -> Vec<(TableDef, Vec<Vec<FieldValue>>)> {
    let mut item = TableDef::new(
        "Items",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("stock", FieldKind::Number(NumericKind::LongInteger)),
        ],
    );
    item.fields[2].properties.required = true;
    let customers = TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 60 })],
    );
    let rows = vec![
        vec![
            FieldValue::Null,
            FieldValue::Text("Road frame".into()),
            FieldValue::Integer(12),
        ],
        vec![
            FieldValue::Null,
            FieldValue::Text("Saddle".into()),
            FieldValue::Integer(40),
        ],
    ];
    let customer_rows = vec![
        vec![FieldValue::Text("Ada".into())],
        vec![FieldValue::Text("Grace".into())],
        vec![FieldValue::Text("Edsger".into())],
    ];
    vec![(item, rows), (customers, customer_rows)]
}

#[test]
fn materialization_is_deterministic_and_reopens_with_every_row() {
    let first = scratch("det-a.mdb");
    let second = scratch("det-b.mdb");
    let tables = inventory();

    materialize_jet4_template(&first, 46_000.0, &tables).expect("materialize a");
    materialize_jet4_template(&second, 46_000.0, &tables).expect("materialize b");
    let a = std::fs::read(&first).expect("a readable");
    let b = std::fs::read(&second).expect("b readable");
    assert_eq!(a, b, "same template and date, byte-identical file");

    let mut port = Jet4Port::new();
    port.open(&first, None)
        .expect("the materialized file opens");
    let names: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
    assert_eq!(
        names,
        ["Items", "Customers"],
        "catalog order is template order"
    );

    let items = port
        .read_rows(
            "Items",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("items read");
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0][0],
        FieldValue::Integer(1),
        "autonumbers start at 1"
    );
    assert_eq!(items[1][0], FieldValue::Integer(2));
    assert_eq!(items[0][1], FieldValue::Text("Road frame".into()));

    let customers = port
        .read_rows(
            "Customers",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("customers read");
    assert_eq!(customers.len(), 3);
    assert_eq!(customers[2][0], FieldValue::Text("Edsger".into()));
    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

#[test]
fn a_failed_materialization_leaves_no_file() {
    let path = scratch("broken.mdb");
    let broken = vec![(
        TableDef::new(
            "Attach",
            vec![FieldDef::new(
                "data",
                FieldKind::Binary(datacraft_core::schema::BinaryKind::Attachment),
            )],
        ),
        Vec::new(),
    )];
    let error =
        materialize_jet4_template(&path, 46_000.0, &broken).expect_err("attachments are readonly");
    assert!(
        error.to_string().to_lowercase().contains("complex"),
        "{error}"
    );
    assert!(!path.exists(), "the failed materialization cleaned up");
}
