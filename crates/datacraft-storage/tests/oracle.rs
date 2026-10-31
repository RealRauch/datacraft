//! libmdb (MDB Tools CLI) oracle comparison (step 2.4, item H7).
//!
//! Gated by `DATACRAFT_ORACLE_MDB` pointing at a pinned fixture. Our side
//! (catalog, schema, row counts) runs whenever the fixture is present; the
//! `mdb-export` comparison only runs where the CLI is installed (CI
//! ubuntu/macOS — see the `oracle` job). Never a runtime dependency.

use std::process::Command;

use datacraft_storage::jet::Jet4TableReader;

fn fixture_path() -> Option<String> {
    std::env::var("DATACRAFT_ORACLE_MDB").ok()
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

    if !mdb_export_available() {
        eprintln!("oracle value comparison skipped: mdb-export not installed");
        return;
    }

    for name in &names {
        let out = Command::new("mdb-export")
            .args([&path.clone(), name])
            .output()
            .expect("mdb-export runs");
        assert!(
            out.status.success(),
            "mdb-export failed for {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let csv = String::from_utf8_lossy(&out.stdout);
        let mut lines = csv.lines();
        let header = parse_csv_line(lines.next().expect("csv header"));
        let oracle_rows = lines.count();
        let table = reader.open_table(name).expect("our read");
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
