//! Compact over the real MCP process (phase 95, step 95.4; item M126).
//!
//! One JSON-RPC line opens the fixture copy, one dispatches the compact and
//! one inspects: the same command vocabulary, the same event and the same
//! refreshed health facts every client sees.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::json;

fn bin() -> Command {
    Command::cargo_bin("datacraft-mcp").expect("the MCP binary builds")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

fn line(id: u32, method: &str, params: Option<serde_json::Value>) -> String {
    let value = match params {
        Some(params) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        None => json!({"jsonrpc": "2.0", "id": id, "method": method}),
    };
    value.to_string() + "\n"
}

#[test]
fn compact_over_a_real_mcp_session_reports_the_event_and_health_facts() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("shop.mdb");
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    let path = path.to_string_lossy().into_owned();

    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": path, "password": null}})),
        ),
        line(2, "dispatch", Some(json!({"CompactDatabase": null}))),
        line(3, "inspect", None),
    ]
    .concat();

    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseCompacted"))
        .stdout(predicate::str::contains("\"jet_version\""))
        .stdout(predicate::str::contains("\"health\":null"))
        .stderr(predicate::str::is_empty());
}
