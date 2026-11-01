//! Contract suite for native-query decode errors at open (step 23.3, M10).
//!
//! A database whose `MSysQueries` carries an undecoded attribute still
//! opens (tables are readable), but the precise reason is carried in the
//! snapshot instead of being dropped (LESSONS #4).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;

fn open(core: &mut Core) -> Vec<Event> {
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("northwind.accdb"),
        password: None,
    })
}

#[test]
fn a_native_query_decode_error_still_opens_the_database() {
    let mut port = MockPort::new();
    port.with_native_queries_error("query \"q\": MSysQueries attribute code 12 is not decoded");
    let mut core = Core::new(Box::new(port));
    let events = open(&mut core);
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "the open succeeds: {events:?}"
    );
    let database = core.inspect().database.expect("database snapshot");
    assert!(
        database.queries.is_empty(),
        "no partially decoded list is fabricated"
    );
    let reason = database
        .native_queries_error
        .expect("the precise decode error is carried");
    assert!(reason.contains("attribute code 12"), "{reason}");
}

#[test]
fn a_decodable_database_carries_no_native_query_error() {
    let mut core = Core::new(Box::new(MockPort::new()));
    open(&mut core);
    let database = core.inspect().database.expect("database snapshot");
    assert!(database.queries.is_empty());
    assert!(database.native_queries_error.is_none());
}

#[test]
fn the_error_crosses_the_inspection_wire() {
    let mut port = MockPort::new();
    port.with_native_queries_error("query \"q\": MSYSQUERIES attribute code 12");
    let mut core = Core::new(Box::new(port));
    open(&mut core);
    let json = datacraft_core::transport::handle_json(
        &mut core,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"inspect\"}",
    );
    assert!(
        json.contains("native_queries_error"),
        "inspect carries the field: {json}"
    );
    assert!(json.contains("attribute code 12"), "{json}");
}
