//! Generates the manual Access-repair-gate artifact (BACKLOG H9 acceptance).
//!
//! Copies the pristine pinned fixture and extends it with a fresh table of
//! every writer-qualified core type through the **same port path the
//! application uses** (create table + insert rows, step 11.1). The result
//! lands in `access-gate/GateCheck.mdb` — open it in real Microsoft Access
//! and follow `access-gate/CHECKLIST.md`. Run from the workspace root:
//!
//! ```text
//! cargo run -p datacraft-storage --example access_gate
//! ```

use datacraft_core::port::StoragePort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn gate_table() -> TableDef {
    TableDef::new(
        "GateCheck",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 60 }),
            FieldDef::new("Score", FieldKind::Number(NumericKind::Double)),
            FieldDef::new("Active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("Note", FieldKind::Memo),
        ],
    )
}

fn main() {
    let source = std::path::Path::new("fixtures/testV2000.mdb");
    let out_dir = std::path::Path::new("access-gate");
    std::fs::create_dir_all(out_dir).expect("creates the output directory");
    let target = out_dir.join("GateCheck.mdb");
    std::fs::copy(source, &target).expect("copies the pinned fixture");

    let mut port = Jet4Port::new();
    port.open(&target, None).expect("opens the fixture copy");
    let table = gate_table();
    port.create_table(&table)
        .expect("creates the GateCheck table");

    let rows: Vec<(i64, &str, f64, bool, Option<&str>)> = vec![
        (1, "alpha", 1.5, true, Some("first row")),
        (2, "beta", -0.25, false, None),
        (3, "gamma", 42.0, true, Some("mit Umlauten: äöüß")),
    ];
    for (id, name, score, active, note) in rows {
        port.insert_row(
            "GateCheck",
            &[
                FieldValue::Integer(id),
                FieldValue::Text(name.to_owned()),
                FieldValue::Float(score),
                FieldValue::Boolean(active),
                match note {
                    Some(text) => FieldValue::Text(text.to_owned()),
                    None => FieldValue::Null,
                },
            ],
        )
        .expect("inserts");
    }
    println!(
        "wrote {} — open it in Access and follow access-gate/CHECKLIST.md",
        target.display()
    );
}
