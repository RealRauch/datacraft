//! Contract suite for the CLI client (step 9.4, item H8).
//!
//! The CLI is the fourth command-bus client: args map to commands, output
//! is deterministic (human or `--json`), exit codes are meaningful, and
//! every command-bus operation is reachable via `exec`. Transport only —
//! no business logic.

use std::path::{Path, PathBuf};

use datacraft_cli::{Outcome, run};
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("datacraft-ui")
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

fn fresh_core() -> Core {
    Core::new(Box::new(MockPort::new()))
}

#[test]
fn open_prints_a_json_snapshot_and_succeeds() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &["open".to_owned(), fixture().to_string_lossy().to_string()],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("\"name\":\"testV2000\""), "{text}");
    assert!(text.contains("DatabaseOpened"), "{text}");
}

#[test]
fn exec_reaches_every_command_and_reports_failures() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"ToggleNavPane\":null}".to_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    assert!(String::from_utf8(out.clone()).unwrap().contains("NavPaneToggled"));

    // A failing command keeps exit code 2 with the reason printed.
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"CloseDatabase\":null}".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::CommandFailed);
    assert!(
        String::from_utf8(out).unwrap().contains("no database"),
        "failure reason printed"
    );
}

#[test]
fn exec_with_a_db_opens_first() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"ToggleNavPane\":null}".to_owned(),
            "--db".to_owned(),
            fixture().to_string_lossy().to_string(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("DatabaseOpened"), "{text}");
    assert!(text.contains("NavPaneToggled"), "{text}");
}

#[test]
fn usage_errors_exit_with_code_one() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(run(&mut core, &[], &mut out), Outcome::UsageError);
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(run(&mut core, &["frobnicate".to_owned()], &mut out), Outcome::UsageError);
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        run(
            &mut core,
            &["exec".to_owned(), "not json".to_owned()],
            &mut out
        ),
        Outcome::UsageError
    );
}

#[test]
fn human_output_names_events_without_json_braces() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &["exec".to_owned(), "{\"ToggleNavPane\":null}".to_owned()],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("NavPaneToggled"), "{text}");
    assert!(text.contains("ok"), "{text}");
}

#[test]
fn repl_shares_the_transport_over_in_memory_streams() {
    use std::io::Cursor;

    let mut core = fresh_core();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}\n";
    let mut output: Vec<u8> = Vec::new();
    datacraft_cli::repl(
        &mut core,
        &mut Cursor::new(input.to_owned()),
        &mut output,
    );
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("nav_pane_visible"), "{text}");
}
