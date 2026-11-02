//! Shared helpers for the headless E2E suites (H33 template): one serve
//! session over in-memory streams plus the response accessors. Every test
//! binary includes this module and uses a subset — dead code is allowed.

#![allow(dead_code)]

use std::io::Cursor;
use std::path::{Path, PathBuf};

use datacraft_core::core::Core;
use datacraft_core::transport::serve;
use serde_json::{Value, json};

/// The committed Jet4 fixture every read-path E2E opens.
pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// A pinned fixture by file name.
pub fn fixture_named(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

/// Runs one serve session; returns the parsed response lines.
pub fn session(core: &mut Core, requests: &[Value]) -> Vec<Value> {
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

/// The `result` envelope of one response.
pub fn result_of(response: &Value) -> &Value {
    response.get("result").expect("success envelope")
}

/// Convenience: one `dispatch` request envelope.
pub fn dispatch(id: u64, params: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"dispatch","params":params})
}
