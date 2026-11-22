//! Excel as an Import File source over the real CLI process (phase 96,
//! step 96.4; item M127): the composition root registers the reader, so a
//! user-visible import works in the shipped binary, not only in tests.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("datacraft-cli").expect("the CLI binary builds")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-cli-excel-import-{}-{n}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn a_workbook_imports_through_the_shipped_binary() {
    let path = scratch_path();
    let _scratch = Scratch(path.clone());
    let create = format!(
        "{{\"CreateDatabase\":{{\"path\":{},\"format\":\"jet4\"}}}}",
        serde_json::to_string(&path.to_string_lossy()).expect("path json")
    );
    bin().args(["exec", &create, "--json"]).assert().success();
    let source = fixture("excelFixture.xlsx").to_string_lossy().into_owned();
    let import = format!(
        "{{\"ImportFile\":{{\"source\":{},\"table\":\"Imported\"}}}}",
        serde_json::to_string(&source).expect("source json")
    );
    bin()
        .args([
            "exec",
            &import,
            "--db",
            path.to_str().expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TableCreated"))
        .stdout(predicate::str::contains("RowInserted"));
    bin()
        .args(["open", path.to_str().expect("utf-8 path"), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name":"Imported""#))
        .stdout(predicate::str::contains(r#""name":"Active""#));
}
