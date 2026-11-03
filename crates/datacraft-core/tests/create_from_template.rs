//! CreateDatabaseFromTemplate command contract (phase 29, step 29.2,
//! item H44).
//!
//! One command materializes a shipped template into a brand-new file: the
//! core creates the file through the port, writes every template table and
//! reports `DatabaseCreated`. The snapshot exposes the shipped templates so
//! GUI, CLI and MCP agents can list them. RED state: the command, the
//! templates module and the snapshot field do not exist yet — the suite
//! fails to compile until they land.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, MockPort};

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-core-template-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn the_blank_template_creates_a_jet4_database_in_one_command() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let path = scratch("blank.mdb");
    let events = core.dispatch(Command::CreateDatabaseFromTemplate {
        path: path.clone(),
        template: "blank".into(),
    });
    assert_eq!(
        events,
        vec![Event::DatabaseCreated {
            path: path.clone(),
            format: DatabaseFormat::Jet4,
        }]
    );
    assert!(core.inspect().database.is_none(), "creation never opens");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn unknown_templates_name_the_available_set() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::CreateDatabaseFromTemplate {
        path: scratch("nope.mdb"),
        template: "nope".into(),
    });
    match events.first() {
        Some(Event::Failed { reason }) => {
            assert!(reason.contains("nope"), "{reason}");
            assert!(
                reason.contains("blank"),
                "names the available set: {reason}"
            );
        }
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

#[test]
fn the_snapshot_lists_the_shipped_templates() {
    let core = Core::new(Box::new(MockPort::new()));
    let templates = &core.inspect().templates;
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        ["blank", "contacts", "inventory", "bike_shop"],
        "the standard set ships in backstage order"
    );
    let blank = templates
        .iter()
        .find(|t| t.name == "blank")
        .expect("blank ships");
    assert_eq!(blank.format, DatabaseFormat::Jet4);
    assert_eq!(blank.tables, 0, "blank carries no tables");
    let bike_shop = templates
        .iter()
        .find(|t| t.name == "bike_shop")
        .expect("the demo ships");
    assert_eq!(bike_shop.tables, 3, "the demo carries its three tables");
}

#[test]
fn template_commands_round_trip_through_json() {
    let command = Command::CreateDatabaseFromTemplate {
        path: "new.mdb".into(),
        template: "blank".into(),
    };
    let json = serde_json::to_string(&command).expect("serialize command");
    assert!(json.contains("\"template\":\"blank\""), "{json}");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(back, command);
}

/// M27 regression: materializing a template must never steal the port
/// session of an open database (the command is rejected while one is open).
#[test]
fn creating_from_a_template_leaves_an_open_database_untouched() {
    use datacraft_core::port::RowRange;
    use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

    let table = TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 20 })],
    );
    let mut port = MockPort::with_tables(vec![table]);
    port.with_row_data("Customers", vec![vec![FieldValue::Text("Ada".into())]]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    let range = RowRange { start: 0, len: 10 };
    let before = core.read_rows("Customers", range).expect("rows before");

    let path = scratch("contacts-while-open.mdb");
    let events = core.dispatch(Command::CreateDatabaseFromTemplate {
        path: path.clone(),
        template: "contacts".into(),
    });
    assert!(
        matches!(
            events.as_slice(),
            [Event::Failed { reason }] if reason.contains("close the open database")
        ),
        "{events:?}"
    );
    assert!(
        core.inspect().database.is_some(),
        "the open database stays in state"
    );
    assert_eq!(
        core.read_rows("Customers", range)
            .expect("the open database stays usable"),
        before,
        "the port session must not be replaced by the template materialization"
    );
}

/// A failing creation reports the port reason and leaves no database.
#[test]
fn a_failing_creation_reports_the_port_reason() {
    let mut port = MockPort::new();
    port.with_write_failure("disk full");
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::CreateDatabaseFromTemplate {
        path: scratch("failing.mdb"),
        template: "blank".into(),
    });
    match events.first() {
        Some(Event::Failed { reason }) => assert!(reason.contains("disk full"), "{reason}"),
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(core.inspect().database.is_none());
}
