//! Affordance availability contracts (phase 30, item M17; ADR-7): the
//! core derives, per open source, which command affordances are disabled
//! and why. The map lists only *restricted* affordances — absence means
//! enabled — so existing clients and UI snapshots stay unchanged for
//! capable sources. RED state: the availability vocabulary does not
//! exist yet — the suite fails to compile until it lands.

use std::collections::BTreeMap;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DriverCapabilities, MockPort, Support};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn one_field_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

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

/// A restricted source disables exactly the matching affordances, with
/// stable reasons keyed by stable ids.
#[test]
fn restricted_capabilities_disable_the_matching_affordances() {
    let mut caps = DriverCapabilities::full();
    caps.create_table = false;
    caps.drop_table = false;
    caps.write_rows = false;
    caps.relationships = Support::No;
    let core = core_with(caps);
    let db = core.inspect().database.expect("database open");
    assert_eq!(
        db.availability,
        BTreeMap::from([
            (
                "table".to_owned(),
                datacraft_core::availability::DisabledReason::CreateTables,
            ),
            (
                "nav.delete".to_owned(),
                datacraft_core::availability::DisabledReason::DropTables,
            ),
            (
                "datasheet.new_record".to_owned(),
                datacraft_core::availability::DisabledReason::WriteRows,
            ),
            (
                "relationships".to_owned(),
                datacraft_core::availability::DisabledReason::NoRelationships,
            ),
        ])
    );
}

/// A capable source leaves the map empty — absence means enabled, and
/// no existing snapshot or UI pin changes.
#[test]
fn full_capabilities_leave_the_availability_map_empty() {
    let core = core_with(DriverCapabilities::full());
    let db = core.inspect().database.expect("database open");
    assert!(db.availability.is_empty());
}

/// The reasons spell deterministically on the wire so agents key on
/// stable JSON, not prose.
#[test]
fn disabled_reasons_serialize_stable_on_the_wire() {
    use datacraft_core::availability::DisabledReason;
    assert_eq!(
        serde_json::to_value(DisabledReason::CreateTables).expect("serialize"),
        serde_json::json!("create_tables")
    );
    assert_eq!(
        serde_json::to_value(DisabledReason::WriteRows).expect("serialize"),
        serde_json::json!("write_rows")
    );
    assert_eq!(
        serde_json::to_value(DisabledReason::DropTables).expect("serialize"),
        serde_json::json!("drop_tables")
    );
    assert_eq!(
        serde_json::to_value(DisabledReason::NoRelationships).expect("serialize"),
        serde_json::json!("no_relationships")
    );
}
