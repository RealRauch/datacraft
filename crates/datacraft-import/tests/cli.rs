//! Real-subprocess tests for `datacraft-import` (step 78.2, item M88).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use datacraft_core::form::FormDef;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, SchemaDocument, TableDef};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/programmability.accdb")
        .canonicalize()
        .expect("the committed fixture is present")
}

fn expected() -> SchemaDocument {
    SchemaDocument::from_json(include_str!("data/programmability.datacraft.json"))
        .expect("the expected sidecar is valid")
}

fn scratch(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "datacraft-import-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create the scratch directory");
    path
}

fn run(args: &[&Path]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_datacraft-import"));
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("run the import binary")
}

#[test]
fn imports_into_an_explicit_sidecar_and_leaves_the_database_alone() {
    let scratch = scratch("basic");
    let sidecar = scratch.join("programmability.datacraft");
    let before = std::fs::read(fixture()).expect("read the fixture");
    let output = run(&[&fixture(), Path::new("--sidecar"), &sidecar]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("imported 4 objects"), "{stdout}");
    assert_eq!(
        SchemaDocument::load(&sidecar).expect("the sidecar loads"),
        expected()
    );
    let after = std::fs::read(fixture()).expect("read the fixture again");
    assert_eq!(before, after, "the database must stay byte-identical");
}

#[test]
fn json_output_carries_the_sidecar_and_the_report() {
    let scratch = scratch("json");
    let sidecar = scratch.join("programmability.datacraft");
    let output = run(&[
        &fixture(),
        Path::new("--sidecar"),
        &sidecar,
        Path::new("--json"),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("standard output is JSON");
    assert_eq!(value["sidecar"], sidecar.display().to_string());
    assert_eq!(value["report"]["objects"].as_array().map(Vec::len), Some(4));
}

#[test]
fn strict_succeeds_for_the_clean_fixture() {
    let scratch = scratch("strict");
    let sidecar = scratch.join("programmability.datacraft");
    let output = run(&[
        &fixture(),
        Path::new("--sidecar"),
        &sidecar,
        Path::new("--strict"),
    ]);
    assert!(output.status.success());
}

#[test]
fn merge_keeps_tables_and_unrelated_designs() {
    let scratch = scratch("merge");
    let sidecar = scratch.join("programmability.datacraft");
    let mut existing = SchemaDocument::new("programmability");
    existing.tables = vec![TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "ID",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )];
    existing.forms = vec![FormDef::new("Keep Me"), FormDef::new("Items Browser")];
    existing.save(&sidecar).expect("write the existing sidecar");

    let output = run(&[&fixture(), Path::new("--sidecar"), &sidecar]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let merged = SchemaDocument::load(&sidecar).expect("the merged sidecar loads");
    assert_eq!(merged.tables.len(), 1, "the table survives");
    assert!(
        merged.forms.iter().any(|form| form.name == "Keep Me"),
        "an unrelated form survives"
    );
    let imported = expected()
        .forms
        .into_iter()
        .find(|form| form.name == "Items Browser")
        .expect("the expected form");
    let replaced = merged
        .forms
        .iter()
        .find(|form| form.name == "Items Browser")
        .expect("the imported form");
    assert_eq!(replaced, &imported, "the same-named form is replaced");
}
