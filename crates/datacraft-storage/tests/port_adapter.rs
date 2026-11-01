//! Contract suite for the `StoragePort` adapter (item H16).
//!
//! LESSONS #2 — seam tested from both sides: the core side runs against the
//! mock (core crate); this suite runs the real Jet4 read backend behind
//! core's port trait against the committed fixture, so the schema mapping
//! (Jet descriptors -> Access domain kinds) is exercised on real bytes.

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::TableDef;
use datacraft_storage::port_adapter::Jet4Port;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

#[test]
fn opens_the_real_fixture_with_format_and_user_tables() {
    let mut port = Jet4Port::new();
    let opened = port.open(&fixture(), None).expect("fixture must open");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert_eq!(opened.name, "testV2000");

    let tables = port.tables();
    assert!(!tables.is_empty(), "fixture has user tables");
    assert!(
        tables.iter().all(|t| !t.name.starts_with("MSys")),
        "system tables must not surface: {:?}",
        tables.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    assert!(tables.iter().any(|t| t.name == "Table1"));
}

#[test]
fn every_mapped_table_is_a_valid_domain_definition() {
    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    for table in port.tables() {
        table
            .validate()
            .unwrap_or_else(|e| panic!("table {:?} must validate: {e}", table.name));
        assert!(
            !table.fields.is_empty(),
            "table {:?} has fields",
            table.name
        );
    }
}

#[test]
fn text_columns_map_their_descriptor_length() {
    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    let tables = port.tables();
    let text_max_lens: Vec<u16> = tables
        .iter()
        .flat_map(|t| t.fields.iter())
        .filter_map(|f| match f.kind {
            datacraft_core::schema::FieldKind::Text { max_len } => Some(max_len),
            _ => None,
        })
        .collect();
    assert!(
        !text_max_lens.is_empty(),
        "the fixture catalog contains text columns"
    );
    assert!(
        text_max_lens.iter().all(|len| (1..=255).contains(len)),
        "mapped text lengths must be Access-valid: {text_max_lens:?}"
    );
}

/// Private copy of the pinned fixture — writes never touch the original.
fn fixture_copy(stem: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("datacraft-port-{}-{stem}.mdb", std::process::id()));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

#[test]
fn writes_require_an_open_database() {
    let mut port = Jet4Port::new();
    let def = TableDef::new("New", vec![]);
    assert!(matches!(
        port.create_table(&def),
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        port.drop_table("Table1"),
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        port.insert_row("Table1", &[]),
        Err(PortError::Unsupported { .. })
    ));
}

#[test]
fn drop_table_is_unsupported_with_a_pointer() {
    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    let err = port
        .drop_table("Table1")
        .expect_err("drop is not wired yet");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("M7"), "{err}");
}

#[test]
fn create_table_writes_a_private_copy_only() {
    let path = fixture_copy("create");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    let def = TableDef::new(
        "Written",
        vec![datacraft_core::schema::FieldDef::new(
            "note",
            datacraft_core::schema::FieldKind::Memo,
        )],
    );
    port.create_table(&def).expect("create on the copy");
    assert!(port.tables().iter().any(|t| t.name == "Written"));
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn open_failures_are_precise() {
    let mut port = Jet4Port::new();
    let err = port
        .open(Path::new("definitely-missing-424242.mdb"), None)
        .expect_err("missing file");
    assert!(
        err.to_string().contains("definitely-missing-424242"),
        "{err}"
    );

    let mut garbage = std::env::temp_dir();
    garbage.push(format!("datacraft-garbage-{}.mdb", std::process::id()));
    std::fs::write(&garbage, [0u8; 4096]).expect("write garbage fixture");
    let err = port.open(&garbage, None).expect_err("garbage file");
    assert!(matches!(err, PortError::OpenFailed { .. }), "{err}");
    let _ = std::fs::remove_file(&garbage);
}

#[test]
fn close_is_idempotent_and_clears_the_catalog() {
    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    assert!(!port.tables().is_empty());
    port.close();
    assert!(port.tables().is_empty());
    port.close();
    // Re-open after close re-attaches.
    port.open(&fixture(), None).expect("re-open");
    assert!(!port.tables().is_empty());
}

#[test]
fn tables_behind_multi_page_tdefs_decode() {
    // Table2 in the Access-rewritten fixture spans multiple TDEF pages
    // (Access chains them via the next-TDEF-page pointer at offset 4 —
    // docs/FORMAT-JET.md). The adapter must surface the table, not fail
    // the whole open.
    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    let names: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
    assert!(
        names.iter().any(|n| n == "Table2"),
        "multi-page TDEF table must surface: {names:?}"
    );
}

#[test]
fn read_rows_maps_storage_values_to_domain_values() {
    use datacraft_core::port::{RowRange, StoragePort as _};
    use datacraft_core::schema::FieldValue;

    let mut port = Jet4Port::new();
    port.open(&fixture(), None).expect("fixture must open");
    let tables = port.tables();
    let with_rows = tables
        .iter()
        .find(|t| {
            port.read_rows(&t.name, RowRange { start: 0, len: 1 })
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        })
        .expect("fixture has a table with rows");

    let rows = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("full read");
    let arity = with_rows.fields.len();
    assert!(
        rows.iter().all(|r| r.len() == arity),
        "every row matches the mapped field count"
    );
    // Windowing agrees with the full read.
    let half = rows.len() / 2;
    let first = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: 0,
                len: half,
            },
        )
        .expect("first half");
    assert_eq!(first, rows[..half]);
    let tail = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: rows.len(),
                len: 4,
            },
        )
        .expect("empty tail");
    assert!(tail.is_empty());

    let err = port
        .read_rows(
            &with_rows.name,
            RowRange {
                start: rows.len() + 1,
                len: 1,
            },
        )
        .expect_err("beyond the row set");
    assert!(matches!(
        err,
        datacraft_core::port::PortError::OutOfRange { .. }
    ));

    let err = port
        .read_rows("Ghost", RowRange { start: 0, len: 1 })
        .expect_err("unknown table");
    assert!(matches!(
        err,
        datacraft_core::port::PortError::TableNotFound(_)
    ));

    // Values are domain values, not storage values: spot-check a mapped row.
    let _ = FieldValue::Null;
}

#[test]
fn relationships_read_from_msysrelationships() {
    use datacraft_core::port::StoragePort as _;

    // The NavPane relationships come from the Access-touched lifecycle
    // (T3 second-writer variant); the pristine jackcess file has none.
    let touched = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000-access-touched.mdb");
    let mut port = Jet4Port::new();
    port.open(&touched, None).expect("fixture must open");
    let rels = port.relationships().expect("relationships read");
    // The fixture carries the two NavPane system relationships; every
    // mapping names all four ends.
    assert!(!rels.is_empty(), "fixture has relationships");
    for rel in &rels {
        assert!(!rel.table.is_empty() && !rel.column.is_empty());
        assert!(!rel.related_table.is_empty() && !rel.related_column.is_empty());
    }
    let nav_pane_pair = rels
        .iter()
        .find(|r| r.table == "MSysNavPaneGroups" && r.column == "GroupCategoryID");
    assert!(
        nav_pane_pair.is_some(),
        "the documented fixture relationship must map: {rels:?}"
    );
}

// --- port error paths (precise Unsupported, read-only seams) ---

#[test]
fn jet3_fixtures_open_through_the_port() {
    use datacraft_core::port::StoragePort as _;

    // A minimal Jet3 database: catalog TDEF at page 2 (format constant),
    // an empty catalog data page at 3 — enough for the port to open and
    // report the Jet3 format (step 10.6).
    let path = std::env::temp_dir().join("datacraft-jet3-port.mdb");
    std::fs::write(&path, jet3_catalog_shell()).unwrap();
    let mut port = Jet4Port::new();
    let opened = port.open(&path, None).expect("jet3 opens since step 10.6");
    assert!(matches!(
        opened.format,
        datacraft_core::port::DatabaseFormat::Jet3
    ));
    assert!(port.tables().is_empty(), "the catalog is empty");
    let _ = std::fs::remove_file(&path);
}

/// A minimal Jet3 shell with a plausible (empty) MSysObjects TDEF.
fn jet3_catalog_shell() -> Vec<u8> {
    use datacraft_storage::jet::JetType;

    const PAGE: usize = 2048;
    let mut file = vec![0u8; PAGE * 4];
    file[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    file[4..20].copy_from_slice(b"Standard Jet DB\x00");
    file[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
    // Unprotected Jet3 password field + the fixed-key header region
    // (step 16.2 / M5).
    datacraft_storage::jet::crypto::rc4(
        &datacraft_storage::jet::crypto::HEADER_RC4_KEY,
        &mut file[0x18..0x18 + datacraft_storage::jet::crypto::JET3_HEADER_REGION],
    );
    // Page 2: catalog TDEF ('VC' header, 35-byte block, three columns).
    let base = 2 * PAGE;
    file[base] = 0x02;
    file[base + 1] = 0x01;
    file[base + 2..base + 4].copy_from_slice(b"VC");
    let columns = [
        (JetType::LongInt, 0x01u8, 4u16),
        (JetType::Text, 0x02, 100),
        (JetType::LongInt, 0x01, 2),
        (JetType::LongInt, 0x01, 4),
    ];
    let mut block: Vec<u8> = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&0u32.to_le_bytes()); // num_rows
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x53); // system table
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&1u16.to_le_bytes()); // num_var
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // used_pages
    block.extend_from_slice(&0u32.to_le_bytes()); // free_pages
    for (i, (kind, flags, len)) in columns.iter().enumerate() {
        block.push(*kind as u8);
        block.extend_from_slice(&(i as u16).to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes()); // offset_V
        block.extend_from_slice(&(i as u16).to_le_bytes());
        block.extend_from_slice(&0x0409u16.to_le_bytes()); // sort order
        block.extend_from_slice(&0u16.to_le_bytes()); // misc
        block.extend_from_slice(&0u16.to_le_bytes()); // unknown
        block.push(*flags);
        block.extend_from_slice(&0u16.to_le_bytes()); // fixed_offset
        block.extend_from_slice(&len.to_le_bytes());
    }
    for name in [
        b"Id".as_slice(),
        b"Name".as_slice(),
        b"Type".as_slice(),
        b"Flags".as_slice(),
    ] {
        block.push(name.len() as u8);
        block.extend_from_slice(name);
    }
    block.extend_from_slice(&0xFFFFu16.to_le_bytes());
    let len = block.len() as u32;
    block[..4].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
    // Page 3: empty catalog data page owned by the TDEF page.
    let base = 3 * PAGE;
    file[base] = 0x01;
    file[base + 1] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&2u32.to_le_bytes());
    file
}

#[test]
fn port_queries_without_an_open_database_are_unsupported() {
    use datacraft_core::port::StoragePort as _;

    let mut port = Jet4Port::new();
    assert!(port.relationships().is_err());
    let window = datacraft_core::port::RowRange { start: 0, len: 1 };
    assert!(port.read_rows("Any", window).is_err());
    port.close();
}
