//! Excel workbooks over the real CLI process (phase 96, step 96.3; item
//! M127): the composed driver set claims the workbook, the snapshot names
//! the read-only surface, and a write answers the capability code.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("datacraft-cli").expect("the CLI binary builds")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("excelFixture.xlsx")
}

#[test]
fn the_cli_opens_a_workbook_as_a_read_only_database() {
    let fixture = fixture().to_string_lossy().into_owned();
    bin()
        .args(["open", &fixture, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""driver":"xlsx""#))
        .stdout(predicate::str::contains(r#""format":"excel""#))
        .stdout(predicate::str::contains(r#""name":"Items""#))
        .stdout(predicate::str::contains(r#""name":"Summary""#))
        .stdout(predicate::str::contains(r#""write_rows":false"#))
        .stderr(predicate::str::is_empty());
}

#[test]
fn a_write_on_a_workbook_answers_the_capability_code() {
    let fixture = fixture().to_string_lossy().into_owned();
    bin()
        .args([
            "exec",
            "{\"InsertRow\":{\"table\":\"Items\",\"values\":[]}}",
            "--db",
            &fixture,
            "--json",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("capability"))
        .stdout(predicate::str::contains("write_rows"));
}
