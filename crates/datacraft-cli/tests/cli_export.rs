//! Table export over the real CLI process (phase 99, step 99.3; item
//! M130).
//!
//! The composition root is the part in-process tests cannot cover: the
//! shipped binary must reach the same command through its wire form, and
//! a real database file proves the source stays byte-identical while the
//! exported document lands beside it.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("datacraft-cli").expect("the CLI binary builds")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// A private copy of the pinned fixture (fixtures are immutable evidence).
fn scratch_copy(temp: &TempDir) -> PathBuf {
    let path = temp.path().join("shop.mdb");
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

/// The wire form of one export command.
fn command(table: &str, format: &str, path: &Path) -> String {
    let target = serde_json::to_string(&path.to_string_lossy()).expect("path json");
    format!(
        "{{\"ExportTable\":{{\"table\":{table},\"format\":\"{format}\",\"path\":{target}}}}}",
        table = serde_json::to_string(table).expect("table json"),
    )
}

#[test]
fn the_cli_exports_a_table_to_csv_and_json_and_leaves_the_source_untouched() {
    let temp = TempDir::new().expect("temp dir");
    let source = scratch_copy(&temp);
    let before = std::fs::read(&source).expect("read the source");
    let csv = temp.path().join("table1.csv");
    let json = temp.path().join("table1.json");
    for (path, format) in [(&csv, "csv"), (&json, "json")] {
        bin()
            .args([
                "exec",
                &command("Table1", format, path),
                "--db",
                source.to_str().expect("utf-8 path"),
                "--json",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("TableExported"))
            .stdout(predicate::str::contains("\"table\":\"Table1\""))
            .stderr(predicate::str::is_empty());
    }
    // The JSON document names the same table and carries a schema; the
    // CSV's header is exactly that schema's field names.
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json).expect("the JSON exists")).expect("valid");
    assert_eq!(doc["table"], "Table1");
    let fields: Vec<&str> = doc["schema"]
        .as_array()
        .expect("a schema array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("a field name"))
        .collect();
    assert!(!fields.is_empty(), "the table has fields");
    let text = String::from_utf8(std::fs::read(&csv).expect("the CSV exists")).expect("UTF-8");
    let header = text.lines().next().expect("a header row");
    assert_eq!(header, fields.join(","), "the CSV header is the schema");
    assert_eq!(
        std::fs::read(&source).expect("read the source again"),
        before,
        "the source database is byte-identical"
    );
}

#[test]
fn an_existing_target_is_refused_over_the_wire() {
    let temp = TempDir::new().expect("temp dir");
    let source = scratch_copy(&temp);
    let target = temp.path().join("taken.csv");
    std::fs::write(&target, b"keep me").expect("seed the target");
    bin()
        .args([
            "exec",
            &command("Table1", "csv", &target),
            "--db",
            source.to_str().expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"io\""))
        .stdout(predicate::str::contains("already exists"))
        .stderr(predicate::str::is_empty());
    assert_eq!(
        std::fs::read(&target).expect("the target"),
        b"keep me",
        "the foreign file is untouched"
    );
}
