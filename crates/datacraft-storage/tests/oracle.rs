//! libmdb (MDB Tools CLI) oracle comparison (step 2.4, item H7).
//!
//! Gated by `DATACRAFT_ORACLE_MDB` pointing at a pinned fixture. Our side
//! (catalog enumeration, schema, full row decode of every table) always runs;
//! the `mdb-export` comparison runs where the CLI is installed (CI
//! ubuntu/macOS — see the `oracle` job). Never a runtime dependency.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use datacraft_storage::StorageError;
use datacraft_storage::jet::Jet4TableReader;

/// Resolves `DATACRAFT_ORACLE_MDB`: as given, or — cargo runs tests with the
/// package root as CWD — relative to the workspace root two levels up.
fn fixture_path() -> Option<PathBuf> {
    let raw = std::env::var("DATACRAFT_ORACLE_MDB").ok()?;
    let direct = Path::new(&raw);
    if direct.exists() {
        return Some(direct.to_path_buf());
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(&raw);
    workspace.exists().then_some(workspace)
}

fn mdb_export_available() -> bool {
    Command::new("mdb-export")
        .arg("--help")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stderr.is_empty())
}

#[test]
fn oracle_fixture_decodes_and_matches_mdb_tools() {
    let Some(path) = fixture_path() else {
        eprintln!("oracle skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(&path).expect("oracle fixture must be readable");
    let reader = Jet4TableReader::new(file).expect("our decoder opens the real fixture");
    let names = reader.table_names().expect("catalog enumeration");
    eprintln!("oracle tables: {names:?}");
    assert!(!names.is_empty(), "fixture must contain user tables");

    // Every resolvable catalog table must decode or fail precisely: tables
    // whose columns use types we deliberately do not support yet (e.g. the
    // undocumented 0x11 of MSysAccessObjects — mdb-tools warns "unsupported
    // data type 17" on the same column) are skipped with their reason, every
    // other error is a failure.
    let mut tables = Vec::new();
    for name in &names {
        match reader.open_table(name) {
            Ok(table) => tables.push((name, table)),
            Err(StorageError::Unsupported { reason }) => {
                eprintln!("oracle table {name} skipped: {reason}");
            }
            Err(err) => panic!("open_table({name}) failed: {err:?}"),
        }
    }

    if !mdb_export_available() {
        eprintln!("oracle record comparison skipped: mdb-export not installed");
        for (name, table) in &tables {
            eprintln!("oracle table {name}: {} rows (our side)", table.rows.len());
        }
        return;
    }

    for (name, table) in &tables {
        let out = Command::new("mdb-export")
            .args([path.as_os_str(), OsStr::new(*name)])
            .output()
            .expect("mdb-export runs");
        assert!(
            out.status.success(),
            "mdb-export failed for {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Records, not lines: quoted OLE property blobs carry raw newlines
        // (230 of them in the MSysObjects export of the touched fixture).
        let records = csv_records(&String::from_utf8_lossy(&out.stdout));
        let header = records
            .first()
            .map(|r| parse_csv_line(r))
            .expect("csv header");
        let oracle_rows = records.len() - 1;
        let our_cols: Vec<String> = table
            .schema
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(our_cols, header, "column names/order for table {name}");
        assert_eq!(table.rows.len(), oracle_rows, "row count for table {name}");
        eprintln!("oracle table {name}: {} rows match", oracle_rows);
    }
}

/// Splits CSV output into records. Fields quoted with double quotes may span
/// physical lines (mdb-export embeds raw newlines in binary property values),
/// so records end only at a newline outside quotes.
fn csv_records(output: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in output.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            '\r' if !in_quotes => {}
            '\n' if !in_quotes => records.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        records.push(current);
    }
    records
}

/// Minimal quoted-CSV line parser (mdb-export output: comma-separated,
/// double-quote escaping).
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => fields.push(std::mem::take(&mut current)),
                _ => current.push(c),
            }
        }
    }
    fields.push(current);
    fields
}
