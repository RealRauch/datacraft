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
    caps.passwords = false;
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
                "datasheet.delete_record".to_owned(),
                datacraft_core::availability::DisabledReason::WriteRows,
            ),
            (
                "datasheet.edit_record".to_owned(),
                datacraft_core::availability::DisabledReason::WriteRows,
            ),
            (
                "relationships".to_owned(),
                datacraft_core::availability::DisabledReason::NoRelationships,
            ),
            (
                "relationships.create".to_owned(),
                datacraft_core::availability::DisabledReason::NoRelationships,
            ),
            (
                "relationships.delete".to_owned(),
                datacraft_core::availability::DisabledReason::NoRelationships,
            ),
            (
                "backstage.save_as".to_owned(),
                datacraft_core::availability::DisabledReason::NoPasswords,
            ),
            (
                "backstage.change_password".to_owned(),
                datacraft_core::availability::DisabledReason::NoPasswords,
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

/// The relationship *write* affordances gate on `Support::Yes` — the
/// dispatch gate refuses anything less, so a read-only surface must not
/// advertise creatable relationships while the viewer affordance (the
/// `relationships` id) stays enabled for it.
#[test]
fn relationship_write_affordances_gate_on_support_yes() {
    let mut caps = DriverCapabilities::full();
    caps.relationships = Support::ReadOnly;
    let db = core_with(caps).inspect().database.expect("database open");
    for id in ["relationships.create", "relationships.delete"] {
        assert_eq!(
            db.availability.get(id),
            Some(&datacraft_core::availability::DisabledReason::NoRelationships),
            "affordance {id:?} must be disabled for a read-only surface"
        );
    }
    assert!(
        !db.availability.contains_key("relationships"),
        "the viewer affordance stays enabled for a read-only surface"
    );

    let mut caps = DriverCapabilities::full();
    caps.relationships = Support::No;
    let db = core_with(caps).inspect().database.expect("database open");
    for id in [
        "relationships",
        "relationships.create",
        "relationships.delete",
    ] {
        assert_eq!(
            db.availability.get(id),
            Some(&datacraft_core::availability::DisabledReason::NoRelationships),
            "affordance {id:?} must be disabled for a No surface"
        );
    }

    let mut caps = DriverCapabilities::full();
    caps.relationships = Support::Yes;
    let db = core_with(caps).inspect().database.expect("database open");
    assert!(db.availability.is_empty(), "{:?}", db.availability);
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
    assert_eq!(
        serde_json::to_value(DisabledReason::NoPasswords).expect("serialize"),
        serde_json::json!("no_passwords")
    );
    assert_eq!(
        serde_json::to_value(DisabledReason::AlterTables).expect("serialize"),
        serde_json::json!("alter_tables")
    );
    assert_eq!(
        serde_json::to_value(DisabledReason::CreateDatabase).expect("serialize"),
        serde_json::json!("create_database")
    );
}

/// The design view's edit affordances gate on `alter_table` and the
/// backstage Create Database button on `create_database`
/// (step 42.2, item L18); capable sources carry neither.
#[test]
fn schema_edit_and_create_affordances_gate_on_their_capabilities() {
    let mut caps = DriverCapabilities::full();
    caps.alter_table = false;
    caps.create_database = false;
    let db = core_with(caps).inspect().database.expect("database open");
    for id in [
        "design.add_field",
        "design.rename_field",
        "design.drop_field",
        "design.set_field_properties",
    ] {
        assert_eq!(
            db.availability.get(id),
            Some(&datacraft_core::availability::DisabledReason::AlterTables),
            "affordance {id:?} must be disabled for a source without alter_table"
        );
    }
    assert_eq!(
        db.availability.get("backstage.create_database"),
        Some(&datacraft_core::availability::DisabledReason::CreateDatabase),
        "the create-database affordance must be disabled without create_database"
    );

    let db = core_with(DriverCapabilities::full())
        .inspect()
        .database
        .expect("database open");
    for id in [
        "design.add_field",
        "design.rename_field",
        "design.drop_field",
        "design.set_field_properties",
        "backstage.create_database",
    ] {
        assert!(
            !db.availability.contains_key(id),
            "a capable source keeps {id:?} enabled"
        );
    }
}
