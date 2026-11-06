//! Fresh-file creation contracts (phase 29, step 29.1, item H44).
//!
//! A brand-new database is created from nothing, reopens through the same
//! boundaries every other file uses, and takes the full write path. The
//! Jet4 builder is byte-deterministic for a fixed creation date (LESSONS
//! #6). RED state: `create_jet4_file` and `StoragePort::create_database`
//! do not exist yet — the suite fails to compile until they land.

use std::path::{Path, PathBuf};
use std::process::id;

use datacraft_core::port::{DatabaseFormat, SchemaChange, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::jet::crypto::{HEADER_RC4_KEY, JET4_HEADER_REGION, decode_header_region, rc4};
use datacraft_jet::jet::{FileHeader, Jet4TableReader};
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_jet::{FormatVersion, StorageError};
use datacraft_sqlite::SqlitePort;

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("datacraft-storage-create-{}-{name}", id()))
}

fn jet_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("the created file exists")
}

/// Page-0 bytes 0xE00..0x1000 — the 256-word tail the Jet engine's
/// open path validates (FORMAT-JET "Page-0 tail and create-path acceptance").
fn page0_tail(bytes: &[u8]) -> &[u8] {
    &bytes[0xE00..0x1000]
}

/// The page-0 header region decrypted with the fixed RC4 key (the writer's
/// in-memory layout; constants are asserted on the decrypted shape).
fn decrypted_region(bytes: &[u8]) -> Vec<u8> {
    let mut region = bytes[0x18..0x18 + JET4_HEADER_REGION].to_vec();
    rc4(&HEADER_RC4_KEY, &mut region);
    region
}

/// The deterministic page-0 state a fresh file must carry: the uniform
/// `0x0100` tail and the decrypted region with the observed constant set
/// (fresh DAO reference, step 49.1) plus code page, password mask and the
/// template's fixed creation date (the engine binds its ACL validation to
/// that time-of-day — the date is carried, never redated; 49.2).
fn expected_page0_state() -> (Vec<u8>, Vec<u8>) {
    /// The template's creation date, byte-exact (OLE days).
    const TEMPLATE_CREATION_DATE: f64 = f64::from_bits(0x40E6_9F9E_3FE5_8AFA);
    let mut tail = vec![0u8; 0x1000 - 0xE00];
    for word in tail.as_chunks_mut::<2>().0 {
        *word = 0x0100u16.to_le_bytes();
    }
    let mut region = vec![0u8; JET4_HEADER_REGION];
    for (index, constant) in [0x100u32, 0x101, 2, 3, 4, 5].into_iter().enumerate() {
        region[index * 4..index * 4 + 4].copy_from_slice(&constant.to_le_bytes());
    }
    region[0x24..0x26].copy_from_slice(&1252u16.to_le_bytes());
    let mask = (TEMPLATE_CREATION_DATE as i64 as u32).to_le_bytes();
    for (index, slot) in region[0x2A..0x52].iter_mut().enumerate() {
        *slot = mask[index % 4];
    }
    region[0x52..0x56].copy_from_slice(&0x11A6u32.to_le_bytes());
    region[0x56..0x5A].copy_from_slice(&0x0409u32.to_le_bytes());
    region[0x5A..0x62].copy_from_slice(&TEMPLATE_CREATION_DATE.to_le_bytes());
    (tail, region)
}

/// The template's fixed creation date (byte-exact; mirrored from
/// `create.rs`'s embedded template for the date assertions).
pub const TEMPLATE_CREATION_DATE: f64 = f64::from_bits(0x40E6_9F9E_3FE5_8AFA);

#[test]
fn a_fresh_jet4_file_carries_the_engine_system_catalog() {
    let path = scratch("fresh.mdb");
    datacraft_jet::jet::create::create_jet4_file(&path, 45_000.0).expect("create");
    let bytes = jet_bytes(&path);

    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("valid header");
    assert_eq!(header.format, FormatVersion::Jet4);
    assert_eq!(
        header.page_count, 19,
        "header, maps, catalog TDEF, usage, four system tables"
    );
    assert_eq!(bytes.len(), 19 * 4096);

    // The fresh catalog carries the engine's own system shape: the three
    // containers, MSysDb and the four system tables (49.2 template; the
    // `MSys` tables stay hidden behind the port's prefix filter).
    let reader = Jet4TableReader::new(bytes).expect("the fresh file reopens");
    let catalog = reader.catalog().expect("catalog decodes");
    let names: Vec<&str> = catalog.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Tables",
            "Databases",
            "Relationships",
            "MSysDb",
            "MSysObjects",
            "MSysACEs",
            "MSysQueries",
            "MSysRelationships",
        ],
        "the fresh catalog is the engine's system shape"
    );
    let aces = catalog
        .iter()
        .find(|entry| entry.name == "MSysACEs")
        .expect("MSysACEs row present");
    assert_eq!(aces.object_type, 1, "system tables are type-1 rows");
    assert_eq!(aces.flags, -2_147_483_648, "the system flag bit is set");

    let mut port = Jet4Port::new();
    let opened = port
        .open(&path, None)
        .expect("the port opens the fresh file");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert!(port.tables().is_empty(), "no user tables yet");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creation_is_deterministic_for_a_fixed_creation_date() {
    let first = scratch("det-a.mdb");
    let second = scratch("det-b.mdb");
    datacraft_jet::jet::create::create_jet4_file(&first, 45_000.0).expect("create a");
    datacraft_jet::jet::create::create_jet4_file(&second, 45_000.0).expect("create b");
    assert_eq!(
        jet_bytes(&first),
        jet_bytes(&second),
        "same input, same bytes"
    );
    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

#[test]
fn a_fresh_database_takes_the_full_write_path() {
    let path = scratch("lifecycle.mdb");
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create through the port");
    port.open(&path, None).expect("open the fresh file");

    let people = TableDef::new(
        "People",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    );
    port.create_table(&people)
        .expect("create a table in the fresh file");
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("Ada".into())],
    )
    .expect("insert a row");
    port.close();

    let mut reopened = Jet4Port::new();
    reopened.open(&path, None).expect("reopen");
    let rows = reopened
        .read_rows(
            "People",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("the row persisted");
    assert_eq!(
        rows,
        [vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())]]
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creating_over_an_existing_file_is_refused_and_leaves_it_intact() {
    let path = scratch("exists.mdb");
    datacraft_jet::jet::create::create_jet4_file(&path, 45_000.0).expect("first create");
    let before = jet_bytes(&path);

    let error = datacraft_jet::jet::create::create_jet4_file(&path, 46_000.0).expect_err("refused");
    match error {
        StorageError::Unsupported { reason } => {
            assert!(reason.contains("already exists"), "{reason}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    assert_eq!(
        jet_bytes(&path),
        before,
        "the refused create touched nothing"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creating_into_a_missing_directory_is_a_precise_error() {
    let dir = scratch("missing-dir");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("fresh.mdb");
    let error = datacraft_jet::jet::create::create_jet4_file(&path, 45_000.0)
        .expect_err("the parent directory does not exist");
    match error {
        StorageError::Unsupported { reason } => {
            assert!(reason.contains("fresh.mdb"), "{reason}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    assert!(!path.exists(), "no partial file");
}

#[test]
fn a_materialization_row_shape_mismatch_is_refused() {
    let path = scratch("row-mismatch.mdb");
    let tables = vec![(
        TableDef::new(
            "People",
            vec![FieldDef::new("name", FieldKind::Text { max_len: 10 })],
        ),
        vec![vec![FieldValue::Text("Ada".into()), FieldValue::Integer(1)]],
    )];
    let error = datacraft_jet::jet::create::materialize_jet4_template(&path, 45_000.0, &tables)
        .expect_err("the row holds more values than the table defines fields");
    assert!(error.to_string().contains("values"), "{error:?}");
    assert!(!path.exists(), "the failed materialization left no file");
}

#[test]
fn the_jet_backend_refuses_every_other_format_precisely() {
    let mut port = Jet4Port::new();
    for (format, needle) in [
        (DatabaseFormat::Ace, "ACE"),
        (DatabaseFormat::Jet3, "Jet3"),
        (DatabaseFormat::Sqlite, "SQLite"),
    ] {
        let error = port
            .create_database(&scratch("wrong.mdb"), format)
            .expect_err("{format:?} is not a Jet4 creation target");
        match error {
            datacraft_core::port::PortError::Unsupported { reason } => {
                assert!(reason.contains(needle), "{format:?}: {reason}");
            }
            other => panic!("{format:?}: expected Unsupported, got {other:?}"),
        }
    }
}

#[test]
fn the_sqlite_backend_creates_an_openable_database() {
    let path = scratch("fresh.sqlite");
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite)
        .expect("create");

    let bytes = jet_bytes(&path);
    assert!(
        bytes.starts_with(b"SQLite format 3\0"),
        "real SQLite header"
    );

    let opened = port.open(&path, None).expect("reopen the fresh database");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert!(port.tables().is_empty());
    // Windows cannot delete a file with a live SQLite handle: close first,
    // otherwise every run leaves a scratch file behind (F5).
    port.close();
    std::fs::remove_file(&path).expect("remove the scratch database");
}

#[test]
fn the_sqlite_backend_refuses_jet4_targets() {
    let mut port = SqlitePort::new();
    let error = port
        .create_database(&scratch("wrong.sqlite"), DatabaseFormat::Jet4)
        .expect_err("SQLite does not create Jet files");
    assert!(
        matches!(error, datacraft_core::port::PortError::Unsupported { .. }),
        "{error:?}"
    );
}

/// Step 49.2 (M63): page 0 of a fresh file carries the region constants and
/// the uniform tail the real Jet engine's open path requires — the tail gate
/// ("Zu viele aktive Benutzer") flipped by byte-level bisection in 49.1.
#[test]
fn the_fresh_page0_carries_the_region_constants_and_the_uniform_tail() {
    let path = scratch("page0.mdb");
    datacraft_jet::jet::create::create_jet4_file(&path, 45_000.0).expect("create");
    let bytes = jet_bytes(&path);
    let (tail, region) = expected_page0_state();

    assert_eq!(
        page0_tail(&bytes),
        tail.as_slice(),
        "page-0 tail: 256 uniform 0x0100 words"
    );
    assert_eq!(
        decrypted_region(&bytes),
        region,
        "decrypted region: constants 0x100/0x101/2/3/4/5, code page, password mask, \
         build number 0x11A6, collation 0x0409, creation date"
    );
    let _ = std::fs::remove_file(&path);
}

/// Step 49.2 (M63): the tail and the region constants survive every rewrite
/// (read-modify-write keeps page 0 verbatim; no counter emulation — the
/// engine adopts the foreign tail as its own baseline).
#[test]
fn the_page0_tail_and_region_constants_survive_every_rewrite() {
    let path = scratch("preserve.mdb");
    let mut port = Jet4Port::new();
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create through the port");
    port.open(&path, None).expect("open the fresh file");

    // The fresh file carries the template's fixed creation date (the ACL
    // anchor) — assert it, then pin tail and region across the lifecycle.
    let bytes = jet_bytes(&path);
    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("valid header");
    let info = decode_header_region(&bytes, &header).expect("the fresh region decodes");
    assert_eq!(
        info.creation_date, TEMPLATE_CREATION_DATE,
        "the template creation date is carried verbatim"
    );
    let (tail, region) = expected_page0_state();

    let unchanged = |label: &str| {
        let bytes = jet_bytes(&path);
        assert_eq!(page0_tail(&bytes), tail.as_slice(), "{label}: tail");
        assert_eq!(decrypted_region(&bytes), region, "{label}: region");
    };
    unchanged("freshly created");

    let people = TableDef::new(
        "People",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    );
    port.create_table(&people).expect("create a table");
    unchanged("after create_table");
    port.insert_row(
        "People",
        &[FieldValue::Integer(1), FieldValue::Text("Ada".into())],
    )
    .expect("insert a row");
    unchanged("after insert_row");
    port.insert_row(
        "People",
        &[FieldValue::Integer(2), FieldValue::Text("Grace".into())],
    )
    .expect("insert a second row");
    port.update_row(
        "People",
        0,
        &[
            FieldValue::Integer(1),
            FieldValue::Text("Ada Lovelace".into()),
        ],
    )
    .expect("update the row");
    unchanged("after update_row");
    port.delete_row("People", 1).expect("delete the row");
    unchanged("after delete_row");
    port.alter_table(
        "People",
        SchemaChange::AddField {
            field: FieldDef::new("note", FieldKind::Text { max_len: 80 }),
        },
    )
    .expect("alter the table");
    unchanged("after alter_table");
    port.drop_table("People").expect("drop the table");
    unchanged("after drop_table");

    port.close();
    let mut reopened = Jet4Port::new();
    reopened
        .open(&path, None)
        .expect("reopen after the lifecycle");
    unchanged("after reopen");
    let _ = std::fs::remove_file(&path);
}
