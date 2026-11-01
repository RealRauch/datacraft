//! Contract suite for the stdio serve loop (step 9.1, item H32).
//!
//! The MCP endpoint is a thin line protocol over the JSON-RPC codec: one
//! request line in, one response line out, EOF ends the session. Tested
//! over in-memory streams — no real pipes needed.

use std::io::Cursor;
use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::transport::serve;

fn request(method: &str, params: &str, id: u64) -> String {
    if params.is_empty() {
        format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\"}}\n")
    } else {
        format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{params}}}\n")
    }
}

#[test]
fn serve_answers_every_line_and_stops_at_eof() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = format!(
        "{}{}",
        request("inspect", "", 1),
        request("dispatch", "{\"ToggleNavPane\":null}", 2)
    );
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);

    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "one response per request: {text}");
    assert!(lines[0].contains("\"nav_pane_visible\": true"), "{lines[0]}");
    assert!(lines[1].contains("NavPaneToggled"), "{lines[1]}");
}

#[test]
fn malformed_lines_get_error_responses_without_killing_the_session() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = format!(
        "not json\n{}",
        request("inspect", "", 7)
    );
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("-32700"), "{lines[0]}");
    assert!(lines[1].contains("\"id\":7"), "{lines[1]}");
}

#[test]
fn blank_lines_are_skipped_silently() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = format!("\n\n{}", request("inspect", "", 3));
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let lines = String::from_utf8(output).unwrap().lines().count();
    assert_eq!(lines, 1, "blank lines produce no responses");
}

#[test]
fn end_to_end_open_over_the_loop() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = request(
        "dispatch",
        "{\"OpenDatabase\":{\"path\":\"shop.mdb\"}}",
        11,
    );
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("DatabaseOpened"), "{text}");
    assert!(text.contains("\"name\": \"mock\""), "{text}");
}

#[test]
fn a_dispatched_open_is_observable_in_the_following_inspect() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = format!(
        "{}{}",
        request("dispatch", "{\"OpenDatabase\":{\"path\":\"shop.mdb\"}}", 1),
        request("inspect", "", 2)
    );
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let lines: Vec<&str> = String::from_utf8(output).unwrap().lines().collect();
    assert!(lines[1].contains("\"database\""), "state carries over: {}", lines[1]);
    let _ = PathBuf::new();
    let _ = Command::CloseDatabase;
}
