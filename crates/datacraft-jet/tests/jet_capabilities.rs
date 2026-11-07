//! Driver capability invariants over the real Jet backend (phase 30,
//! item M17; ADR-7): a capability reported `No` answers `Unsupported` on
//! the matching port call; a capability reported `Yes` is honoured (the
//! write paths themselves are pinned by `jet_port_writes`,
//! `jet_alter_table` and `create_database`). RED state: `Jet4Port` does
//! not override the capability reporting yet — it answers the optimistic
//! trait defaults, so the identity, atomicity, AutoNumber and Jet3
//! read-only assertions fail.

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, DriverId, PortError, SchemaChange, StoragePort, Support,
};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn any_table() -> TableDef {
    TableDef::new(
        "CapabilityProbe",
        vec![FieldDef::new(
            "n",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

/// The Jet driver names itself; the session-independent capabilities are
/// the read/write reference set.
#[test]
fn the_jet_driver_reports_its_reference_capabilities() {
    let port = Jet4Port::new();
    assert_eq!(port.driver_id(), DriverId::new("jet"));
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.write_rows);
    assert!(caps.create_table);
    assert!(caps.drop_table);
    assert!(caps.create_database);
    assert!(caps.alter_table);
    assert!(caps.atomic_batch, "Jet batches apply in one writer session");
    assert!(caps.passwords);
    assert_eq!(caps.autonumber, Support::Yes);
    assert_eq!(caps.relationships, Support::Yes);
    assert_eq!(caps.native_queries, Support::ReadOnly);
    assert_eq!(caps.macros, Support::ReadOnly);
    assert_eq!(caps.vba, Support::ReadOnly);
    assert_eq!(caps.triggers, Support::No);
    assert_eq!(caps.stored_procedures, Support::No);
}

/// An open Jet4 session keeps the reference set (caps are per session;
/// Jet4 has no session variance).
#[test]
fn a_jet4_session_reports_the_reference_capabilities() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2000.mdb"), None)
        .expect("open the Jet4 fixture");
    let caps = port.capabilities();
    assert!(caps.write_rows);
    assert!(caps.alter_table);
    assert!(caps.atomic_batch);
    assert_eq!(caps.autonumber, Support::Yes);
    assert!(!port.tables().is_empty(), "read_tables capability honoured");
}

/// An open Jet3 (Access 97) session is read-only: no writer exists for
/// Jet3, and the capability set says so — except file creation, which is
/// session-independent.
#[test]
fn a_jet3_session_reports_read_only_capabilities() {
    let mut port = Jet4Port::new();
    let opened = port
        .open(&fixture("testV1997.mdb"), None)
        .expect("open the Jet3 fixture");
    assert_eq!(opened.format, DatabaseFormat::Jet3);
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.create_database, "creation is session-independent");
    assert!(!caps.write_rows);
    assert!(!caps.create_table);
    assert!(!caps.drop_table);
    assert!(!caps.alter_table);
    assert!(!caps.atomic_batch);
    assert_eq!(caps.autonumber, Support::No);
}

/// The declared read-only state is a contract: every write call on the
/// Jet3 session answers a precise `Unsupported`.
#[test]
fn jet3_write_capability_gaps_answer_unsupported() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV1997.mdb"), None)
        .expect("open the Jet3 fixture");
    let first = port
        .tables()
        .first()
        .expect("the Jet3 fixture catalogues tables")
        .clone();
    // Arity-correct Null row: the port validates arity before the writer
    // rejects the Jet3 session.
    let null_row = vec![FieldValue::Null; first.fields.len()];

    let error = port
        .create_table(&any_table())
        .expect_err("create_table is unsupported on Jet3");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    let error = port
        .insert_row(&first.name, &null_row)
        .expect_err("insert_row is unsupported on Jet3");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    let error = port
        .drop_table(&first.name)
        .expect_err("drop_table is unsupported on Jet3");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    let error = port
        .alter_table(
            &first.name,
            SchemaChange::AddField {
                field: FieldDef::new("probe", FieldKind::Number(NumericKind::LongInteger)),
            },
        )
        .expect_err("alter_table is unsupported on Jet3");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
}

/// A reported `Yes` is honoured: Jet4 file creation works through the
/// reported capability (the write paths are pinned by their own suites).
#[test]
fn jet4_create_database_capability_is_honoured() {
    let mut port = Jet4Port::new();
    assert!(port.capabilities().create_database);
    let path = std::env::temp_dir().join(format!("datacraft-jet-caps-{}.mdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create the Jet4 file");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

/// A scratch-dir copy of `name` with an empty Access lock file beside it
/// (M59 acceptance: fixture-built empty file in a scratch dir). The
/// fixture itself is only ever read.
fn locked_copy(name: &str, lock_extension: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "datacraft-ldb-{}-{}",
        name.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '.'),
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create the scratch dir");
    let copy = dir.join(name);
    std::fs::copy(fixture(name), &copy).expect("copy the fixture");
    let lock = dir.join(format!(
        "{}.{lock_extension}",
        name.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '.')
    ));
    std::fs::write(&lock, b"").expect("write the empty lock file");
    copy
}

/// Co-existence (step 53.1, item M59, user incident 2026-11-06): a
/// sibling `.ldb` means Access may hold the database open — the session
/// downgrades to the Jet3 read-only shape and every write answers a
/// precise `Unsupported`. DataCraft never creates or deletes lock files.
#[test]
fn a_sibling_lock_file_forces_a_read_only_jet4_session() {
    let path = locked_copy("testV2000.mdb", "ldb");
    let mut port = Jet4Port::new();
    let opened = port.open(&path, None).expect("open the locked copy");
    assert_eq!(
        opened.lock_file.as_deref(),
        Some("testV2000.ldb"),
        "the lock file travels as data for every client to localize"
    );
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.create_database, "creation is session-independent");
    assert!(!caps.write_rows);
    assert!(!caps.create_table);
    assert!(!caps.drop_table);
    assert!(!caps.alter_table);
    assert!(!caps.atomic_batch);
    assert_eq!(caps.autonumber, Support::No);

    let first = port
        .tables()
        .first()
        .expect("the fixture catalogues tables")
        .clone();
    let null_row = vec![FieldValue::Null; first.fields.len()];
    let error = port
        .insert_row(&first.name, &null_row)
        .expect_err("insert_row is unsupported on a locked session");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    let error = port
        .create_table(&any_table())
        .expect_err("create_table is unsupported on a locked session");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
}

/// Without a lock file the session is unchanged — the reference
/// read/write set (M59: the detection must not false-positive).
#[test]
fn without_a_lock_file_the_jet4_session_stays_writable() {
    let dir = std::env::temp_dir().join(format!("datacraft-nolock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the scratch dir");
    let copy = dir.join("testV2000.mdb");
    std::fs::copy(fixture("testV2000.mdb"), &copy).expect("copy the fixture");
    let mut port = Jet4Port::new();
    port.open(&copy, None).expect("open the unlocked copy");
    let caps = port.capabilities();
    assert!(caps.write_rows);
    assert!(caps.alter_table);
    assert!(caps.atomic_batch);
    assert_eq!(caps.autonumber, Support::Yes);
}

/// ACE sources lock through their own extension: a sibling `.laccdb`
/// downgrades the session just like an `.ldb` does for Jet.
#[test]
fn an_accdb_lock_file_is_detected_by_its_own_extension() {
    let path = locked_copy("testV2007.accdb", "laccdb");
    let mut port = Jet4Port::new();
    let opened = port.open(&path, None).expect("open the locked accdb copy");
    assert_eq!(
        opened.lock_file.as_deref(),
        Some("testV2007.laccdb"),
        "ACE sources lock through .laccdb"
    );
    let caps = port.capabilities();
    assert!(!caps.write_rows);
    assert!(!caps.alter_table);
    assert_eq!(caps.autonumber, Support::No);
}
