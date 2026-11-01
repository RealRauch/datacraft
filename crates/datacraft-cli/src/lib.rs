//! DataCraft CLI (step 9.4, item H8) — the fourth command-bus client.
//!
//! Transport only: args map onto commands, output is deterministic
//! (human or `--json`), exit codes are meaningful, and `exec` reaches the
//! full command language as JSON. The `repl` shares the JSON-RPC
//! transport with the MCP endpoint.

use std::io::Write;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::transport;

/// Process exit code semantics of a CLI run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The command ran (failures inside the bus still print their reason).
    Ok,
    /// A command dispatched but the bus rejected it (`Event::Failed`).
    CommandFailed,
    /// The invocation itself was malformed.
    UsageError,
}

impl Outcome {
    /// The process exit code.
    pub fn code(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::CommandFailed => 2,
            Self::UsageError => 1,
        }
    }
}

/// Runs one CLI invocation against the core, writing output to `out`
/// (the binary passes the real stdout; `repl` reads real stdin).
pub fn run(core: &mut Core, args: &[String], out: &mut dyn Write) -> Outcome {
    run_with_streams(core, args, out, &mut std::io::stdin().lock())
}

/// Stream-injected variant so every arm is testable in-process.
pub fn run_with_streams(
    core: &mut Core,
    args: &[String],
    out: &mut dyn Write,
    reader: &mut dyn std::io::BufRead,
) -> Outcome {
    let Some(command) = args.first() else {
        return usage(
            out,
            "usage: datacraft-cli (open <path> | exec <json> | repl) [--json] \
             [--db <path>] [-p <password>]",
        );
    };
    let json_output = args.iter().any(|a| a == "--json");
    let password = flag_value(args, &["-p", "--password"]);
    let db_path = args
        .iter()
        .position(|a| a == "--db")
        .and_then(|i| args.get(i + 1))
        .cloned();

    match command.as_str() {
        "open" => {
            let Some(path) = args.get(1).filter(|p| !p.as_str().starts_with("--")) else {
                return usage(out, "usage: datacraft-cli open <path> [-p <password>]");
            };
            run_command(
                core,
                open_command(path, password.as_deref()),
                json_output,
                out,
            )
        }
        "exec" => {
            let Some(json) = args.get(1) else {
                return usage(out, "usage: datacraft-cli exec '<json command>'");
            };
            let Ok(command) = serde_json::from_str::<Command>(json) else {
                return usage(out, "invalid command JSON");
            };
            let mut events = Vec::new();
            if let Some(path) = db_path {
                events.extend(core.dispatch(open_command(&path, password.as_deref())));
                if failed(&events) {
                    print_events(core, &events, json_output, out);
                    return Outcome::CommandFailed;
                }
            }
            events.extend(core.dispatch(command));
            print_events(core, &events, json_output, out);
            if failed(&events) {
                Outcome::CommandFailed
            } else {
                Outcome::Ok
            }
        }
        "repl" => {
            if let Some(path) = db_path {
                core.dispatch(open_command(&path, password.as_deref()));
            }
            repl(core, reader, out);
            Outcome::Ok
        }
        other => usage(out, &format!("unknown command {other:?}")),
    }
}

fn run_command(
    core: &mut Core,
    command: Command,
    json_output: bool,
    out: &mut dyn Write,
) -> Outcome {
    let events = core.dispatch(command);
    print_events(core, &events, json_output, out);
    if failed(&events) {
        Outcome::CommandFailed
    } else {
        Outcome::Ok
    }
}

fn print_events(
    core: &Core,
    events: &[datacraft_core::event::Event],
    json_output: bool,
    out: &mut dyn Write,
) {
    let has_failure = failed(events);
    if json_output {
        let payload = serde_json::json!({
            "events": events,
            "snapshot": core.inspect(),
        });
        let _ = writeln!(out, "{payload}");
    } else {
        for event in events {
            let _ = writeln!(out, "{event:?}");
        }
        let status = if has_failure { "failed" } else { "ok" };
        let _ = writeln!(out, "{status}");
    }
}

/// Whether the events include a rejected command (including the dedicated
/// password events, step 16.2 / M5).
fn failed(events: &[datacraft_core::event::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            datacraft_core::event::Event::Failed { .. }
                | datacraft_core::event::Event::PasswordRequired { .. }
                | datacraft_core::event::Event::PasswordRejected { .. }
        )
    })
}

/// Value of a `-p`/`--password` flag (the first occurrence wins).
fn flag_value(args: &[String], flags: &[&str]) -> Option<String> {
    args.iter()
        .position(|arg| flags.contains(&arg.as_str()))
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn usage(out: &mut dyn Write, message: &str) -> Outcome {
    let _ = writeln!(out, "{message}");
    Outcome::UsageError
}

/// Line-based JSON-RPC session over arbitrary streams (shared with the
/// MCP endpoint's transport).
pub fn repl(core: &mut Core, reader: &mut dyn std::io::BufRead, writer: &mut dyn Write) {
    transport::serve(reader, writer, core);
}

fn open_command(path: &str, password: Option<&str>) -> Command {
    serde_json::from_value(
        serde_json::json!({"OpenDatabase": {"path": path, "password": password}}),
    )
    .expect("the open command shape is fixed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use datacraft_core::port::MockPort;

    /// Open-command JSON stays in the command vocabulary (transport only).
    #[test]
    fn open_deserializes_into_the_command_language() {
        let command = open_command("shop.mdb", None);
        assert!(matches!(command, Command::OpenDatabase { .. }));
    }

    /// Exit codes are part of the headless contract.
    #[test]
    fn outcome_codes_are_meaningful() {
        assert_eq!(Outcome::Ok.code(), 0);
        assert_eq!(Outcome::CommandFailed.code(), 2);
        assert_eq!(Outcome::UsageError.code(), 1);
    }

    /// Repl honors --db (the pre-open lands before the session starts).
    #[test]
    fn repl_opens_the_database_first() {
        use std::io::Cursor;

        let mut core = Core::new(Box::new(MockPort::new()));
        let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}\n";
        let mut output: Vec<u8> = Vec::new();
        let outcome = run_with_streams(
            &mut core,
            &["repl".to_owned(), "--db".to_owned(), "shop.mdb".to_owned()],
            &mut output,
            &mut Cursor::new(input.to_owned()),
        );
        assert_eq!(outcome, Outcome::Ok);
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("\"database\""), "{text}");
    }
}
