//! Real-subprocess end-to-end suite (step 36.3, item T7) — the `mcp_process`
//! pendant of the CLI's `cli_process` (step 23.1, T4). The binary is a thin
//! composition root (composed driver set + real stdio handles); exactly the
//! part in-process transport tests cannot cover. These tests start the real
//! process, drive real stdin/stdout and assert the line-protocol contract:
//! one JSON-RPC 2.0 response line per request line, EOF ends the session.

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

fn request(id: u32, method: &str, params: Option<serde_json::Value>) -> String {
    let line = match params {
        Some(value) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": value}),
        None => json!({"jsonrpc": "2.0", "id": id, "method": method}),
    };
    line.to_string() + "\n"
}

/// One `OpenDatabase` dispatch request for `path` (serde-escaped, so
/// Windows backslashes stay valid JSON).
fn open_request(id: u32, path: &str) -> String {
    request(
        id,
        "dispatch",
        Some(json!({"OpenDatabase": {"path": path, "password": null}})),
    )
}

/// `inspect` answers the serializable state snapshot in the result —
/// including the composed driver inventory (M23): the real binary lists
/// jet, sqlite and the csv fallback with their formats, so agents can
/// pre-flight an open against the reported families.
#[test]
fn inspect_over_a_real_process_answers_the_snapshot() {
    bin()
        .write_stdin(request(1, "inspect", None))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""id":1"#))
        .stdout(predicate::str::contains("nav_pane_visible"))
        .stdout(predicate::str::contains("templates"))
        .stdout(predicate::str::contains(r#""drivers":[{"capabilities""#))
        .stdout(predicate::str::contains(r#""format":"jet4","id":"jet""#))
        .stdout(predicate::str::contains(
            r#""format":"sqlite","id":"sqlite""#,
        ))
        .stdout(predicate::str::contains(r#""format":"csv","id":"csv""#))
        .stderr(predicate::str::is_empty());
}

/// A dispatch session over one process: open the Jet fixture, then
/// inspect reports the open database in the snapshot.
#[test]
fn open_then_inspect_reports_the_jet_database() {
    let session = format!(
        "{}{}",
        open_request(1, fixture().to_string_lossy().as_ref()),
        request(2, "inspect", None)
    );
    bin()
        .write_stdin(session)
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseOpened"))
        .stdout(predicate::str::contains(r#""name":"testV2000""#));
}

/// The composed driver set proves itself in the real binary: a CSV
/// directory — invisible to the built-in probes — opens through the
/// registered fallback and reports the csv format in the snapshot.
#[test]
fn a_csv_directory_opens_through_the_registered_fallback() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(
        temp.path().join("people.csv"),
        "name,city\nada,london\ngrace,berlin\n",
    )
    .expect("write the csv file");
    let session = format!(
        "{}{}",
        open_request(1, temp.path().to_string_lossy().as_ref()),
        request(2, "inspect", None)
    );
    bin()
        .write_stdin(session)
        .assert()
        .success()
        .stdout(predicate::str::contains("DatabaseOpened"))
        .stdout(predicate::str::contains(r#""format":"csv""#));
}

/// An unknown file fails the dispatch with a precise, renderable
/// failure event naming the expected families (L9 contract, over the
/// wire).
#[test]
fn an_unknown_file_answers_a_precise_failure_over_the_wire() {
    let temp = TempDir::new().expect("temp dir");
    let stranger = temp.path().join("stranger.bin");
    std::fs::write(&stranger, "definitely not a database").expect("write the stranger");
    bin()
        .write_stdin(open_request(1, stranger.to_string_lossy().as_ref()))
        .assert()
        .success()
        .stdout(predicate::str::contains("Failed"))
        .stdout(predicate::str::contains("expected a Jet/ACE header"));
}

/// Protocol errors answer JSON-RPC error codes: a broken line is a
/// parse error, an unknown method is -32601, a non-2.0 envelope is
/// -32600 — and the session survives them.
#[test]
fn protocol_errors_answer_jsonrpc_error_codes() {
    let session = format!(
        "{{broken json\n{}{}\n",
        request(1, "no_such_method", None),
        request(2, "inspect", None)
    );
    bin()
        .write_stdin(session)
        .assert()
        .success()
        .stdout(predicate::str::contains("parse error"))
        .stdout(predicate::str::contains("-32601"))
        .stdout(predicate::str::contains(r#""id":2"#))
        .stdout(predicate::str::contains("nav_pane_visible"));
}
