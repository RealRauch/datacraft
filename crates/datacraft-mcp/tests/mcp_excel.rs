//! Excel workbooks over the real MCP process (phase 96, step 96.3; item
//! M127): the stdio endpoint lists the driver, opens the workbook and
//! reports the same read-only surface every other client sees.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
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
        .join("excelFixture.xlsx")
}

fn line(id: u32, method: &str, params: Option<serde_json::Value>) -> String {
    let value = match params {
        Some(params) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        None => json!({"jsonrpc": "2.0", "id": id, "method": method}),
    };
    value.to_string() + "\n"
}

#[test]
fn the_mcp_endpoint_lists_and_opens_the_excel_driver() {
    let path = fixture().to_string_lossy().into_owned();
    let input = [
        line(1, "inspect", None),
        line(
            2,
            "dispatch",
            Some(json!({"OpenDatabase": {"path": path, "password": null}})),
        ),
        line(3, "inspect", None),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""driver":"xlsx""#))
        .stdout(predicate::str::contains(r#""format":"excel""#))
        .stdout(predicate::str::contains(r#""write_rows":false"#))
        .stderr(predicate::str::is_empty());
}

#[test]
fn the_driver_inventory_names_all_four_drivers() {
    bin()
        .write_stdin(line(1, "inspect", None))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""id":"jet""#))
        .stdout(predicate::str::contains(r#""id":"sqlite""#))
        .stdout(predicate::str::contains(r#""id":"csv""#))
        .stdout(predicate::str::contains(r#""id":"xlsx""#));
}
