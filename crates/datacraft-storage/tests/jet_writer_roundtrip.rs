//! Writer round-trip acceptance (step 10.3, item H9).
//!
//! The H9 acceptance bar: `decode(encode(decode(x))) == decode(x)` over the
//! fixtures, golden round-trips of fully synthetic databases, and — where
//! the oracle CLI exists — `mdb-export` reading a file *we* wrote (the
//! strongest CI-available proxy for "Access opens it without repair"; the
//! real Access check stays a manual gate item, see PROGRESS 10.3).

use std::path::PathBuf;

use datacraft_storage::jet::encode::encode_row;
use datacraft_storage::jet::{Jet4TableReader, Jet4Writer, JetColumn, JetTableDef, JetType};
use datacraft_storage::{StorageValue, jet};

mod support;

/// Deterministic pseudo-random values (LCG) — proptest-style coverage
/// without a new dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn text(&mut self, len: usize) -> String {
        (0..len)
            .map(|_| char::from(b'a' + (self.next() % 26) as u8))
            .collect()
    }
}

/// The full acceptance: every row of every decodable fixture table survives
/// decode → encode → decode unchanged.
#[test]
fn fixture_rows_round_trip_through_reencode() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let reader = Jet4TableReader::new(file.clone()).expect("opens");
    let page_size = reader.header().page_size;
    let mut checked = 0usize;
    let mut tables = 0usize;
    for entry in reader.catalog().expect("catalog") {
        let Some(tdef_page) = entry.tdef_page else {
            continue;
        };
        let Ok(def) = JetTableDef::decode(&file, tdef_page) else {
            continue; // unsupported column shapes (0x11) — BACKLOG L2
        };
        let Ok(data) = reader.read_table(tdef_page) else {
            continue;
        };
        tables += 1;
        for (i, row) in data.rows.iter().enumerate() {
            let encoded =
                encode_row(row, &def).unwrap_or_else(|e| panic!("{}: row {i}: {e}", entry.name));
            let decoded = jet::decode_row(&encoded, &def, &file, page_size)
                .unwrap_or_else(|e| panic!("{}: row {i}: {e}", entry.name));
            assert_eq!(decoded, *row, "{}: row {i} must round-trip", entry.name);
            checked += 1;
        }
    }
    assert!(tables >= 5, "fixture must expose several tables: {tables}");
    assert!(checked >= 50, "fixture must expose many rows: {checked}");
    eprintln!("round-trip: {checked} rows across {tables} tables");
}

/// Golden: a fully synthetic database (own catalog, no fixture) carries a
/// 25-row table of every type and reads back identically.
#[test]
fn synthetic_database_golden_round_trip() {
    let file = support::synthetic_catalog_db();
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "Golden",
            &[
                JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 60),
                JetColumn::new("Score", JetType::Double, 0x03, 2, 4, 8),
                JetColumn::new("Flag", JetType::Bool, 0x01, 3, 12, 0),
            ],
        )
        .expect("created");

    let mut rng = Lcg(0x5EED);
    let mut expected = Vec::new();
    for i in 0..25u32 {
        let name = if i % 7 == 0 {
            StorageValue::Null
        } else {
            StorageValue::Text(rng.text(1 + (i as usize % 12)))
        };
        let row = vec![
            StorageValue::Int(i64::from(i)),
            name,
            StorageValue::Float(rng.next() as f64 / 1e6),
            StorageValue::Bit(i % 3 != 0),
        ];
        writer.insert_row(tdef, &row).expect("inserts");
        expected.push(row);
    }
    let written = writer.into_file();

    let reader = Jet4TableReader::new(written).expect("written db opens");
    let data = reader.open_table("Golden").expect("golden table reads");
    assert_eq!(data.rows.len(), expected.len());
    for (want, got) in expected.iter().zip(data.rows.iter()) {
        assert_eq!(want, got);
    }
    // The catalog grew by exactly the new table's row (read via the TDEF
    // page directly — the synthetic catalog has no self-referencing row).
    assert_eq!(
        reader
            .read_table(datacraft_storage::jet::CATALOG_TDEF_PAGE)
            .expect("catalog")
            .rows
            .len(),
        1,
        "synthetic catalog starts empty and gains the new table's row"
    );
}

/// The oracle reads a file we wrote: `mdb-export` on a fixture copy that
/// carries a new table must produce the rows we inserted (ubuntu/macOS CI,
/// where mdbtools exists).
#[test]
fn oracle_reads_the_file_we_write() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    if !oracle_available() {
        eprintln!("skipped: mdb-export not installed");
        return;
    }
    let file = std::fs::read(path).expect("fixture readable");
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "OracleProbe",
            &[
                JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 40),
                JetColumn::new("Score", JetType::Double, 0x03, 2, 4, 8),
            ],
        )
        .expect("created");
    let rows = vec![
        vec![
            StorageValue::Int(1),
            StorageValue::Text("alpha".into()),
            StorageValue::Float(1.5),
        ],
        vec![
            StorageValue::Int(2),
            StorageValue::Null,
            StorageValue::Float(-0.25),
        ],
        vec![
            StorageValue::Int(3),
            StorageValue::Text("gamma".into()),
            StorageValue::Float(42.0),
        ],
    ];
    for row in &rows {
        writer.insert_row(tdef, row).expect("inserts");
    }
    let out = temp_file("datacraft-oracle-probe.mdb");
    std::fs::write(&out, writer.into_file()).expect("temp write");

    let export = std::process::Command::new("mdb-export")
        .arg(&out)
        .arg("OracleProbe")
        .output()
        .expect("mdb-export runs");
    assert!(
        export.status.success(),
        "mdb-export must succeed on our file: {}",
        String::from_utf8_lossy(&export.stderr)
    );
    let csv = String::from_utf8_lossy(&export.stdout);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "Id,Name,Score", "header: {csv}");
    assert_eq!(lines.len(), 4, "row count: {csv}");
    assert_eq!(lines[1], "1,alpha,1.5", "row 1: {csv}");
    assert_eq!(lines[2], "2,,-0.25", "row 2 (NULL name): {csv}");
    assert_eq!(lines[3], "3,gamma,42", "row 3: {csv}");
    let _ = std::fs::remove_file(&out);
}

/// Writer error paths stay precise: allocation rejects non-TDEF pages and
/// implausible (freed) TDEF pages.
#[test]
fn allocation_rejects_non_tables_precisely() {
    let file = support::synthetic_catalog_db();
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let err = writer.allocate_page(1).unwrap_err(); // page 1: global map page
    assert!(
        matches!(err, datacraft_storage::StorageError::Corrupt { .. }),
        "{err}"
    );
}

fn oracle_available() -> bool {
    std::process::Command::new("mdb-export")
        .arg("--help")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stderr.is_empty())
}

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}
