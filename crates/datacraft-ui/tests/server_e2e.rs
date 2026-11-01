//! Headless E2E harness (step 9.2, item H33).
//!
//! Drives the FULL stack over the wire: `transport::serve` with a
//! DetectingPort-composed core against the committed fixture — JSON-RPC lines
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
    assert_eq!(
        snapshot["database"]["tables"][0]["name"], "notes",
        "{snapshot}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn jet_writes_persist_over_the_wire_and_drops_stay_precise() {
    // Writes land on a private copy — fixtures are immutable evidence.
    let path = std::env::temp_dir().join(format!(
        "datacraft-e2e-{}-jet-write.mdb",
        std::process::id()
    ));
    std::fs::copy(fixture(), &path).expect("copy the fixture");

    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": path.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"CreateTable":{"definition":{
                       "name":"Written",
                       "fields":[{"name":"note","kind":{"type":"memo"},
                                  "properties":{"caption":null,"input_mask":null,
                                  "format":null,"default_value":null,
                                  "validation_rule":null,"validation_text":null,
                                  "required":false,"allow_zero_length":false,
                                  "indexed":"none"}}]}}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"InsertRow":{"table":"Written",
                       "values":[{"Text":"over the wire"}]}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"dispatch",
                   "params":{"DropTable":{"table":"Written"}}}),
        ],
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
    let failed = &result_of(&responses[3])["events"];
    assert!(
        failed[0]["Failed"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("M7"),
        "{failed}"
    );

    // Reopen verifies the write persisted.
    let mut reopened = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut reopened,
        &[json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                  "params":{"OpenDatabase":{"path": path.to_string_lossy()}}})],
    );
    let snapshot = &result_of(&responses[0])["snapshot"]["database"]["tables"];
    assert!(
        snapshot
            .as_array()
            .expect("tables")
            .iter()
            .any(|t| t["name"] == "Written"),
        "{snapshot}"
    );
    let _ = std::fs::remove_file(&path);
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

#[test]
fn creates_and_fills_a_sqlite_table_over_the_wire() {
    use datacraft_core::command::Command;
    use datacraft_core::schema::{
        AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
    };

    let path = scratch_sqlite("writes");
    let definition = TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    );
    let create =
        serde_json::to_value(Command::CreateTable { definition }).expect("serialize CreateTable");
    let insert = serde_json::to_value(Command::InsertRow {
        table: "people".into(),
        values: vec![
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(2.5),
        ],
    })
    .expect("serialize InsertRow");

    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": path.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch","params":create}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch","params":insert}),
        ],
    );
    assert_eq!(
        result_of(&responses[1])["events"][0]["TableCreated"]["table"],
        "people",
        "{responses:?}"
    );
    assert_eq!(
        result_of(&responses[2])["events"][0]["RowInserted"]["table"],
        "people",
        "{responses:?}"
    );

    // Persistence: a fresh core reopens the file and reads the row back.
    let mut reopened = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut reopened,
        &[json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                  "params":{"OpenDatabase":{"path": path.to_string_lossy()}}})],
    );
    assert_eq!(
        result_of(&responses[0])["events"][0]["DatabaseOpened"]["format"],
        "sqlite",
        "{responses:?}"
    );
    let rows = reopened
        .read_rows(
            "people",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("the inserted row persisted");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], FieldValue::Integer(1));
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][2], FieldValue::Float(2.5));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn run_sql_over_the_wire() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture().to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"RunSql":{"sql":"SELECT COUNT(*) AS n FROM \"Table1\""}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"RunSql":{"sql":"DELETE FROM Table1"}}}),
        ],
    );
    let events = &result_of(&responses[1])["events"];
    assert_eq!(events[0]["SqlRan"]["rows"], 1, "{events}");
    let failed = &result_of(&responses[2])["events"];
    assert!(
        failed[0]["Failed"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("only SELECT"),
        "{failed}"
    );
}

#[test]
fn opens_an_ace_fixture_over_the_wire() {
    let ace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2007.accdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": ace.to_string_lossy()}}}),
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

#[test]
fn vba_project_and_module_source_cross_the_wire() {
    let northwind = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("northwind2007.accdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": northwind.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"LoadVbaModule":{"module":"Utilities"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"dispatch",
                   "params":{"OpenObject":{"kind":"module","name":"Utilities"}}}),
        ],
    );
    let snapshot = &result_of(&responses[1]);
    let modules = snapshot["database"]["vba"]["modules"]
        .as_array()
        .expect("module list");
    assert_eq!(modules.len(), 18, "{snapshot}");
    assert_eq!(modules[0]["name"], "Utilities", "{snapshot}");
    assert_eq!(modules[0]["kind"], "standard", "{snapshot}");
    assert!(snapshot["database"]["vba_error"].is_null(), "{snapshot}");

    let loaded = &result_of(&responses[2])["events"][0]["VbaModuleLoaded"]["module"];
    assert_eq!(loaded["name"], "Utilities", "{loaded}");
    assert!(
        loaded["source"]
            .as_str()
            .unwrap_or_default()
            .contains("Attribute VB_Name = \"Utilities\""),
        "{loaded}"
    );

    let opened = &result_of(&responses[3])["events"][0]["TabOpened"];
    assert_eq!(opened["object"]["kind"], "module", "{opened}");
}
