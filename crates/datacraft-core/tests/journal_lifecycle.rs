//! Journal lifetime across the database lifecycle (step 34.5, item H48).
//!
//! A previewed or applied journal belongs to the database it was computed
//! on; closing and reopening must drop it.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::WriteOp;
use datacraft_core::port::{DriverCapabilities, MockPort, RowRange};
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

fn core_with_rows() -> Core {
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
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
        ..DriverCapabilities::full()
    });
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    core
}

fn rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows("Customers", RowRange { start: 0, len: 100 })
        .expect("read rows")
}

fn failure(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn batch() -> Vec<WriteOp> {
    vec![
        WriteOp::Update {
            row: 0,
            values: vec![
                FieldValue::Null,
                FieldValue::Text("Ada L.".into()),
                FieldValue::Float(1.75),
            ],
        },
        WriteOp::Delete { row: 1 },
    ]
}

fn preview_token(events: &[Event]) -> u64 {
    match events.first() {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    }
}

/// H48 regression: a journal belongs to the database it was previewed on —
/// closing and reopening must drop it instead of letting a stale token
/// rewrite the next database.
#[test]
fn journals_do_not_survive_a_database_lifecycle() {
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    core.dispatch(Command::ApplyWrite { token });
    let applied = rows(&core);
    core.dispatch(Command::CloseDatabase);
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });

    let reason = failure(&core.dispatch(Command::RollbackWrite { token }));
    assert!(reason.contains("no applied"), "{reason}");
    assert_eq!(
        rows(&core),
        applied,
        "a stale rollback must not rewrite the reopened database"
    );

    let pending = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Insert {
            values: vec![
                FieldValue::Null,
                FieldValue::Text("Linus".into()),
                FieldValue::Float(3.5),
            ],
        }],
    }));
    core.dispatch(Command::CloseDatabase);
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    let reason = failure(&core.dispatch(Command::ApplyWrite { token: pending }));
    assert!(reason.contains("no pending"), "{reason}");
}
