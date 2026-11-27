//! The app-readiness report over the real CLI process (phase 103, step
//! 103.2; item M134): `exec '"AnalyzeApp"'` reaches the bus and the JSON
//! payload carries the full report.

use std::path::{Path, PathBuf};

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

#[test]
fn exec_analyze_app_answers_the_full_report() {
    bin()
        .args([
            "exec",
            "\"AnalyzeApp\"",
            "--db",
            fixture("programmability.accdb")
                .to_str()
                .expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("\"AppAnalyzed\""))
        .stdout(predicate::str::contains("\"objects\":"))
        .stdout(predicate::str::contains("\"effort\":"))
        .stdout(predicate::str::contains("\"vba\":"))
        .stdout(predicate::str::contains("\"transplicable_percent\""))
        .stdout(predicate::str::contains("\"unsupported\":"));
}

#[test]
fn exec_analyze_app_without_a_database_fails_with_the_precise_reason() {
    bin()
        .args(["exec", "\"AnalyzeApp\"", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"AppAnalyzed\"").not())
        .stdout(predicate::str::contains("\"code\":\"no_database\""));
}
