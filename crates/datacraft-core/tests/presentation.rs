//! Contract suite for presentation helpers (step 5.1, item H17).
//!
//! ARCHITECTURE §8: no layout or business logic in paint code — the UI
//! renders strings and models computed here, so they are unit-testable
//! without a window.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::present::status_line;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn open_core(path: &str) -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from(path),
    });
    core
}

fn add_table(core: &mut Core, name: &str) {
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(name, vec![FieldDef::new("id", FieldKind::Currency)]),
    });
}

#[test]
fn closed_core_reports_no_database() {
    let snapshot = Core::new(Box::new(MockPort::new())).inspect();
    assert_eq!(status_line(&snapshot), "No database open");
}

#[test]
fn open_database_states_name_format_and_table_count() {
    let mut core = open_core("shop.mdb");
    let snapshot = core.inspect();
    assert_eq!(status_line(&snapshot), "shop (Jet4) — 0 tables");

    add_table(&mut core, "Customers");
    assert_eq!(status_line(&core.inspect()), "shop (Jet4) — 1 table");

    add_table(&mut core, "Orders");
    assert_eq!(status_line(&core.inspect()), "shop (Jet4) — 2 tables");
}

#[test]
fn format_names_are_human_readable() {
    assert_eq!(
        datacraft_core::port::DatabaseFormat::Jet3.to_string(),
        "Jet3"
    );
    assert_eq!(
        datacraft_core::port::DatabaseFormat::Jet4.to_string(),
        "Jet4"
    );
    assert_eq!(datacraft_core::port::DatabaseFormat::Ace.to_string(), "Ace");
}
