//! Real-subprocess end-to-end suite (step 23.1, item T4).
//!
//! The CLI binary is a thin composition root over the library — exactly the
//! part that in-process tests cannot cover. These tests start the real
//! process, drive real stdin/stdout/stderr and assert the documented exit
//! codes, so argument parsing and stream wiring are covered too.

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

fn fixture_arg() -> String {
    fixture().to_string_lossy().into_owned()
}

#[test]
fn open_over_a_real_process_prints_the_snapshot_and_exits_zero() {
    bin()
        .args(["open", &fixture_arg(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseOpened"))
        .stdout(predicate::str::contains("\"name\":\"testV2000\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn exec_runs_against_the_opened_database_over_real_stdio() {
    bin()
        .args([
            "exec",
            "{\"RunExpression\":{\"context\":null,\"expression\":\"1+1\"}}",
            "--db",
            &fixture_arg(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ExpressionEvaluated"));
}

#[test]
fn a_missing_file_exits_2_with_a_precise_reason_on_stderr() {
    let temp = TempDir::new().expect("temp dir");
    let missing = temp.path().join("missing.mdb");
    bin()
        .args(["open", missing.to_str().expect("utf-8 path")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("database open failed"));
}

#[test]
fn a_rejected_command_exits_2_and_reports_on_stderr() {
    bin()
        .args([
            "exec",
            "{\"OpenObject\":{\"kind\":\"table\",\"name\":\"NoSuchTable\"}}",
            "--db",
            &fixture_arg(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn an_unknown_subcommand_exits_1_with_usage_on_stderr() {
    bin()
        .arg("frobnicate")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("unknown command"));
}

#[test]
fn a_non_interactive_repl_session_answers_every_request() {
    bin()
        .args(["repl", "--db", &fixture_arg()])
        .write_stdin(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"inspect\"}\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\":1"))
        .stdout(predicate::str::contains("\"id\":2"));
}

#[test]
fn open_accepts_a_file_uri_over_the_real_process() {
    let uri = if cfg!(windows) {
        format!("file:///{}", fixture_arg().replace('\\', "/"))
    } else {
        format!("file://{}", fixture_arg())
    };
    bin()
        .args(["open", &uri, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseOpened"))
        .stdout(predicate::str::contains("\"source\":{\"uri\""));
}
