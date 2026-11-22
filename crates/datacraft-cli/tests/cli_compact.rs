//! Compact over the real CLI process (phase 95, step 95.4; item M126).
//!
//! The composition root is the part in-process tests cannot cover: the
//! binary must reach the same command, report the event plus the refreshed
//! snapshot facts, and refuse a source without the capability with the
//! machine-readable code.

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
fn scratch_copy() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("shop.mdb");
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    (temp, path)
}

#[test]
fn compact_over_the_real_cli_reports_the_event_and_the_refreshed_facts() {
    let (_temp, path) = scratch_copy();
    bin()
        .args([
            "exec",
            "{\"CompactDatabase\":null}",
            "--db",
            path.to_str().expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseCompacted"))
        .stdout(predicate::str::contains("\"jet_version\""))
        .stdout(predicate::str::contains("\"health\":null"))
        .stderr(predicate::str::is_empty());

    // The swapped file is still a full database for the next process.
    bin()
        .args(["open", path.to_str().expect("utf-8 path"), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Table1\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn compact_of_a_source_without_the_capability_names_the_code() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("notes.csv"), "id,name\n1,Ada\n").expect("csv");
    bin()
        .args([
            "exec",
            "{\"CompactDatabase\":null}",
            "--db",
            temp.path().to_str().expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("capability"))
        .stdout(predicate::str::contains("compact"));
}
