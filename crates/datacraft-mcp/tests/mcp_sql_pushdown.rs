//! SQLite SQL pushdown over the real MCP process (phase 98, step 98.3;
//! item M129): the endpoint creates and fills the file through commands,
//! runs a native statement, names the dialect and refuses a write.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command as CliCommand;
use datacraft_core::command::Command;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use predicates::prelude::*;
use serde_json::json;

fn bin() -> CliCommand {
    CliCommand::cargo_bin("datacraft-mcp").expect("the MCP binary builds")
}

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-mcp-sql-pushdown-{}-{n}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn dispatch(id: u32, command: Command) -> String {
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
    TableDef::new(
        "items",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

#[test]
fn the_endpoint_runs_native_sql_and_refuses_writes() {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let path_text = path.to_string_lossy().into_owned();
    let input = [
        dispatch(
            1,
            Command::CreateDatabase {
                path: path.clone(),
                format: datacraft_core::port::DatabaseFormat::Sqlite,
            },
        ),
        dispatch(
            2,
            Command::OpenDatabase {
                path: path.clone(),
                password: None,
                source: None,
            },
        ),
        dispatch(
            3,
            Command::CreateTable {
                definition: table(),
            },
        ),
        dispatch(
            4,
            Command::InsertRow {
                table: "items".into(),
                values: vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            },
        ),
        dispatch(
            5,
            Command::RunSql {
                sql: "SELECT ROW_NUMBER() OVER (ORDER BY id) AS n, name FROM items".into(),
            },
        ),
        dispatch(
            6,
            Command::RunSql {
                sql: "EXPLAIN QUERY PLAN SELECT * FROM items".into(),
            },
        ),
        dispatch(
            7,
            Command::RunSql {
                sql: "DELETE FROM items".into(),
            },
        ),
        json!({"jsonrpc": "2.0", "id": 8, "method": "inspect"}).to_string() + "\n",
    ]
    .concat();
    bin()
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""SqlRan":{"rows":1}"#))
        .stdout(predicate::str::contains(r#""sql_dialect":"native""#))
        .stdout(predicate::str::contains(r#""code":"unsupported""#))
        .stdout(predicate::str::contains(path_text.replace('\\', "\\\\")))
        .stderr(predicate::str::is_empty());
    assert!(scratch.0.exists());
}
