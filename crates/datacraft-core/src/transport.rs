//! JSON-RPC 2.0 transport codec (step 4.3, item H15; decision default 8).
//!
//! A synchronous, server-less codec over the command bus: Phase 9.1 wraps
//! this in a stdio loop. Stub semantics (documented, contract-tested):
//! requests must be single JSON-RPC 2.0 objects with an `id`
//! (notifications and batches are rejected with `-32600`); application
//! failures surface as `Event::Failed` in the **result**, never as
//! transport errors.

use serde_json::{Value, json};

use crate::command::Command;
use crate::core::Core;

/// Methods of the stub.
const METHOD_DISPATCH: &str = "dispatch";
const METHOD_INSPECT: &str = "inspect";

/// Applies one JSON-RPC request to the core and renders the response.
pub fn handle_json(core: &mut Core, request_json: &str) -> String {
    let request: Value = match serde_json::from_str(request_json) {
        Ok(value) => value,
        Err(e) => return error_response(Value::Null, -32700, &format!("parse error: {e}")),
    };
    if !request.is_object() {
        return error_response(
            Value::Null,
            -32600,
            "invalid request: only single JSON-RPC 2.0 objects are supported (no batches)",
        );
    }
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let version = request.get("jsonrpc").and_then(Value::as_str);
    let method = request.get("method").and_then(Value::as_str);
    if version != Some("2.0") || method.is_none() || request.get("id").is_none() {
        return error_response(
            id,
            -32600,
            "invalid request: jsonrpc must be \"2.0\" and method and id are required",
        );
    }
    match method.unwrap_or_default() {
        METHOD_DISPATCH => dispatch(core, &request, id),
        METHOD_INSPECT => {
            success_response(id, serde_json::to_value(core.inspect()).unwrap_or_default())
        }
        other => error_response(id, -32601, &format!("method {other:?} not found")),
    }
}

/// Runs one command; result = events + snapshot (the post-state).
fn dispatch(core: &mut Core, request: &Value, id: Value) -> String {
    let Some(params) = request.get("params") else {
        return error_response(
            id,
            -32602,
            "invalid params: dispatch requires the command as params",
        );
    };
    let command: Command = match serde_json::from_value(params.clone()) {
        Ok(command) => command,
        Err(e) => return error_response(id, -32602, &format!("invalid params: {e}")),
    };
    let events = core.dispatch(command);
    let result = json!({
        "events": events,
        "snapshot": core.inspect(),
    });
    success_response(id, result)
}

fn success_response(id: Value, result: Value) -> String {
    render(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    render(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    }))
}

/// Value rendering is infallible (no non-string map keys).
fn render(value: Value) -> String {
    value.to_string()
}

/// Serves line-delimited JSON-RPC requests from , writing one
/// response line per request to  (flushed immediately). Blank
/// lines are skipped silently; EOF ends the session. The MCP endpoint
/// (decision default 8) is exactly this loop over stdin/stdout.
pub fn serve(reader: &mut dyn std::io::BufRead, writer: &mut dyn std::io::Write, core: &mut Core) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let request = line.trim();
        if request.is_empty() {
            continue;
        }
        let response = handle_json(core, request);
        if writeln!(writer, "{response}").is_err() {
            return;
        }
        let _ = writer.flush();
    }
}
