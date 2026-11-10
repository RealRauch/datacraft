//! The VBA wire contracts (split from `server_e2e.rs` to stay under
//! the file-size gate; L30): the extraction and the static analysis
//! cross the real wire as read-only inventory.

use super::*;

#[test]
fn vba_project_and_module_source_cross_the_wire() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("programmability.accdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
            json!({"jsonrpc":"2.0","id":3,"method":"dispatch",
                   "params":{"LoadVbaModule":{"module":"InventoryHelpers"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"dispatch",
                   "params":{"OpenObject":{"kind":"module","name":"InventoryHelpers"}}}),
        ],
    );
    let snapshot = &result_of(&responses[1]);
    let modules = snapshot["database"]["vba"]["modules"]
        .as_array()
        .expect("module list");
    assert_eq!(modules.len(), 8, "{snapshot}");
    assert_eq!(modules[0]["name"], "InventoryHelpers", "{snapshot}");
    assert_eq!(modules[0]["kind"], "standard", "{snapshot}");
    assert!(snapshot["database"]["vba_error"].is_null(), "{snapshot}");

    let loaded = &result_of(&responses[2])["events"][0]["VbaModuleLoaded"]["module"];
    assert_eq!(loaded["name"], "InventoryHelpers", "{loaded}");
    assert!(
        loaded["source"]
            .as_str()
            .unwrap_or_default()
            .contains("Attribute VB_Name = \"InventoryHelpers\""),
        "{loaded}"
    );

    let opened = &result_of(&responses[3])["events"][0]["TabOpened"];
    assert_eq!(opened["object"]["kind"], "module", "{opened}");
}

#[test]
fn vba_static_analysis_crosses_the_wire() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("programmability.accdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"AnalyzeVba":null}}),
        ],
    );
    let analysis = &result_of(&responses[1])["events"][0]["VbaAnalyzed"]["analysis"];
    assert_eq!(
        analysis["call_graph"].as_array().expect("call graph").len(),
        8,
        "{analysis}"
    );
    assert!(
        !analysis["sql_literals"].as_array().expect("SQL").is_empty(),
        "{analysis}"
    );
    assert!(
        analysis["table_references"]
            .as_array()
            .expect("table refs")
            .iter()
            .any(|table| table == "Staff Permissions"),
        "{analysis}"
    );
}
