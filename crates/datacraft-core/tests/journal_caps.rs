//! Journal batch capability gate (step 33.1, item M20) — split from the
//! journal suite so both files stay below the size gate.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::WriteOp;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

fn core() -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("Ada".into()),
                FieldValue::Float(1.5),
            ],
            vec![
                FieldValue::Integer(2),
                FieldValue::Text("Grace".into()),
                FieldValue::Float(2.5),
            ],
        ],
    );
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    core
}

fn failure(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

#[test]
fn multi_op_previews_require_an_atomic_driver() {
    // The honest mock default reports atomic_batch = false (like SQLite).
    let mut core = core();
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".to_owned(),
        ops: vec![
            WriteOp::Update {
                row: 0,
                values: vec![
                    FieldValue::Null,
                    FieldValue::Text("Ada L.".into()),
                    FieldValue::Float(1.75),
                ],
            },
            WriteOp::Update {
                row: 1,
                values: vec![
                    FieldValue::Null,
                    FieldValue::Text("Grace L.".into()),
                    FieldValue::Float(2.75),
                ],
            },
        ],
    }));
    assert!(
        reason.contains("atomic") && reason.contains("one operation"),
        "{reason}"
    );

    let events = core.dispatch(Command::PreviewWrite {
        table: "Customers".to_owned(),
        ops: vec![WriteOp::Update {
            row: 0,
            values: vec![
                FieldValue::Null,
                FieldValue::Text("single".into()),
                FieldValue::Float(0.5),
            ],
        }],
    });
    assert!(
        matches!(events.first(), Some(Event::WritePreviewed { .. })),
        "{events:?}"
    );
}
