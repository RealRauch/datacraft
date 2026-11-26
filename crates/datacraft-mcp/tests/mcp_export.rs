//! Table export over the real MCP process (phase 99, step 99.3; item
//! M130): the endpoint reaches the same command and reports the same
//! event through its wire vocabulary.

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
fn the_endpoint_exports_a_table_as_json() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("shop.mdb");
    std::fs::copy(fixture(), &source).expect("copy the fixture");
    let before = std::fs::read(&source).expect("read the source");
    let target = temp.path().join("table1.json");
    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": source.to_string_lossy(), "password": null}})),
        ),
        line(
            2,
            "dispatch",
            Some(json!({"ExportTable": {
                "table": "Table1",
                "format": "json",
                "path": target.to_string_lossy(),
            }})),
        ),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("TableExported"))
        .stdout(predicate::str::contains("\"table\":\"Table1\""))
        .stderr(predicate::str::is_empty());
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target).expect("the document exists"))
            .expect("valid JSON");
    assert_eq!(doc["table"], "Table1");
    assert!(doc["rows"].as_array().is_some(), "the rows array exists");
    assert_eq!(
        std::fs::read(&source).expect("read the source again"),
        before,
        "the source database is byte-identical"
    );
}

#[test]
fn the_endpoint_refuses_an_unknown_table_precisely() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("shop.mdb");
    std::fs::copy(fixture(), &source).expect("copy the fixture");
    let target = temp.path().join("missing.csv");
    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": source.to_string_lossy(), "password": null}})),
        ),
        line(
            2,
            "dispatch",
            Some(json!({"ExportTable": {
                "table": "NoSuchTable",
                "format": "csv",
                "path": target.to_string_lossy(),
            }})),
        ),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"code\":\"not_found\""))
        .stdout(predicate::str::contains("NoSuchTable"))
        .stderr(predicate::str::is_empty());
    assert!(!target.exists(), "nothing is written on failure");
}
