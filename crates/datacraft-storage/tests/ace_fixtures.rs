//! Pinned ACE fixture contract (step 14.1, item H38).
//!
//! The three `.accdb` fixtures are pristine downloads (hashes and
//! `repo@commit` provenance in `fixtures/manifest`, verified by the CI
//! fixtures job). This suite proves the pinned set covers ACE 2007, 2010
//! and 2016 and that the real files decode through the header/catalog path.

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, StoragePort};
use datacraft_storage::detecting::DetectingPort;
use datacraft_storage::jet::OBJECT_TYPE_TABLE;
use datacraft_storage::jet::{ACE_2007_VERSION, ACE_2010_VERSION, FileHeader, Jet4TableReader};
use datacraft_storage::{FormatVersion, StorageError};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

/// The pinned ACE set: file, expected version dword, generation label.
fn pinned() -> [(&'static str, u32, &'static str); 3] {
    [
        ("testV2007.accdb", ACE_2007_VERSION, "ACE 2007"),
        ("testV2010.accdb", ACE_2010_VERSION, "ACE 2010"),
        (
            "attachmentsV2016.accdb",
            ACE_2007_VERSION,
            "ACE 2016 (2007 format version)",
        ),
    ]
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).unwrap_or_else(|e| panic!("fixture {name} is pinned: {e}"))
}

#[test]
fn every_pinned_ace_fixture_decodes_with_the_ace_header() {
    for (name, version, generation) in pinned() {
        let file = bytes(name);
        let header = FileHeader::decode(&file, file.len() as u64)
            .unwrap_or_else(|e| panic!("{name} header: {e}"));
        assert_eq!(header.format, FormatVersion::Ace, "{generation}");
        assert_eq!(header.version, version, "{generation}");
        assert_eq!(header.page_size, 4096, "{generation}");
        assert_eq!(header.page_count * 4096, file.len(), "{name} page geometry");
    }
}

#[test]
fn every_pinned_ace_fixture_exposes_user_tables() {
    for (name, _, generation) in pinned() {
        let reader = Jet4TableReader::new(bytes(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let catalog = reader
            .catalog()
            .unwrap_or_else(|e| panic!("{name} catalog: {e}"));
        let user_tables: Vec<&str> = catalog
            .iter()
            .filter(|entry| entry.object_type == OBJECT_TYPE_TABLE)
            .map(|entry| entry.name.as_str())
            .filter(|name| !name.starts_with("MSys"))
            .collect();
        assert!(
            !user_tables.is_empty(),
            "{generation} must carry user tables: {user_tables:?}"
        );
    }
}

#[test]
fn detection_reports_ace_for_every_pinned_fixture() {
    for (name, _, generation) in pinned() {
        let mut port = DetectingPort::new();
        let opened = port
            .open(&fixture(name), None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(opened.format, DatabaseFormat::Ace, "{generation}");
        assert!(!port.tables().is_empty(), "{generation} tables");
    }
}

#[test]
fn ace_pages_are_readable_through_the_jet4_path() {
    // The 2007 fixture is the smallest base file; its first user table
    // must decode rows through the unchanged Jet4 reader.
    let file = bytes("testV2007.accdb");
    let reader = Jet4TableReader::new(file).expect("ACE 2007 opens");
    let table = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .find(|entry| entry.object_type == OBJECT_TYPE_TABLE && !entry.name.starts_with("MSys"))
        .expect("a user table");
    let page = table.tdef_page.expect("the TDEF page is resolved");
    let data = reader
        .read_table(page)
        .unwrap_or_else(|e| panic!("reading {}: {e}", table.name));
    assert!(!data.schema.columns.is_empty());
}

#[test]
fn an_unknown_ace_version_is_a_precise_unsupported() {
    let mut file = bytes("testV2007.accdb");
    file[0x14..0x18].copy_from_slice(&0x0000_00FFu32.to_le_bytes());
    let err = FileHeader::decode(&file, file.len() as u64).expect_err("unknown version");
    match err {
        StorageError::Unsupported { reason } => {
            assert!(reason.contains("0x000000ff"), "{reason}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn every_pinned_ace_table_decodes_rows() {
    let expected = [
        ("testV2007.accdb", "Table1", 2usize),
        ("testV2010.accdb", "Table1", 2usize),
        ("attachmentsV2016.accdb", "Table1", 3usize),
    ];
    for (name, _, generation) in pinned() {
        let mut port = DetectingPort::new();
        port.open(&fixture(name), None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        for table in port.tables() {
            port.read_rows(
                &table.name,
                datacraft_core::port::RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .unwrap_or_else(|e| panic!("{generation} table {}: {e}", table.name));
        }
        let want = expected
            .iter()
            .find(|(file, _, _)| *file == name)
            .map(|(_, table, rows)| (*table, *rows))
            .expect("expected row count is listed");
        let rows = port
            .read_rows(
                want.0,
                datacraft_core::port::RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .expect("the base table reads");
        assert_eq!(rows.len(), want.1, "{generation} {}", want.0);
    }
}

#[test]
fn ace_values_match_the_mdb_tools_export() {
    use datacraft_core::schema::FieldValue;
    let mut port = DetectingPort::new();
    port.open(&fixture("testV2007.accdb"), None)
        .expect("open ACE 2007");
    let rows = port
        .read_rows(
            "Table1",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("read Table1");
    let mut values: Vec<Vec<FieldValue>> = rows;
    values.sort_by_key(|row| format!("{row:?}"));

    let small = vec![
        FieldValue::Text("a".into()),
        FieldValue::Text("b".into()),
        FieldValue::Integer(0),
        FieldValue::Integer(0),
        FieldValue::Integer(0),
        FieldValue::Float(0.0),
        FieldValue::DateTime(29_932.0),
        FieldValue::Currency(0),
        FieldValue::Boolean(false),
    ];
    let large = vec![
        FieldValue::Text("abcdefg".into()),
        FieldValue::Text("hijklmnop".into()),
        FieldValue::Integer(2),
        FieldValue::Integer(222),
        FieldValue::Integer(333_333_333),
        FieldValue::Float(444.555),
        FieldValue::DateTime(27_293.0),
        FieldValue::Currency(35_000),
        FieldValue::Boolean(true),
    ];
    let mut expected = vec![small, large];
    expected.sort_by_key(|row| format!("{row:?}"));
    assert_eq!(values, expected);
}

#[test]
fn ace_relationships_are_reported() {
    let mut port = DetectingPort::new();
    port.open(&fixture("attachmentsV2016.accdb"), None)
        .expect("open ACE 2016");
    let relationships = port.relationships().expect("read relationships");
    assert!(!relationships.is_empty(), "the fixture carries catalog FKs");
}

#[test]
fn a_truncated_ace_file_is_rejected_precisely() {
    let mut file = bytes("testV2007.accdb");
    file.truncate(file.len() - 512);
    let path = std::env::temp_dir().join(format!(
        "datacraft-ace-truncated-{}.accdb",
        std::process::id()
    ));
    std::fs::write(&path, &file).expect("write truncated fixture");
    let mut port = DetectingPort::new();
    let err = port
        .open(&path, None)
        .expect_err("truncated files are corrupt");
    let _ = std::fs::remove_file(&path);
    match err {
        datacraft_core::port::PortError::OpenFailed { detail } => {
            assert!(detail.contains("not a multiple"), "{detail}");
        }
        other => panic!("expected OpenFailed, got {other:?}"),
    }
}
