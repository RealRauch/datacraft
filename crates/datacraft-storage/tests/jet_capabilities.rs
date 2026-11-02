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
use datacraft_storage::port_adapter::Jet4Port;

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
    assert_eq!(caps.relationships, Support::ReadOnly);
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
    let path = std::env::temp_dir().join(format!(
        "datacraft-jet-caps-{}.mdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create the Jet4 file");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}
