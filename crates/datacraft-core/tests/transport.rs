//! Contract suite for the JSON-RPC transport codec (step 4.3, item H15).
//!
//! Locks the headless seam (decision default 8): `handle_json` is a
//! synchronous, server-less JSON-RPC 2.0 codec over the command bus.
//! Application-level failures are results (events), transport-level
//! problems are JSON-RPC errors — the layering agents rely on.

use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::transport::handle_json;
use serde_json::{Value, json};

fn core() -> Core {
    Core::new(Box::new(MockPort::new()))
}

fn response_of(core: &mut Core, request: Value) -> Value {
    let raw = handle_json(core, &request.to_string());
    serde_json::from_str(&raw).expect("every response must parse as JSON")
}

fn error_code(response: &Value) -> i64 {
    response["error"]["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected error envelope: {response}"))
}

// ---------------------------------------------------------------------------
// success path
// ---------------------------------------------------------------------------

#[test]
fn dispatch_returns_events_and_snapshot() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 1, "method": "dispatch",
               "params": {"ToggleNavPane": null}}),
    );
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(
        response["result"]["events"][0]["NavPaneToggled"]["visible"],
        false
    );
    assert_eq!(response["result"]["snapshot"]["nav_pane_visible"], false);
}

#[test]
fn inspect_returns_the_snapshot() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": "abc", "method": "inspect"}),
    );
    assert_eq!(response["id"], "abc");
    assert!(response["result"]["nav_pane_visible"].is_boolean());
    assert!(response["result"]["database"].is_null());
}

#[test]
fn dispatch_round_trip_opens_a_database() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 7, "method": "dispatch",
               "params": {"OpenDatabase": {"path": "shop.mdb"}}}),
    );
    let events = &response["result"]["events"];
    assert_eq!(events[0]["DatabaseOpened"]["name"], "shop");
    assert_eq!(events[0]["DatabaseOpened"]["format"], "jet4");
    assert_eq!(response["result"]["snapshot"]["database"]["name"], "shop");
}

#[test]
fn application_failures_are_results_not_errors() {
    let mut core = core();
    core.dispatch(datacraft_core::command::Command::ToggleNavPane);
    // Second open while the mock has no database concept — use a real
    // failure: opening an object with no database open.
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 2, "method": "dispatch",
               "params": {"OpenObject": {"kind": "table", "name": "Customers"}}}),
    );
    assert!(
        response.get("error").is_none(),
        "no transport error: {response}"
    );
    let reason = response["result"]["events"][0]["Failed"]["reason"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        reason.contains("no database"),
        "reason must name the failure: {response}"
    );
}

#[test]
fn notification_style_requests_without_id_are_rejected() {
    let mut core = core();
    let response = response_of(&mut core, json!({"jsonrpc": "2.0", "method": "inspect"}));
    assert_eq!(error_code(&response), -32600);
}

// ---------------------------------------------------------------------------
// transport errors
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_is_a_parse_error() {
    let mut core = core();
    let raw = handle_json(&mut core, "{ not json");
    let response: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(error_code(&response), -32700);
    assert!(response["id"].is_null());
}

#[test]
fn wrong_jsonrpc_version_is_an_invalid_request() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "1.0", "id": 1, "method": "inspect"}),
    );
    assert_eq!(error_code(&response), -32600);
}

#[test]
fn missing_method_is_an_invalid_request() {
    let mut core = core();
    let response = response_of(&mut core, json!({"jsonrpc": "2.0", "id": 1}));
    assert_eq!(error_code(&response), -32600);
}

#[test]
fn batch_requests_are_rejected_by_the_stub() {
    let mut core = core();
    let raw = handle_json(
        &mut core,
        "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}]",
    );
    let response: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(error_code(&response), -32600);
}

#[test]
fn unknown_method_is_method_not_found() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 3, "method": "execute"}),
    );
    assert_eq!(error_code(&response), -32601);
}

#[test]
fn malformed_command_params_are_invalid_params() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 4, "method": "dispatch", "params": {"Bogus": {}}}),
    );
    assert_eq!(error_code(&response), -32602);
}

#[test]
fn non_object_params_are_invalid_params() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 5, "method": "dispatch", "params": "nope"}),
    );
    assert_eq!(error_code(&response), -32602);
}

#[test]
fn dispatch_without_params_is_invalid_params() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 6, "method": "dispatch"}),
    );
    assert_eq!(error_code(&response), -32602);
}

// ---------------------------------------------------------------------------
// envelope shape
// ---------------------------------------------------------------------------

#[test]
fn every_error_carries_a_message() {
    let mut core = core();
    for request in [
        json!({"jsonrpc": "1.0", "id": 1, "method": "inspect"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "nope"}),
    ] {
        let response = response_of(&mut core, request);
        let message = response["error"]["message"].as_str().unwrap_or_default();
        assert!(!message.is_empty(), "empty message: {response}");
    }
}

#[test]
fn error_responses_carry_the_request_id_when_present() {
    let mut core = core();
    let response = response_of(
        &mut core,
        json!({"jsonrpc": "2.0", "id": 42, "method": "nope"}),
    );
    assert_eq!(response["id"], 42);
}
