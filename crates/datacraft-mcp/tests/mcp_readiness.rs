//! The app-readiness report over the real MCP process (phase 103, step
//! 103.2; item M134): `dispatch` carries `AnalyzeApp` and the reply's
//! result holds the report.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;

fn bin() -> Command {
    Command::cargo_bin("datacraft-mcp").expect("the MCP binary builds")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn line(id: u32, method: &str, params: Option<serde_json::Value>) -> String {
    let value = match params {
        Some(params) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        None => json!({"jsonrpc": "2.0", "id": id, "method": method}),
    };
    value.to_string() + "\n"
}

#[test]
fn the_endpoint_answers_analyze_app_with_the_report() {
    let source = fixture("programmability.accdb");
    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": source.to_string_lossy(), "password": null}})),
        ),
        line(2, "dispatch", Some(json!("AnalyzeApp"))),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("\"AppAnalyzed\""))
        .stdout(predicate::str::contains("\"report\""))
        .stdout(predicate::str::contains("\"objects\":"))
        .stdout(predicate::str::contains("\"effort\":"));
}
