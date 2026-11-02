//! CSV-directory wire contracts (phase 30, item M18; ADR-7).
//!
//! The out-of-tree CSV driver docks through the composite's registered
//! fallback: the full stack (open, snapshot with driver/capabilities/
//! availability, row writes, reopen) over JSON-RPC lines — the template
//! agents copy for automation against a CSV directory.

mod support;

use datacraft_core::core::Core;
use datacraft_storage::detecting::DetectingPort;
use serde_json::json;

use support::{result_of, session};

fn csv_driver() -> Box<dyn datacraft_core::port::StoragePort> {
    Box::new(datacraft_csv::CsvPort::new())
}

/// A CSV directory docks as an out-of-tree driver over the wire (phase
/// 30, item M18; ADR-7): the composite opens it through the registered
/// fallback, the snapshot carries driver/capabilities, and row writes
/// persist across a reopen.
#[test]
fn a_csv_directory_drives_the_full_stack_over_the_wire() {
    let dir = std::env::temp_dir().join(format!("datacraft-e2e-{}-csv-dir", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the csv directory");
    std::fs::write(dir.join("notes.csv"), "body\nhello\n").expect("write the csv table");

    let mut core = Core::new(Box::new(DetectingPort::new().with_fallback(csv_driver)));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": dir.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"InsertRow":{"table":"notes","values":[{"Text":"over the wire"}]}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"dispatch",
                   "params":{"CloseDatabase":{}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": dir.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":6,"method":"inspect"}),
        ],
    );

    let events = &result_of(&responses[0])["events"];
    assert_eq!(events[0]["DatabaseOpened"]["format"], "csv", "{events}");
    let stem = dir.file_stem().unwrap().to_string_lossy().into_owned();
    assert_eq!(events[0]["DatabaseOpened"]["name"], stem);

    let snapshot = &result_of(&responses[1]);
    assert_eq!(snapshot["database"]["driver"], "csv");
    assert_eq!(snapshot["database"]["format"], "csv");
    assert_eq!(
        snapshot["database"]["capabilities"]["atomic_batch"],
        json!(true)
    );
    assert_eq!(
        snapshot["database"]["capabilities"]["alter_table"],
        json!(false)
    );
    // CSV has no relationships, so the mapper affordance is disabled —
    // the availability map names exactly that (ADR-7).
    assert_eq!(
        snapshot["database"]["availability"],
        json!({"relationships": "no_relationships"})
    );

    let events = &result_of(&responses[2])["events"];
    assert_eq!(events[0]["RowInserted"]["table"], "notes", "{events}");

    let snapshot = &result_of(&responses[5]);
    let tables = snapshot["database"]["tables"].as_array().expect("tables");
    assert_eq!(tables[0]["name"], "notes");

    let _ = std::fs::remove_dir_all(&dir);
}
