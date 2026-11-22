//! SQLite SQL pushdown over the real CLI repl (phase 98, step 98.3; item
//! M129): the shipped binary names the native dialect, runs a statement
//! the shadow engine cannot (window function), accepts an EXPLAIN QUERY
//! PLAN statement and refuses a write with the capability code. (The
//! wire carries the row count and the snapshot; the plan and result rows
//! stay in-process for the UI grid, which the port tests pin.)

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command as CliCommand;
use datacraft_core::command::Command;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, IndexMode, NumericKind, TableDef};
use predicates::prelude::*;
use serde_json::json;

fn bin() -> CliCommand {
    CliCommand::cargo_bin("datacraft-cli").expect("the CLI binary builds")
}

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-cli-sql-pushdown-{}-{n}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn line(id: u32, command: Command) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "dispatch",
        "params": serde_json::to_value(command).expect("command json"),
    })
    .to_string()
        + "\n"
}

fn table() -> TableDef {
    let mut name = FieldDef::new("name", FieldKind::Text { max_len: 50 });
    name.properties.indexed = IndexMode::DuplicatesOk;
    TableDef::new(
        "items",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            name,
        ],
    )
}

#[test]
fn the_repl_runs_native_sql_plans_and_refuses_writes() {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let create = json!({"CreateDatabase": {"path": path.to_string_lossy(), "format": "sqlite"}});
    bin()
        .args(["exec", &create.to_string(), "--json"])
        .assert()
        .success();
    let input = [
        line(
            1,
            Command::CreateTable {
                definition: table(),
            },
        ),
        line(
            2,
            Command::InsertRow {
                table: "items".into(),
                values: vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            },
        ),
        line(
            3,
            Command::InsertRow {
                table: "items".into(),
                values: vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
            },
        ),
        line(
            4,
            Command::RunSql {
                sql: "SELECT ROW_NUMBER() OVER (ORDER BY id) AS n, name FROM items ORDER BY id"
                    .into(),
            },
        ),
        line(
            5,
            Command::RunSql {
                sql: "EXPLAIN QUERY PLAN SELECT * FROM items WHERE name = 'Ada'".into(),
            },
        ),
        line(
            6,
            Command::RunSql {
                sql: "DELETE FROM items".into(),
            },
        ),
        line(
            7,
            Command::RunSql {
                sql: "SELECT COUNT(*) AS n FROM items".into(),
            },
        ),
        json!({"jsonrpc": "2.0", "id": 8, "method": "inspect"}).to_string() + "\n",
    ]
    .concat();
    bin()
        .args(["repl", "--db", &path.to_string_lossy()])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""SqlRan":{"rows":2}"#))
        .stdout(predicate::str::contains(r#""SqlRan":{"rows":1}"#))
        .stdout(predicate::str::contains(r#""sql_dialect":"native""#))
        .stdout(predicate::str::contains(r#""code":"unsupported""#))
        .stderr(predicate::str::is_empty());
    assert!(scratch.0.exists());
}
