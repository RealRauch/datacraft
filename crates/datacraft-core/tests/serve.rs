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
    assert!(
        lines[0].contains("\"nav_pane_visible\":true"),
        "{}",
        lines[0]
    );
    assert!(lines[1].contains("NavPaneToggled"), "{}", lines[1]);
}

#[test]
fn malformed_lines_get_error_responses_without_killing_the_session() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = format!("not json\n{}", request("inspect", "", 7));
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("-32700"), "{}", lines[0]);
    assert!(lines[1].contains("\"id\":7"), "{}", lines[1]);
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
    let input = request("dispatch", "{\"OpenDatabase\":{\"path\":\"shop.mdb\"}}", 11);
    let mut output: Vec<u8> = Vec::new();
    serve(&mut Cursor::new(input), &mut output, &mut core);
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("DatabaseOpened"), "{text}");
    assert!(text.contains("\"name\":\"shop\""), "{text}");
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
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[1].contains("\"database\""),
        "state carries over: {}",
        lines[1]
    );
    let _ = PathBuf::new();
    let _ = Command::CloseDatabase;
}

/// A writer that fails after N bytes (the broken-pipe path).
struct FailingWriter {
    written: usize,
    fail_at: usize,
}

impl std::io::Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.written + buf.len() > self.fail_at {
            Err(std::io::Error::other("broken pipe"))
        } else {
            self.written += buf.len();
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A reader that errors on the first read (the broken-input path).
struct BrokenReader;

impl std::io::Read for BrokenReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("device gone"))
    }
}

impl std::io::BufRead for BrokenReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("device gone"))
    }
    fn consume(&mut self, _amt: usize) {}
}

#[test]
fn a_broken_writer_ends_the_session_without_panicking() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let input = request("inspect", "", 1);
    let mut writer = FailingWriter {
        written: 0,
        fail_at: 4,
    };
    serve(&mut Cursor::new(input), &mut writer, &mut core);
}

#[test]
fn a_broken_reader_ends_the_session_without_panicking() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let mut output: Vec<u8> = Vec::new();
    serve(&mut BrokenReader, &mut output, &mut core);
    assert!(output.is_empty());
}
