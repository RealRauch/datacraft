//! PDF export over the real CLI repl (phase 97, step 97.3; item M128):
//! the shipped binary carries the writer and the same command vocabulary.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;

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

fn target() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("datacraft-cli-pdf-{}-{n}.pdf", std::process::id()))
}

fn line(id: u32, command: serde_json::Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": "dispatch", "params": command}).to_string() + "\n"
}

#[test]
fn the_repl_exports_a_report_pdf() {
    let target = target();
    let path = serde_json::to_string(&target.to_string_lossy()).expect("path json");
    let input = [
        line(1, json!({"CreateReport": {"name": "Wire"}})),
        line(
            2,
            json!({"AddReportControl": {
                "report": "Wire",
                "control": {
                    "name": "Title",
                    "kind": {"type": "label", "value": {"text": "Wire Report"}},
                    "rect": {"x": 36.0, "y": 36.0, "width": 300.0, "height": 24.0}
                }
            }}),
        ),
        line(
            3,
            serde_json::from_str(&format!(
                "{{\"ExportReportPdf\":{{\"report\":\"Wire\",\"path\":{path}}}}}"
            ))
            .expect("command"),
        ),
    ]
    .concat();
    bin()
        .args(["repl", "--db", &fixture().to_string_lossy()])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("ReportExported"))
        .stderr(predicate::str::is_empty());
    let pdf = std::fs::read(&target).expect("the PDF exists");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 1"));
    let _ = std::fs::remove_file(&target);
}
