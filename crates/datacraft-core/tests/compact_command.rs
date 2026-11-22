//! Compact through the command bus (phase 95, step 95.2; item M126).
//!
//! One command, one capability gate, one observable event — the bus
//! contract every client shares. The real Jet/ACE rewrite is pinned by
//! `datacraft-jet/tests/jet_compact.rs`.

use datacraft_core::availability::DisabledReason;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{DriverCapabilities, MockPort, Support};

fn open(port: MockPort) -> Core {
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. })),
        "the mock opens: {events:?}"
    );
    core
}

#[test]
fn compact_reports_the_compacted_database() {
    let mut core = open(MockPort::new());
    let events = core.dispatch(Command::CompactDatabase);
    assert!(
        matches!(
            events.as_slice(),
            [Event::DatabaseCompacted { name }] if name == "memory"
        ),
        "{events:?}"
    );
    // The session stays open and the tabs survive (the mock has none here).
    assert!(core.inspect().database.is_some());
}

#[test]
fn compact_is_refused_without_the_capability() {
    let mut port = MockPort::new();
    port.with_capabilities(DriverCapabilities {
        compact: Support::No,
        ..DriverCapabilities::full()
    });
    let mut core = open(port);
    let events = core.dispatch(Command::CompactDatabase);
    match events.as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Capability(DisabledReason::Compact));
            assert!(reason.contains("compact"), "{reason}");
        }
        other => panic!("expected one capability failure, got {other:?}"),
    }
    let database = core.inspect().database.expect("still open");
    assert_eq!(
        database.availability.get("maintenance.compact"),
        Some(&DisabledReason::Compact),
        "the affordance map names the reason"
    );
}

#[test]
fn compact_without_a_database_is_a_precise_failure() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::CompactDatabase);
    match events.as_slice() {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NoDatabase),
        other => panic!("expected a NoDatabase failure, got {other:?}"),
    }
}
