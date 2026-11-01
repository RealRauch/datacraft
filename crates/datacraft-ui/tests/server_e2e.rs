//! Headless E2E harness (step 9.2, item H33).
//!
//! Drives the FULL stack over the wire: `transport::serve` with a
//! Jet4Port-composed core against the committed fixture — JSON-RPC lines
//! in, response JSON out, assertions on events and snapshots, never on
//! pixels. This is the template agents copy for automation.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use datacraft_core::core::Core;
use datacraft_core::transport::serve;
use datacraft_storage::detecting::DetectingPort;
use serde_json::{Value, json};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// Generates a throwaway SQLite file (never a committed fixture: the
/// content is deterministic and the file is removed by the test).
fn scratch_sqlite(stem: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "datacraft-e2e-{}-{stem}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch SQLite file");
    conn.execute_batch(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);
         INSERT INTO notes (body) VALUES ('over the wire');",
    )
    .expect("populate scratch SQLite file");
    path
}

/// Runs one serve session; returns the parsed response lines.
fn session(core: &mut Core, requests: &[Value]) -> Vec<Value> {
    let mut input = String::new();
    for request in requests {
        input.push_str(&request.to_string());
        input.push('\n');
    }
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, core);
    String::from_utf8(output)
        .expect("responses are utf-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every response parses"))
        .collect()
}

fn result_of(response: &Value) -> &Value {
    response.get("result").expect("success envelope")
}

#[test]
fn open_the_fixture_and_inspect_over_the_wire() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
        ],
    );
    let events = &result_of(&responses[0])["events"];
    assert_eq!(events[0]["DatabaseOpened"]["format"], "jet4", "{events}");
    assert_eq!(events[0]["DatabaseOpened"]["name"], "testV2000");

    let snapshot = &result_of(&responses[1]);
    let tables: Vec<&str> = snapshot["database"]["tables"]
        .as_array()
        .expect("tables listed")
        .iter()
        .map(|t| t["name"].as_str().expect("table name"))
        .collect();
    assert!(
        tables.contains(&"Table1"),
        "the fixture's user tables surface: {tables:?}"
    );
    assert!(
        !tables.iter().any(|t| t.starts_with("MSys")),
        "system tables stay filtered: {tables:?}"
    );
}

#[test]
fn opens_a_sqlite_database_over_the_wire() {
    let path = scratch_sqlite("notes");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": path.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
        ],
    );
    let events = &result_of(&responses[0])["events"];
    assert_eq!(events[0]["DatabaseOpened"]["format"], "sqlite", "{events}");
    let snapshot = &result_of(&responses[1]);
    assert_eq!(snapshot["database"]["format"], "sqlite", "{snapshot}");
    assert_eq!(snapshot["database"]["tables"][0]["name"], "notes", "{snapshot}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_read_only_backend_rejects_writes_over_the_wire() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"CreateTable":{"definition":{
                       "name":"New","fields":[]}}}}),
        ],
    );
    let events = &result_of(&responses[1])["events"];
    assert!(
        events[0]["Failed"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("read-only"),
        "{events}"
    );
}

#[test]
fn qbe_and_run_over_the_wire() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"CreateQuery":{"name":"All"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"SetQbeCell":{"query":"All","row":"table","col":0,
                       "value":{"kind":"text","value":"Table1"}}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"dispatch",
                   "params":{"SetQbeCell":{"query":"All","row":"field","col":0,
                       "value":{"kind":"text","value":"Id"}}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"dispatch",
                   "params":{"RunQuery":{"query":"All"}}}),
        ],
    );
    let events = &result_of(&responses[4])["events"];
    assert_eq!(events[0]["QueryRan"]["query"], "All", "{events}");
    let rows = events[0]["QueryRan"]["rows"].as_u64().expect("row count");
    assert!(rows > 0, "the fixture's Table1 has rows: {events}");
}

#[test]
fn the_design_document_round_trips_over_the_wire() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let path = std::env::temp_dir().join(format!("datacraft-e2e-{}.datacraft", std::process::id()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"CreateForm":{"name":"Card"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"SaveDesignDocument":{"path": path.to_string_lossy()}}}),
        ],
    );
    assert_eq!(
        result_of(&responses[2])["events"][0]["DesignDocumentSaved"]["path"],
        path.to_string_lossy().as_ref(),
        "{responses:?}"
    );

    // A fresh core reloads the design layer over the wire.
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"LoadDesignDocument":{"path": path.to_string_lossy()}}}),
        ],
    );
    let loaded = &result_of(&responses[1])["events"][0]["DesignDocumentLoaded"];
    assert_eq!(loaded["forms"], 1, "{loaded}");
    let _ = std::fs::remove_file(&path);
}
