//! Linked-table inventory over the real CLI process (phase 100, step
//! 100.4; item M131): the shipped binary reports the records and never
//! the credential.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("datacraft-cli").expect("the CLI binary builds")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn the_cli_reports_a_file_link_in_the_snapshot() {
    bin()
        .args([
            "open",
            fixture("linkerTestV2007.accdb")
                .to_str()
                .expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"linked_tables\""))
        .stdout(predicate::str::contains("\"Table2\""))
        .stdout(predicate::str::contains("\"kind\":\"access_file\""))
        .stdout(predicate::str::contains("linkeeTest.accdb"))
        .stdout(predicate::str::contains("\"foreign_name\":\"Table1\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn the_cli_redacts_the_odbc_password_over_the_wire() {
    let output = bin()
        .args([
            "open",
            fixture("odbcLinkerTestV2007.accdb")
                .to_str()
                .expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("UTF-8");
    assert!(text.contains("\"Ordrar\""), "{text}");
    assert!(text.contains("\"kind\":\"odbc\""), "{text}");
    assert!(text.contains("DSN=Magnapinna"), "{text}");
    assert!(text.contains("PWD=***"), "{text}");
    assert!(
        !text.contains("DummyPassword"),
        "the fixture's password never reaches stdout: {text}"
    );
}

#[test]
fn opening_a_link_over_the_wire_is_refused_precisely() {
    bin()
        .args([
            "exec",
            "{\"OpenObject\":{\"kind\":\"table\",\"name\":\"Table2\"}}",
            "--db",
            fixture("linkerTestV2007.accdb")
                .to_str()
                .expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"unsupported\""))
        .stdout(predicate::str::contains("linked table"))
        .stderr(predicate::str::is_empty());
}
