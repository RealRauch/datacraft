//! ACE-over-the-wire contracts (phase 25, step 25.2, item H43).
//!
//! MCP and the CLI speak this identical JSON-RPC vocabulary: the suite
//! opens the pinned ACE fixtures and drives the full write lifecycle on a
//! private copy of `testV2010.accdb`, asserting the format stays "ace"
//! across every operation and every change survives a real reopen.

use std::process::id;

use datacraft_core::core::Core;
use datacraft_core::port::RowRange;
use datacraft_core::schema::FieldValue;
use datacraft_storage::detecting::DetectingPort;
use serde_json::json;

mod support;

use support::{dispatch, fixture_named, result_of, session};

#[test]
fn opens_an_ace_fixture_over_the_wire() {
    let ace = fixture_named("testV2007.accdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            dispatch(1, json!({"OpenDatabase":{"path": ace.to_string_lossy()}})),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
        ],
    );
    let events = &result_of(&responses[0])["events"];
    assert_eq!(events[0]["DatabaseOpened"]["format"], "ace", "{events}");
    let snapshot = &result_of(&responses[1]);
    assert_eq!(snapshot["database"]["format"], "ace", "{snapshot}");
    assert_eq!(
        snapshot["database"]["tables"][0]["name"], "Table1",
        "{snapshot}"
    );
}

/// The full write lifecycle over the wire: create, insert, reopen, drop —
/// on a private copy of the pinned ACE 2010 fixture.
#[test]
fn ace_writes_persist_over_the_wire_and_stay_ace() {
    let path = std::env::temp_dir().join(format!("datacraft-e2e-{}-ace-write.accdb", id()));
    std::fs::copy(fixture_named("testV2010.accdb"), &path).expect("copy the fixture");
    let path = path.to_string_lossy().to_string();

    // Session 1: open, create, insert — the snapshot still reports ACE.
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            dispatch(1, json!({"OpenDatabase":{"path": path}})),
            dispatch(
                2,
                json!({"CreateTable":{"definition":{
                "name":"Written",
                "fields":[{"name":"note","kind":{"type":"memo"},
                           "properties":{"caption":null,"input_mask":null,
                           "format":null,"default_value":null,
                           "validation_rule":null,"validation_text":null,
                           "required":false,"allow_zero_length":false,
                           "indexed":"none"}}]}}}),
            ),
            dispatch(
                3,
                json!({"InsertRow":{"table":"Written",
                "values":[{"Text":"over the wire"}]}}),
            ),
            json!({"jsonrpc":"2.0","id":4,"method":"inspect"}),
        ],
    );
    assert_eq!(
        result_of(&responses[0])["events"][0]["DatabaseOpened"]["format"],
        "ace",
        "{responses:?}"
    );
    assert_eq!(
        result_of(&responses[1])["events"][0]["TableCreated"]["table"],
        "Written",
        "{responses:?}"
    );
    assert_eq!(
        result_of(&responses[2])["events"][0]["RowInserted"]["table"],
        "Written",
        "{responses:?}"
    );
    let snapshot = &result_of(&responses[3]);
    assert_eq!(
        snapshot["database"]["format"], "ace",
        "the format stays ACE"
    );
    assert!(
        snapshot["database"]["tables"]
            .as_array()
            .expect("tables")
            .iter()
            .any(|table| table["name"] == "Written"),
        "the created table is catalogued: {snapshot}"
    );

    // Session 2 (fresh core = real reopen): the row persisted.
    let mut reopened = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut reopened,
        &[dispatch(1, json!({"OpenDatabase":{"path": path}}))],
    );
    assert_eq!(
        result_of(&responses[0])["events"][0]["DatabaseOpened"]["format"],
        "ace",
        "{responses:?}"
    );
    let rows = reopened
        .read_rows("Written", RowRange { start: 0, len: 10 })
        .expect("the inserted row persisted");
    assert_eq!(rows, [vec![FieldValue::Text("over the wire".into())]]);

    // The drop runs on the still-open core; session 3 proves it persisted.
    let responses = session(
        &mut reopened,
        &[dispatch(2, json!({"DropTable":{"table":"Written"}}))],
    );
    assert_eq!(
        result_of(&responses[0])["events"][0]["TableDropped"]["table"],
        "Written",
        "{responses:?}"
    );

    // Session 3: the drop persisted and the fixture catalog survived.
    let mut final_core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut final_core,
        &[dispatch(1, json!({"OpenDatabase":{"path": path}}))],
    );
    let snapshot = &result_of(&responses[0])["snapshot"]["database"];
    assert_eq!(snapshot["format"], "ace", "{snapshot}");
    let tables = snapshot["tables"].as_array().expect("tables");
    assert!(
        !tables.iter().any(|table| table["name"] == "Written"),
        "{snapshot:?}"
    );
    assert!(
        tables.iter().any(|table| table["name"] == "Table1"),
        "{snapshot:?}"
    );
    let _ = std::fs::remove_file(&path);
}
