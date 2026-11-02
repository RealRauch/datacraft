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
    assert!(!templates.is_empty(), "the standard set ships");
    let blank = templates
        .iter()
        .find(|t| t.name == "blank")
        .expect("blank ships");
    assert_eq!(blank.format, DatabaseFormat::Jet4);
    assert_eq!(blank.tables, 0, "blank carries no tables");
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
