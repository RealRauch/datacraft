//! Shipped-samples E2E (phase 29, step 29.4, item H44).
//!
//! The strongest chain of the sample work in one wire pass: the shipped
//! demo database opens over the JSON-RPC bus (the vocabulary MCP and the
//! CLI share) and its `.datacraft` demo sidecar auto-loads with the open.

use std::path::Path;

use datacraft_core::core::Core;
use datacraft_storage::detecting::DetectingPort;
use serde_json::json;

mod support;

use support::{dispatch, result_of, session};

#[test]
fn the_bike_shop_demo_opens_and_auto_loads_its_sidecar() {
    let demo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("bike_shop.mdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            dispatch(1, json!({"OpenDatabase":{"path": demo.to_string_lossy()}})),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
        ],
    );
    let opened = &result_of(&responses[0])["events"];
    assert_eq!(opened[0]["DatabaseOpened"]["format"], "jet4", "{opened}");
    assert_eq!(
        opened[1]["DesignDocumentLoaded"]["forms"], 1,
        "the demo sidecar auto-loads: {opened}"
    );
    assert_eq!(opened[1]["DesignDocumentLoaded"]["queries"], 1, "{opened}");

    let snapshot = &result_of(&responses[1])["database"];
    assert_eq!(snapshot["format"], "jet4", "{snapshot}");
    let tables: Vec<&str> = snapshot["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        tables,
        ["Bikes", "Customers", "ServiceOrders"],
        "{snapshot:?}"
    );
    let forms = snapshot["forms"].as_array().expect("forms");
    assert_eq!(
        forms[0]["name"], "Bike Finder",
        "the demo form is in: {forms:?}"
    );
    assert_eq!(
        snapshot["queries"]
            .as_array()
            .expect("queries")
            .iter()
            .map(|query| query["name"].as_str().expect("name"))
            .collect::<Vec<_>>(),
        ["Expensive bikes"],
        "{snapshot:?}"
    );
    assert!(snapshot["sidecar_error"].is_null(), "{snapshot:?}");
}
