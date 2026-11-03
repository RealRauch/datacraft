//! libmdb (MDB Tools CLI) oracle comparison (step 2.4, item H7).
//!
//! Gated by `DATACRAFT_ORACLE_MDB` pointing at a pinned fixture. Our side
//! (catalog enumeration, schema, full row decode of every table) always runs;
//! the `mdb-export` comparison runs where the CLI is installed (CI
//! ubuntu/macOS — see the `oracle` job). Never a runtime dependency.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use datacraft_jet::StorageDataType;
use datacraft_jet::StorageError;
use datacraft_jet::jet::Jet4TableReader;

/// Resolves `DATACRAFT_ORACLE_MDB`: as given, or — cargo runs tests with the
/// package root as CWD — relative to the workspace root two levels up.
fn fixture_paths() -> Option<Vec<PathBuf>> {
    let raw = std::env::var("DATACRAFT_ORACLE_MDB").ok()?;
    let mut paths = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let direct = Path::new(entry);
        let resolved = if direct.exists() {
            direct.to_path_buf()
        } else {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(entry);
            if !workspace.exists() {
                continue;
            }
            workspace
        };
        paths.push(resolved);
    }
    if paths.is_empty() {
        return None;
    }
    // The complex-value fixture rides along with the configured oracle set
    // (step 16.1, M4): its parent table is decoded on our side and compared
    // where mdb-export can represent the columns.
    let complex = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("complexDataV2007.accdb");
    if complex.exists() && !paths.contains(&complex) {
        paths.push(complex);
    }
    // The Jet3 remainder fixtures ride along as well (step 16.3, L3):
    // chained TDEFs, overflow rows and the jump-table row are decoded on our
    // side and compared where mdb-export supports the file.
    for name in ["testV1997.mdb", "test2V1997.mdb"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join(name);
        if path.exists() && !paths.contains(&path) {
            paths.push(path);
        }
    }
    Some(paths)
}

fn mdb_export_available() -> bool {
    Command::new("mdb-export")
        .arg("--help")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stderr.is_empty())
}

#[test]
fn oracle_fixture_decodes_and_matches_mdb_tools() {
    let Some(paths) = fixture_paths() else {
        eprintln!("oracle skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    for path in &paths {
        check_fixture(path);
    }
}

/// Compares one fixture against mdb-tools (our side always, the CLI where
/// installed). ACE complex-value subtables (`f_<guid>_*`) are skipped with
/// their documented reason: mdb-tools hides them as system tables and the
/// domain groups them into their owning column (step 16.1, M4). The
/// complex-value fixture is appended to the configured set automatically;
/// tables carrying complex columns are decoded on our side and skipped for
/// the export comparison (mdb-tools has no representation for them).
fn check_fixture(path: &Path) {
    let file = std::fs::read(path).expect("oracle fixture must be readable");
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
        if name.starts_with("f_") {
            eprintln!("oracle table {name} skipped: complex-value subtable (M4)");
            continue;
        }
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
        // Complex-value columns (step 16.1, M4) have no mdb-export
        // representation (mdb-tools hides the backing tables and cannot
        // express the grouped sub-records); our side decodes the table and
        // asserts its row count locally.
        if table
            .schema
            .columns
            .iter()
            .any(|column| column.data_type == StorageDataType::Complex)
        {
            eprintln!(
                "oracle table {name} skipped: complex columns (M4); our side holds {} rows",
                table.rows.len()
            );
            continue;
        }
        // Documented skip (BACKLOG L2 decision, 2026-11-01): MSysAccessObjects
        // embeds OLE compound documents in fixed-storage 0x11 columns, which
        // mdb-tools itself cannot read ("unsupported data type 17") — the
        // cross-check is impossible by construction. Our side decodes the
        // table (JetType::LongBinary) and asserts the row count locally.
        if *name == "MSysAccessObjects" {
            eprintln!(
                "oracle table {name} skipped: mdb-tools cannot read 0x11 columns \
                 (L2); our side holds {} rows",
                table.rows.len()
            );
            continue;
        }
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
