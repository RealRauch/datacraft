//! Datasheet edit-buffer vocabulary (step 85.1, item H61).
//!
//! The datasheet's per-tab edit buffer stages updates, inserts and
//! deletes against a stable slot list and serializes them onto the
//! journal's `WriteOp` batch — the evolving-index contract of
//! [`datacraft_core::journal`]. Nothing here touches the port; the buffer
//! is pure state the UI owns per tab and the journal applies as one
//! atomic batch.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::edit_buffer::{EditBuffer, RowState};
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

fn row(id: i64, name: &str, score: f64) -> Vec<FieldValue> {
    vec![
        FieldValue::Integer(id),
        FieldValue::Text(name.into()),
        FieldValue::Float(score),
    ]
}

fn seeded_rows() -> Vec<Vec<FieldValue>> {
    vec![row(1, "Ada", 1.5), row(2, "Grace", 2.5), row(3, "Linus", 3.5)]
}

fn buffer() -> EditBuffer {
    EditBuffer::new(seeded_rows())
}

fn update(row: usize, name: &str) -> Vec<FieldValue> {
    let mut values = seeded_rows()[row].clone();
    values[1] = FieldValue::Text(name.into());
    values
}

fn insert(name: &str) -> Vec<FieldValue> {
    vec![FieldValue::Null, FieldValue::Text(name.into()), FieldValue::Float(9.9)]
}

fn failure(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn core_with_rows() -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows());
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
        ..DriverCapabilities::full()
    });
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    core
}

#[test]
fn a_fresh_buffer_holds_the_rows_without_pending_state() {
    let buffer = buffer();
    assert_eq!(buffer.row_count(), 3);
    assert!(buffer.is_empty(), "no staged edits yet");
    assert_eq!(buffer.row_state(0), RowState::Unchanged);
    assert_eq!(
        buffer.row_image(0).expect("row image"),
        seeded_rows()[0].as_slice()
    );
}

#[test]
fn staged_edits_serialize_to_the_journal_batch() {
    let mut buffer = buffer();
    buffer.stage_update(0, update(0, "Ada L.")).expect("update");
    buffer.stage_delete(1).expect("delete");
    buffer.stage_insert(insert("New").clone());
    assert!(!buffer.is_empty());
    assert_eq!(buffer.row_state(0), RowState::Updated);
    assert_eq!(buffer.row_state(1), RowState::Deleted);
    assert_eq!(buffer.row_state(2), RowState::Unchanged);
    assert_eq!(buffer.row_state(3), RowState::Inserted);
    assert_eq!(
        buffer.ops(),
        vec![
            WriteOp::Update {
                row: 0,
                values: update(0, "Ada L.")
            },
            WriteOp::Delete { row: 1 },
            WriteOp::Insert {
                values: insert("New")
            },
        ]
    );
}

#[test]
fn a_delete_shifts_following_updates_to_evolving_indices() {
    let mut buffer = buffer();
    buffer.stage_delete(0).expect("delete row 0");
    buffer.stage_update(2, update(2, "Linus T.")).expect("update");
    assert_eq!(
        buffer.ops(),
        vec![
            WriteOp::Delete { row: 0 },
            WriteOp::Update {
                row: 0,
                values: update(2, "Linus T.")
            },
        ],
        "after the delete, original row 2 is evolving index 0"
    );
}

#[test]
fn re_editing_a_row_merges_into_one_pending_update() {
    let mut buffer = buffer();
    buffer.stage_update(0, update(0, "Ada M.")).expect("first");
    buffer.stage_update(0, update(0, "Ada L.")).expect("second");
    assert_eq!(
        buffer.ops(),
        vec![WriteOp::Update {
            row: 0,
            values: update(0, "Ada L.")
        }],
        "one update op per row, latest values win"
    );
    assert_eq!(buffer.changed_cells(0), vec![1], "only the name column");
    // The unedited columns keep the row's original values.
    assert_eq!(
        buffer.row_image(0).expect("image")[2],
        FieldValue::Float(1.5)
    );
}

#[test]
fn editing_a_deleted_row_is_refused_precisely() {
    let mut buffer = buffer();
    buffer.stage_delete(1).expect("delete");
    let reason = buffer.stage_update(1, update(1, "x")).unwrap_err();
    assert!(reason.contains("pending delete"), "{reason}");
    let reason = buffer.stage_delete(1).unwrap_err();
    assert!(reason.contains("pending delete"), "{reason}");
    assert_eq!(
        buffer.row_image(1).expect("deleted rows stay visible")[1],
        FieldValue::Text("Grace".into()),
        "the delete marker keeps the row image"
    );
}

#[test]
fn out_of_range_rows_are_refused() {
    let mut buffer = buffer();
    let reason = buffer.stage_update(3, update(0, "x")).unwrap_err();
    assert!(reason.contains("out of range"), "{reason}");
    let reason = buffer.stage_delete(7).unwrap_err();
    assert!(reason.contains("out of range"), "{reason}");
}

#[test]
fn escape_discards_the_active_record() {
    let mut buffer = buffer();
    buffer.stage_update(0, update(0, "Ada L.")).expect("update");
    buffer.stage_delete(1).expect("delete");
    buffer.stage_insert(insert("New"));
    buffer.discard_row(0).expect("discard update");
    buffer.discard_row(1).expect("discard delete");
    buffer.discard_row(3).expect("discard insert");
    assert!(buffer.is_empty(), "every staged edit reverted");
    assert_eq!(buffer.row_state(0), RowState::Unchanged);
    assert_eq!(buffer.row_state(1), RowState::Unchanged);
    assert_eq!(
        buffer.row_image(0).expect("image"),
        seeded_rows()[0].as_slice(),
        "the original values are restored"
    );
    assert_eq!(buffer.ops(), vec![]);
}

#[test]
fn discarding_an_insert_keeps_the_indices_stable() {
    let mut buffer = buffer();
    buffer.stage_insert(insert("A"));
    buffer.stage_insert(insert("B"));
    buffer.discard_row(3).expect("discard first insert");
    assert_eq!(buffer.row_state(3), RowState::Void, "the slot stays");
    assert_eq!(buffer.row_state(4), RowState::Inserted);
    assert_eq!(buffer.row_count(), 5, "the slot list never shifts");
    assert_eq!(
        buffer.ops(),
        vec![WriteOp::Insert {
            values: insert("B")
        }],
        "the surviving insert lands at the evolving end"
    );
    assert!(buffer.row_image(3).is_none(), "a void slot shows nothing");
}

#[test]
fn the_buffer_applies_as_one_journal_batch() {
    let mut core = core_with_rows();
    let mut buffer = buffer();
    buffer.stage_update(0, update(0, "Ada L.")).expect("update");
    buffer.stage_delete(1).expect("delete");
    buffer.stage_insert(insert("New"));
    let token = match core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: buffer.ops(),
    })
    .first()
    {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    };
    core.dispatch(Command::ApplyWrite { token });
    let after = core
        .read_rows("Customers", RowRange { start: 0, len: 100 })
        .expect("read rows");
    assert_eq!(after.len(), 3);
    assert_eq!(after[0][1], FieldValue::Text("Ada L.".into()));
    assert_eq!(after[1][1], FieldValue::Text("New".into()));
    assert_eq!(after[2][1], FieldValue::Text("Linus".into()));
}

#[test]
fn the_snapshot_reports_the_pending_journal_summary() {
    let mut core = core_with_rows();
    assert!(core.inspect().pending_write.is_none());
    let mut buffer = buffer();
    buffer.stage_update(0, update(0, "Ada L.")).expect("update");
    buffer.stage_delete(1).expect("delete");
    buffer.stage_insert(insert("New"));
    let token = match core.dispatch(Command::PreviewWrite {
        table: "Customers".into(),
        ops: buffer.ops(),
    })
    .first()
    {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    };
    let summary = core.inspect().pending_write.expect("pending summary");
    assert_eq!(summary.table, "Customers");
    assert_eq!(summary.token, token);
    assert_eq!((summary.inserts, summary.updates, summary.deletes), (1, 1, 1));
    core.dispatch(Command::DiscardWrite { token });
    assert!(core.inspect().pending_write.is_none(), "cleared on discard");
}

#[test]
fn a_journal_for_another_table_refuses_a_new_preview() {
    let mut port = MockPort::with_tables(vec![
        customers(),
        TableDef::new(
            "Orders",
            vec![
                FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("total", FieldKind::Number(NumericKind::Double)),
            ],
        ),
    ]);
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
        ..DriverCapabilities::full()
    });
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    let token = match core
        .dispatch(Command::PreviewWrite {
            table: "Customers".into(),
            ops: vec![WriteOp::Delete { row: 0 }],
        })
        .first()
    {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    };
    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "Orders".into(),
        ops: vec![WriteOp::Delete { row: 0 }],
    }));
    assert!(reason.contains("pending"), "{reason}");
    assert!(reason.contains("Customers"), "{reason}");
    // The same table re-previews (the buffer flow refreshes its batch).
    let refreshed = match core
        .dispatch(Command::PreviewWrite {
            table: "Customers".into(),
            ops: vec![WriteOp::Delete { row: 0 }],
        })
        .first()
    {
        Some(Event::WritePreviewed { token, .. }) => *token,
        other => panic!("expected WritePreviewed, got {other:?}"),
    };
    assert!(refreshed > token, "a fresh token for the refreshed batch");
}
