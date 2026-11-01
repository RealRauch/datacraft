//! CLI password plumbing (step 16.2, item M5): the flag reaches the same
//! `OpenDatabase` command the UI and MCP use.

use std::io::Cursor;

use datacraft_cli::{Outcome, run_with_streams};
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;

fn core_with_password() -> Core {
    let mut port = MockPort::new();
    port.with_password("secret");
    Core::new(Box::new(port))
}

fn run(core: &mut Core, args: &[&str]) -> (Outcome, String) {
    let mut output: Vec<u8> = Vec::new();
    let outcome = run_with_streams(
        core,
        &args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
        &mut output,
        &mut Cursor::new(String::new()),
    );
    (outcome, String::from_utf8(output).expect("utf-8 output"))
}

#[test]
fn the_open_flag_passes_the_password() {
    let mut core = core_with_password();
    let (outcome, text) = run(
        &mut core,
        &["open", "shop.mdb", "-p", "secret", "--json"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("DatabaseOpened"), "{text}");
}

#[test]
fn a_missing_password_fails_the_command() {
    let mut core = core_with_password();
    let (outcome, text) = run(&mut core, &["open", "shop.mdb", "--json"]);
    assert_eq!(outcome, Outcome::CommandFailed, "{text}");
    assert!(text.contains("PasswordRequired"), "{text}");
}

#[test]
fn a_wrong_password_fails_the_command() {
    let mut core = core_with_password();
    let (outcome, text) = run(
        &mut core,
        &["open", "shop.mdb", "--password", "wrong", "--json"],
    );
    assert_eq!(outcome, Outcome::CommandFailed, "{text}");
    assert!(text.contains("PasswordRejected"), "{text}");
}

#[test]
fn exec_json_carries_the_password() {
    let mut core = core_with_password();
    let (outcome, text) = run(
        &mut core,
        &[
            "exec",
            r#"{"OpenDatabase":{"path":"shop.mdb","password":"secret"}}"#,
            "--json",
        ],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("DatabaseOpened"), "{text}");
}

#[test]
fn the_repl_pre_open_accepts_a_password() {
    let mut core = core_with_password();
    let mut output: Vec<u8> = Vec::new();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}\n";
    let outcome = run_with_streams(
        &mut core,
        &[
            "repl".to_owned(),
            "--db".to_owned(),
            "shop.mdb".to_owned(),
            "--password".to_owned(),
            "secret".to_owned(),
        ],
        &mut output,
        &mut Cursor::new(input.to_owned()),
    );
    assert_eq!(outcome, Outcome::Ok);
    let text = String::from_utf8(output).expect("utf-8 output");
    assert!(text.contains("\"database\""), "{text}");
}
