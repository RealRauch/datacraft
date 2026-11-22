//! PDF export over the real MCP process (phase 97, step 97.3; item M128).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
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

fn target() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("datacraft-mcp-pdf-{}-{n}.pdf", std::process::id()))
}

fn line(id: u32, method: &str, params: Option<serde_json::Value>) -> String {
    let value = match params {
        Some(params) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        None => json!({"jsonrpc": "2.0", "id": id, "method": method}),
    };
    value.to_string() + "\n"
}

#[test]
fn the_endpoint_exports_a_report_pdf() {
    let source = serde_json::to_string(&fixture().to_string_lossy()).expect("source json");
    let target = target();
    let destination = serde_json::to_string(&target.to_string_lossy()).expect("path json");
    let input = [
        line(
            1,
            "dispatch",
            Some(
                serde_json::from_str(&format!(
                    "{{\"OpenDatabase\":{{\"path\":{source},\"password\":null}}}}"
                ))
                .expect("open"),
            ),
        ),
        line(
            2,
            "dispatch",
            Some(json!({"CreateReport": {"name": "Wire"}})),
        ),
        line(
            3,
            "dispatch",
            Some(
                serde_json::from_str(&format!(
                    "{{\"ExportReportPdf\":{{\"report\":\"Wire\",\"path\":{destination}}}}}"
                ))
                .expect("export"),
            ),
        ),
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("ReportExported"))
        .stderr(predicate::str::is_empty());
    assert!(
        std::fs::read(&target)
            .expect("the PDF exists")
            .starts_with(b"%PDF-")
    );
    let _ = std::fs::remove_file(&target);
}
