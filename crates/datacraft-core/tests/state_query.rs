//! Contract suite for the state query API and test IDs (step 4.2, item H14).
//!
//! Locks ARCHITECTURE §5: `Core::inspect()` returns a serializable snapshot
//! (the headless test seam), test IDs compose from `ObjectId` per the
//! documented convention, and equal states produce byte-identical JSON.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::snapshot::UiStateSnapshot;
use datacraft_core::{ObjectId, ObjectKind};
use proptest::prelude::*;

fn core() -> Core {
    Core::new(Box::new(MockPort::new()))
}

fn customers_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn opened_with_customers() -> Core {
    let mut core = core();
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core
}

// ---------------------------------------------------------------------------
// snapshot structure
// ---------------------------------------------------------------------------

#[test]
fn empty_core_inspects_to_a_closed_snapshot() {
    let snap = core().inspect();
    assert!(snap.database.is_none());
    assert!(snap.nav_pane_visible);
    assert!(snap.tabs.is_empty());
    assert!(snap.active_tab.is_none());
}

#[test]
fn opened_database_is_fully_described() {
    let snap = opened_with_customers().inspect();
    let db = snap.database.as_ref().expect("database open");
    assert_eq!(db.name, "shop");
    assert_eq!(db.path, PathBuf::from("shop.mdb"));
    assert_eq!(db.format, datacraft_core::port::DatabaseFormat::Jet4);
    assert_eq!(db.tables.len(), 1);
    assert_eq!(db.tables[0].name, "Customers");
}

#[test]
fn nav_pane_nodes_carry_the_convention_test_id() {
    let snap = opened_with_customers().inspect();
    let db = snap.database.as_ref().unwrap();
    assert_eq!(
        db.tables[0].ui_test_id,
        "navpane.expanded.node.table.customers"
    );
}

#[test]
fn nav_pane_test_id_reflects_visibility() {
    let mut core = opened_with_customers();
    core.dispatch(Command::ToggleNavPane);
    let db = core.inspect().database.unwrap();
    assert_eq!(
        db.tables[0].ui_test_id,
        "navpane.collapsed.node.table.customers"
    );
}

#[test]
fn tabs_carry_the_convention_test_id_and_kind() {
    let mut core = opened_with_customers();
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    let snap = core.inspect();
    assert_eq!(snap.tabs.len(), 1);
    assert_eq!(snap.tabs[0].object_name, "Customers");
    assert_eq!(snap.tabs[0].object_kind, "table");
    assert_eq!(snap.tabs[0].ui_test_id, "workspace.tab.table.customers");
    assert_eq!(snap.active_tab, Some(snap.tabs[0].id));
}

#[test]
fn active_tab_is_observable_through_the_snapshot() {
    let mut core = opened_with_customers();
    core.dispatch(Command::CreateTable {
        definition: TableDef::new("Orders", vec![FieldDef::new("total", FieldKind::Currency)]),
    });
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Orders",
    )));
    let snap = core.inspect();
    assert_eq!(snap.tabs.len(), 2);
    let active = snap.active_tab.expect("active tab");
    assert_eq!(active, snap.tabs[1].id);

    core.dispatch(Command::SetActiveTab {
        tab_id: snap.tabs[0].id,
    });
    assert_eq!(core.inspect().active_tab, Some(snap.tabs[0].id));
}

#[test]
fn closed_database_clears_the_snapshot() {
    let mut core = opened_with_customers();
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    core.dispatch(Command::CloseDatabase);
    let snap = core.inspect();
    assert!(snap.database.is_none());
    assert!(snap.tabs.is_empty());
    assert!(snap.active_tab.is_none());
}

// ---------------------------------------------------------------------------
// serialization seam
// ---------------------------------------------------------------------------

#[test]
fn snapshot_json_is_deterministic_for_equal_states() {
    let a = opened_with_customers().inspect().to_json();
    let b = opened_with_customers().inspect().to_json();
    assert_eq!(a, b, "equal states must serialize identically");
}

#[test]
fn snapshot_json_round_trips() {
    let mut core = opened_with_customers();
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    let snap = core.inspect();
    let json = snap.to_json();
    let back: UiStateSnapshot = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(back, snap);
}

// ---------------------------------------------------------------------------
// property-based determinism
// ---------------------------------------------------------------------------

proptest! {
    /// Database names never influence determinism: two cores opened on the
    /// same-name file produce byte-identical snapshots.
    #[test]
    fn snapshot_is_deterministic_regardless_of_content(name in "[a-z][a-z0-9]{0,20}") {
        let build = || {
            let mut core = Core::new(Box::new(MockPort::new()));
            core.dispatch(Command::OpenDatabase {
                path: PathBuf::from(format!("{name}.mdb")),
            });
            core.dispatch(Command::CreateTable { definition: customers_def() });
            core.inspect().to_json()
        };
        prop_assert_eq!(build(), build());
    }
}

#[test]
fn table_entries_carry_their_field_names() {
    let snap = opened_with_customers().inspect();
    let db = snap.database.expect("database open");
    let names: Vec<&str> = db.tables[0].fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["id", "name"]);
}
