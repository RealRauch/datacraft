//! Linked-table inventory over the real MCP process (phase 100, step
//! 100.4; item M131): `inspect` carries the records, dispatch refuses a
//! link, and the credential never leaves the process.

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
fn the_endpoint_reports_the_link_inventory_without_credentials() {
    let source = fixture("odbcLinkerTestV2007.accdb");
    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": source.to_string_lossy(), "password": null}})),
        ),
        line(2, "inspect", None),
    ]
    .concat();
    let output = bin()
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("UTF-8");
    let reply: serde_json::Value = text
        .lines()
        .find_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).ok()?;
            (value["id"] == 2).then_some(value)
        })
        .expect("the inspect reply");
    let links = &reply["result"]["database"]["linked_tables"];
    assert_eq!(links.as_array().map(Vec::len), Some(1), "{links}");
    let link = &links[0];
    assert_eq!(link["name"], "Ordrar");
    assert_eq!(link["kind"], "odbc");
    assert_eq!(link["foreign_name"], "dbo.Ordrar");
    assert_eq!(link["target"], "DSN=Magnapinna");
    assert!(
        link["connect"]
            .as_str()
            .is_some_and(|c| c.contains("PWD=***"))
    );
    assert!(
        !text.contains("DummyPassword"),
        "the fixture's password never reaches stdout: {text}"
    );
}

#[test]
fn the_endpoint_refuses_to_open_a_link() {
    let source = fixture("linkerTestV2007.accdb");
    let input = [
        line(
            1,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": source.to_string_lossy(), "password": null}})),
        ),
        line(
            2,
            "dispatch",
            Some(json!({"OpenObject": {"kind": "table", "name": "Table2"}})),
        ),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"code\":\"unsupported\""))
        .stdout(predicate::str::contains("linked table"))
        .stderr(predicate::str::is_empty());
}
