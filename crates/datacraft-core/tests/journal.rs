//! Journaled write-profile contract (step 11.11, item M2).
//!
//! Writes are previewed as a diff, applied only after the caller confirms
//! the token, and come with the inverse operations for a rollback. The
//! profile restricts the batch to qualified field families; anything else
//! is a precise `Unsupported`-style refusal. The whole flow is reachable
//! over JSON-RPC.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::{ChangeKind, WriteOp, write_profile};
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_core::transport;

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

fn seeded_rows() -> Vec<Vec<FieldValue>> {
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
    ]
}

fn core_with_rows() -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
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
        WriteOp::Insert {
            values: vec![
                FieldValue::Null,
                FieldValue::Text("Linus".into()),
                FieldValue::Float(3.5),
            ],
        },
    ]
}

fn preview_token(events: &[Event]) -> u64 {
    match events.first() {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    }
}

#[test]
fn preview_reports_the_diff_without_touching_rows() {
    let mut core = core_with_rows();
    let events = core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    });
    let token = preview_token(&events);
    let Event::WritePreviewed { changes, .. } = &events[0] else {
        panic!("{events:?}");
    };
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].kind, ChangeKind::Update);
    assert_eq!(changes[0].row, Some(0));
    assert_eq!(
        changes[0].before.as_ref().expect("before")[1],
        FieldValue::Text("Ada".into())
    );
    assert_eq!(
        changes[0].after.as_ref().expect("after")[1],
        FieldValue::Text("Ada L.".into())
    );
    assert_eq!(changes[1].kind, ChangeKind::Delete);
    assert_eq!(
        changes[1].before.as_ref().expect("before")[1],
        FieldValue::Text("Grace".into())
    );
    assert!(changes[1].after.is_none());
    assert_eq!(changes[2].kind, ChangeKind::Insert);
    assert_eq!(
        changes[2].row,
        Some(1),
        "insert lands at the evolving index"
    );
    assert!(changes[2].before.is_none());

    assert_eq!(rows(&core), seeded_rows(), "preview is read-only");
    assert!(token >= 1);
}

#[test]
fn apply_writes_the_batch_and_reports_rollback_information() {
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    let events = core.dispatch(Command::ApplyWrite { token });
    let Event::WriteApplied {
        inserted,
        updated,
        deleted,
        rollback,
        ..
    } = &events[0]
    else {
        panic!("{events:?}");
    };
    assert_eq!((*inserted, *updated, *deleted), (1, 1, 1));
    assert_eq!(rollback.len(), 3, "one inverse per applied operation");
    let after = rows(&core);
    assert_eq!(after.len(), 2);
    assert_eq!(after[0][1], FieldValue::Text("Ada L.".into()));
    assert_eq!(after[1][1], FieldValue::Text("Linus".into()));
}

#[test]
fn rollback_restores_the_rows() {
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    core.dispatch(Command::ApplyWrite { token });
    assert_eq!(
        core.dispatch(Command::RollbackWrite { token }),
        vec![Event::WriteRolledBack {
            table: "Customers".into(),
            token
        }]
    );
    assert_eq!(rows(&core), seeded_rows(), "values and order are restored");
}

#[test]
fn stale_tokens_and_missing_journals_are_rejected() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(Command::ApplyWrite { token: 7 }));
    assert!(reason.contains("no pending"), "{reason}");
    let reason = failure(&core.dispatch(Command::RollbackWrite { token: 7 }));
    assert!(reason.contains("no applied"), "{reason}");

    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    let reason = failure(&core.dispatch(Command::ApplyWrite { token: token + 99 }));
    assert!(reason.contains("stale"), "{reason}");
}

#[test]
fn discard_clears_the_pending_journal() {
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    assert_eq!(
        core.dispatch(Command::DiscardWrite { token }),
        vec![Event::WriteDiscarded { token }]
    );
    let reason = failure(&core.dispatch(Command::ApplyWrite { token }));
    assert!(reason.contains("no pending"), "{reason}");
    assert_eq!(rows(&core), seeded_rows());
}

#[test]
fn the_profile_rejects_unqualified_families() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "Docs",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("file", FieldKind::Binary(BinaryKind::Attachment)),
            ],
        ),
    });
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Docs".into(),
        ops: vec![WriteOp::Delete { row: 0 }],
    }));
    assert!(reason.contains("write profile"), "{reason}");
    assert!(reason.contains("attachment"), "{reason}");
    assert!(reason.contains("M4"), "{reason}");
}

#[test]
fn the_profile_reports_fixed_width_tables() {
    let fixed = TableDef::new(
        "Metrics",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    );
    assert!(write_profile(&fixed).expect("qualified").fixed_width);
    assert!(!write_profile(&customers()).expect("qualified").fixed_width);
}

#[test]
fn preview_validates_arity_bounds_and_autonumbers() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Insert {
            values: vec![FieldValue::Null, FieldValue::Text("x".into())],
        }],
    }));
    assert!(reason.contains("expected 3 values"), "{reason}");

    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Delete { row: 9 }],
    }));
    assert!(reason.contains("out of range"), "{reason}");

    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Update {
            row: 0,
            values: vec![
                FieldValue::Integer(99),
                FieldValue::Text("Ada".into()),
                FieldValue::Float(1.5),
            ],
        }],
    }));
    assert!(reason.to_lowercase().contains("autonumber"), "{reason}");
}

#[test]
fn a_changed_table_makes_the_preview_stale() {
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    // Another write lands between preview and apply.
    core.dispatch(Command::InsertRow {
        table: "Customers".into(),
        values: vec![
            FieldValue::Null,
            FieldValue::Text("Between".into()),
            FieldValue::Float(0.5),
        ],
    });
    let reason = failure(&core.dispatch(Command::ApplyWrite { token }));
    assert!(reason.contains("changed since the preview"), "{reason}");
}

#[test]
fn the_journal_round_trips_over_json_rpc() {
    let mut core = core_with_rows();
    let preview = Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    };
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "dispatch",
        "params": preview,
    });
    let response = transport::handle_json(&mut core, &request.to_string());
    let value: serde_json::Value = serde_json::from_str(&response).expect("response json");
    let events = value["result"]["events"].as_array().expect("events");
    let token = events[0]["WritePreviewed"]["token"]
        .as_u64()
        .expect("preview token");

    let apply = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "dispatch",
        "params": {"ApplyWrite": {"token": token}},
    });
    let response = transport::handle_json(&mut core, &apply.to_string());
    assert!(response.contains("WriteApplied"), "{response}");
    assert_eq!(rows(&core)[1][1], FieldValue::Text("Linus".into()));
}

#[test]
fn journal_commands_round_trip_through_json() {
    let command = Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    };
    let json = serde_json::to_string(&command).expect("serialize command");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(back, command);
}

#[test]
fn the_profile_rejects_guid_autonumbers() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(
            "Ids",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::ReplicationId)),
                FieldDef::new("name", FieldKind::Text { max_len: 10 }),
            ],
        ),
    });
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Ids".into(),
        ops: vec![],
    }));
    assert!(reason.contains("GUID AutoNumber"), "{reason}");
    assert!(reason.contains("M8"), "{reason}");
}

#[test]
fn preview_errors_cover_missing_tables_updates_and_reads() {
    let mut bare = Core::new(Box::new(MockPort::new()));
    let reason = failure(&bare.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![],
    }));
    assert!(reason.contains("no database"), "{reason}");
    bare.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    let reason = failure(&bare.dispatch(Command::PreviewWrite {
        table: "Ghost".into(),
        ops: vec![],
    }));
    assert!(reason.contains("not found"), "{reason}");

    let mut core = core_with_rows();
    let values = vec![
        FieldValue::Integer(1),
        FieldValue::Text("Ada".into()),
        FieldValue::Float(1.5),
    ];
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Update {
            row: 9,
            values: values.clone(),
        }],
    }));
    assert!(reason.contains("out of range"), "{reason}");
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Update {
            row: 0,
            values: vec![FieldValue::Null],
        }],
    }));
    assert!(reason.contains("expected 3 values"), "{reason}");

    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows())
        .with_read_failure("rows unreadable");
    let mut broken = Core::new(Box::new(port));
    broken.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    let reason = failure(&broken.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![],
    }));
    assert!(reason.contains("rows unreadable"), "{reason}");
}

#[test]
fn apply_and_discard_error_paths_are_precise() {
    // The re-validation at apply time surfaces a stale preview.
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Delete { row: 1 }],
    }));
    core.dispatch(Command::DeleteRow {
        table: "Customers".into(),
        row: 1,
    });
    let reason = failure(&core.dispatch(Command::ApplyWrite { token }));
    assert!(reason.contains("changed since the preview"), "{reason}");

    // A table dropped between preview and apply is named.
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    core.dispatch(Command::DropTable {
        table: "Customers".into(),
    });
    let reason = failure(&core.dispatch(Command::ApplyWrite { token }));
    assert!(reason.contains("not found"), "{reason}");

    // A write failure at apply time surfaces the backend reason.
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows())
        .with_write_failure("disk full");
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: vec![WriteOp::Delete { row: 0 }],
    }));
    let reason = failure(&core.dispatch(Command::ApplyWrite { token }));
    assert!(reason.contains("disk full"), "{reason}");

    // Discard with a stale token, then with the pending one, then without
    // any pending journal (the failed apply above kept its journal).
    let reason = failure(&core.dispatch(Command::DiscardWrite { token: token + 5 }));
    assert!(reason.contains("stale"), "{reason}");
    assert_eq!(
        core.dispatch(Command::DiscardWrite { token }),
        vec![Event::WriteDiscarded { token }]
    );
    let reason = failure(&core.dispatch(Command::DiscardWrite { token }));
    assert!(reason.contains("no pending"), "{reason}");

    // Rollback with a stale token, then the real one.
    let mut core = core_with_rows();
    let token = preview_token(&core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: batch(),
    }));
    core.dispatch(Command::ApplyWrite { token });
    let reason = failure(&core.dispatch(Command::RollbackWrite { token: token + 1 }));
    assert!(reason.contains("stale rollback"), "{reason}");
    assert!(matches!(
        core.dispatch(Command::RollbackWrite { token })[0],
        Event::WriteRolledBack { .. }
    ));
}
