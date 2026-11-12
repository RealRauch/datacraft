//! File facts and the health warning in the core state (phase 73, step
//! 73.3; item M91).
//!
//! The port reports driver facts and a health verdict; the snapshot
//! carries both, and an unhealthy file emits one precise warning event
//! beside the successful open (every client surfaces it).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, SourceFact, SourceFacts};

fn open_with(port: MockPort) -> Core {
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.sqlite"),
        password: None,
        source: None,
    });
    core
}

fn facts() -> SourceFacts {
    SourceFacts {
        facts: vec![
            SourceFact {
                name: "sqlite_version".into(),
                value: "3.53.4".into(),
            },
            SourceFact {
                name: "page_size".into(),
                value: "4096".into(),
            },
        ],
        health: None,
    }
}

#[test]
fn the_snapshot_carries_the_facts() {
    let mut port = MockPort::new();
    port.with_facts(facts());
    let core = open_with(port);
    let database = core.inspect().database.expect("open");
    assert_eq!(database.facts.len(), 2);
    assert_eq!(database.facts[0].name, "sqlite_version");
    assert_eq!(database.health, None);
}

#[test]
fn an_unhealthy_file_emits_one_precise_warning() {
    let mut port = MockPort::new();
    port.with_facts(SourceFacts {
        facts: facts().facts,
        health: Some("quick_check: *** in database main *** page 3: btree init".into()),
    });
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.sqlite"),
        password: None,
        source: None,
    });
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::SourceHealthWarning { reason } if reason.contains("quick_check")
        )),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. })),
        "the open still succeeds: {events:?}"
    );
    let database = core.inspect().database.expect("open");
    assert!(database.health.is_some());
}

#[test]
fn drivers_without_facts_report_an_empty_report() {
    let core = open_with(MockPort::new());
    let database = core.inspect().database.expect("open");
    assert!(database.facts.is_empty());
    assert_eq!(database.health, None);
}
