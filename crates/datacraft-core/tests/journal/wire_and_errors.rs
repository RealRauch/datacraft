//! The journal's wire surface and precise error paths (split from
//! `journal.rs` to stay under the file-size gate; L30).

use super::*;

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
        password: None,
        source: None,
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
        password: None,
        source: None,
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
        password: None,
        source: None,
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
        password: None,
        source: None,
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
