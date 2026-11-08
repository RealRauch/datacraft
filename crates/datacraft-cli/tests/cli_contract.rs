//! Contract suite for the CLI client (step 9.4, item H8).
//!
//! The CLI is the fourth command-bus client: args map to commands, output
//! is deterministic (human or `--json`), exit codes are meaningful, and
//! every command-bus operation is reachable via `exec`. Transport only —
//! no business logic.

use std::path::{Path, PathBuf};

use datacraft_cli::{Outcome, run};
use datacraft_core::core::Core;
use datacraft_core::port::{DriverCapabilities, MockPort};

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
        &[
            "open".to_owned(),
            fixture().to_string_lossy().to_string(),
            "--json".to_owned(),
        ],
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
    assert!(
        String::from_utf8(out.clone())
            .unwrap()
            .contains("NavPaneToggled")
    );

    // A failing command keeps exit code 2 with the reason printed.
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &["exec".to_owned(), "{\"CloseDatabase\":null}".to_owned()],
        &mut out,
    );
    assert_eq!(outcome, Outcome::CommandFailed);
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("NoDatabase"),
        "the stable failure code is printed (I6): {text}"
    );
    assert!(text.contains("no database"), "failure reason printed");
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
fn sqlite_databases_open_through_the_cli_vocabulary() {
    let path = std::env::temp_dir().join(format!(
        "datacraft-cli-{}-sqlite.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch SQLite file");
    conn.execute_batch(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);
         INSERT INTO notes (body) VALUES ('cli');",
    )
    .expect("populate scratch SQLite file");

    let mut core = Core::new(Box::new(datacraft_storage::detecting::DetectingPort::new()));
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "open".to_owned(),
            path.to_string_lossy().to_string(),
            "--json".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("\"format\":\"sqlite\""), "{text}");
    assert!(text.contains("\"name\":\"notes\""), "{text}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn usage_errors_exit_with_code_one() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(run(&mut core, &[], &mut out), Outcome::UsageError);
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        run(&mut core, &["frobnicate".to_owned()], &mut out),
        Outcome::UsageError
    );
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
fn missing_arguments_are_usage_errors() {
    let mut core = fresh_core();
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        run(&mut core, &["open".to_owned()], &mut out),
        Outcome::UsageError
    );
    assert!(String::from_utf8(out).unwrap().contains("open <path>"));

    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        run(&mut core, &["exec".to_owned()], &mut out),
        Outcome::UsageError
    );
    assert!(String::from_utf8(out).unwrap().contains("exec"));
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
    datacraft_cli::repl(&mut core, &mut Cursor::new(input.to_owned()), &mut output);
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("nav_pane_visible"), "{text}");
}

#[test]
fn a_failing_pre_open_reports_the_failure_with_exit_code_two() {
    let mut core = Core::new(Box::new(MockPort::failing_open("disk unreadable")));
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"ToggleNavPane\":null}".to_owned(),
            "--db".to_owned(),
            "broken.mdb".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::CommandFailed);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("disk unreadable"), "{text}");
    assert!(
        !text.contains("NavPaneToggled"),
        "the command never ran: {text}"
    );
}

#[test]
fn ace_databases_open_through_the_cli_vocabulary() {
    let ace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2010.accdb");
    let mut core = Core::new(Box::new(datacraft_storage::detecting::DetectingPort::new()));
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "open".to_owned(),
            ace.to_string_lossy().to_string(),
            "--json".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("\"format\":\"ace\""), "{text}");
    assert!(text.contains("\"name\":\"testV2010\""), "{text}");
}

/// ACE write-through through the CLI vocabulary (phase 25, step 25.2, item
/// H43): `exec` creates a table and inserts a row on a private copy of the
/// pinned ACE 2010 fixture; a fresh core proves the persistence on reopen.
#[test]
fn ace_databases_take_writes_through_the_cli_vocabulary() {
    use datacraft_core::command::Command;
    use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

    let copy = std::env::temp_dir().join(format!(
        "datacraft-cli-{}-ace-write.accdb",
        std::process::id()
    ));
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("testV2010.accdb"),
        &copy,
    )
    .expect("copy the fixture");

    let definition = TableDef::new("CliWritten", vec![FieldDef::new("note", FieldKind::Memo)]);
    let create = serde_json::to_string(&Command::CreateTable { definition }).expect("serialize");
    let insert = serde_json::to_string(&Command::InsertRow {
        table: "CliWritten".into(),
        values: vec![FieldValue::Text("via the cli".into())],
    })
    .expect("serialize");

    // One `exec` per core: `--db` opens the copy, and a core with an open
    // database rejects a second open ("a database is already open").
    for command in [&create, &insert] {
        let mut core = Core::new(Box::new(datacraft_storage::detecting::DetectingPort::new()));
        let mut out: Vec<u8> = Vec::new();
        let outcome = run(
            &mut core,
            &[
                "exec".to_owned(),
                command.to_owned(),
                "--db".to_owned(),
                copy.to_string_lossy().to_string(),
                "--json".to_owned(),
            ],
            &mut out,
        );
        assert_eq!(outcome, Outcome::Ok, "{}", String::from_utf8_lossy(&out));
    }

    // A fresh core reopens the file: the write persisted, the format is ACE.
    let mut reopened = Core::new(Box::new(datacraft_storage::detecting::DetectingPort::new()));
    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut reopened,
        &[
            "open".to_owned(),
            copy.to_string_lossy().to_string(),
            "--json".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("\"format\":\"ace\""),
        "the format stays ACE"
    );
    let rows = reopened
        .read_rows(
            "CliWritten",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("the inserted row persisted");
    assert_eq!(rows, [vec![FieldValue::Text("via the cli".into())]]);
    let _ = std::fs::remove_file(&copy);
}

#[test]
fn exec_reaches_save_as_and_password_change() {
    let mut port = MockPort::new();
    port.with_capabilities(DriverCapabilities::full());
    let mut core = Core::new(Box::new(port));

    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"SaveDatabaseAs\":{\"path\":\"copy.mdb\",\"password\":\"secret\"}}".to_owned(),
            "--json".to_owned(),
            "--db".to_owned(),
            "shop.mdb".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("DatabaseSavedAs"), "{text}");

    let mut out: Vec<u8> = Vec::new();
    let outcome = run(
        &mut core,
        &[
            "exec".to_owned(),
            "{\"ChangePassword\":{\"new_password\":null}}".to_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("PasswordChanged"), "{text}");
}
