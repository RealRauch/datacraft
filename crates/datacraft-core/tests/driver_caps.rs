//! Driver capability contracts (phase 30, item M17; ADR-7).
//!
//! Every data source reports a stable driver id and per-session
//! capabilities; the snapshot carries both for every client; dispatch
//! rejects commands the open source declared impossible before the port
//! is called. RED state: the capability vocabulary does not exist yet —
//! the suite fails to compile until `DriverId`, `DriverCapabilities`,
//! `Support` and the port methods land.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, MockPort, StoragePort, Support,
};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn one_field_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

/// A core whose mock reports exactly `caps`, with one table catalogued
/// and the database open.
fn core_with(caps: DriverCapabilities) -> Core {
    let mut port = MockPort::with_tables(vec![one_field_table()]);
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: std::path::PathBuf::from("shop.mdb"),
        password: None,
    });
    assert!(
        matches!(events.first(), Some(Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    core
}

/// Exactly one `Failed` event, with its reason.
fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected exactly one Failed event, got {other:?}"),
    }
}

/// The mock reports its honest default set: everything DataCraft's
/// commands can drive except atomic batches (the trait default applies
/// operations one by one) and passwords (the mock ignores them).
#[test]
fn the_mock_reports_honest_default_capabilities() {
    let port = MockPort::new();
    assert_eq!(port.driver_id(), DriverId::new("mock"));
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.write_rows);
    assert!(caps.create_table);
    assert!(caps.drop_table);
    assert!(caps.create_database);
    assert!(caps.alter_table);
    assert!(!caps.atomic_batch, "the mock applies batches one by one");
    assert!(!caps.passwords, "the mock ignores passwords");
    assert_eq!(caps.autonumber, Support::No);
    assert_eq!(caps.relationships, Support::Yes);
    assert_eq!(caps.native_queries, Support::ReadOnly);
    assert_eq!(caps.macros, Support::ReadOnly);
    assert_eq!(caps.vba, Support::ReadOnly);
    assert_eq!(caps.triggers, Support::No);
    assert_eq!(caps.stored_procedures, Support::No);
}

/// Driver identity and capabilities ride the snapshot, so GUI, CLI and
/// MCP agents see the same truth about the open source.
#[test]
fn the_snapshot_carries_driver_and_capabilities() {
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    caps.atomic_batch = false;
    let core = core_with(caps);
    let db = core.inspect().database.expect("the database is open");
    assert_eq!(db.driver, DriverId::new("mock"));
    assert!(!db.capabilities.write_rows);
    assert!(!db.capabilities.atomic_batch);
    assert!(db.capabilities.read_tables);
    assert_eq!(db.capabilities.relationships, Support::Yes);
}

/// The detected Access lock file travels into the snapshot as data (step
/// 53.3, item M59): the CLI and MCP clients surface the notice through
/// the same JSON the GUI localizes from — agents key on data, not prose
/// (I6; ADR-4 keeps the language UI-side).
#[test]
fn the_snapshot_carries_the_lock_file_for_the_headless_clients() {
    let mut port = MockPort::new();
    port.with_lock_file("shop.ldb");
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    let db = core.inspect().database.expect("the database is open");
    assert_eq!(db.lock_file.as_deref(), Some("shop.ldb"));
}

/// The wire spells the vocabulary deterministically — flat capability
/// object, snake_case support levels, transparent driver id — so agents
/// key on stable JSON, not prose.
#[test]
fn capabilities_serialize_deterministically_over_the_wire() {
    let mut caps = DriverCapabilities::full();
    caps.relationships = Support::ReadOnly;
    caps.triggers = Support::No;
    let json = serde_json::to_value(caps).expect("capabilities serialize");
    assert_eq!(json["write_rows"], serde_json::json!(true));
    assert_eq!(json["relationships"], serde_json::json!("read_only"));
    assert_eq!(json["triggers"], serde_json::json!("no"));
    let round: DriverCapabilities = serde_json::from_value(json).expect("round trip");
    assert_eq!(round, caps);

    let core = core_with(DriverCapabilities::full());
    let snapshot = serde_json::to_value(core.inspect()).expect("snapshot serializes");
    assert_eq!(snapshot["database"]["driver"], serde_json::json!("mock"));
    assert_eq!(
        snapshot["database"]["capabilities"]["create_table"],
        serde_json::json!(true)
    );
}

/// Row writes the driver declared impossible are refused in dispatch —
/// with the same Failed shape a port refusal produces — and never reach
/// the port.
#[test]
fn dispatch_rejects_row_writes_the_driver_declared_impossible() {
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    let mut core = core_with(caps);
    let reason = failed_reason(&core.dispatch(Command::InsertRow {
        table: "Customers".to_owned(),
        values: vec![FieldValue::Integer(1)],
    }));
    assert!(reason.contains("unsupported"), "{reason}");
    assert!(reason.contains("write"), "{reason}");

    let events = core.dispatch(Command::UpdateRow {
        table: "Customers".to_owned(),
        row: 0,
        values: vec![FieldValue::Integer(2)],
    });
    assert!(
        matches!(events.as_slice(), [Event::Failed { .. }]),
        "{events:?}"
    );
    let events = core.dispatch(Command::DeleteRow {
        table: "Customers".to_owned(),
        row: 0,
    });
    assert!(
        matches!(events.as_slice(), [Event::Failed { .. }]),
        "{events:?}"
    );
}

/// Schema and table writes are refused per their own capabilities.
#[test]
fn dispatch_rejects_schema_writes_the_driver_declared_impossible() {
    let mut caps = DriverCapabilities::full();
    caps.create_table = false;
    caps.drop_table = false;
    caps.alter_table = false;
    let mut core = core_with(caps);

    let reason = failed_reason(&core.dispatch(Command::CreateTable {
        definition: one_field_table(),
    }));
    assert!(reason.contains("unsupported"), "{reason}");
    assert!(reason.contains("create"), "{reason}");

    let reason = failed_reason(&core.dispatch(Command::DropTable {
        table: "Customers".to_owned(),
    }));
    assert!(reason.contains("unsupported"), "{reason}");

    let reason = failed_reason(&core.dispatch(Command::AddField {
        table: "Customers".to_owned(),
        field: FieldDef::new("name2", FieldKind::Number(NumericKind::LongInteger)),
    }));
    assert!(reason.contains("unsupported"), "{reason}");
    assert!(reason.contains("alter"), "{reason}");
}

/// Database creation is refused before the port when the driver reports
/// the capability as absent — session-independent, like the command.
#[test]
fn dispatch_rejects_database_creation_the_driver_declared_impossible() {
    let mut caps = DriverCapabilities::full();
    caps.create_database = false;
    let mut port = MockPort::new();
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    let reason = failed_reason(&core.dispatch(Command::CreateDatabase {
        path: std::env::temp_dir().join("datacraft-driver-caps-probe.mdb"),
        format: DatabaseFormat::Jet4,
    }));
    assert!(reason.contains("unsupported"), "{reason}");
    assert!(reason.contains("create"), "{reason}");
}
