//! Native query execution against the pinned fixture (step 27.1, M12).
//!
//! `queryTestV2000.mdb` carries the real MSysQueries dialect: three-table
//! equality joins with a left-join flavor, an Access boolean WHERE
//! expression (`Or`, `In`), and an ORDER BY entry. The fixture tables are
//! empty, so the contract is "executes with the decoded projection and a
//! precise `Unsupported` for every control-flow flavor".

use std::path::{Path, PathBuf};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn core() -> Core {
    let mut core = Core::new(Box::new(Jet4Port::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: fixture("queryTestV2000.mdb"),
        password: None,
        source: None,
    });
    assert!(
        matches!(events[0], Event::DatabaseOpened { .. }),
        "{events:?}"
    );
    core
}

#[test]
fn the_fixture_select_query_executes_through_the_engine() {
    let mut core = core();
    let result = core
        .run_query("SelectQuery")
        .expect("the decoded select query executes");
    assert_eq!(
        result.columns,
        ["col1", "col2", "col3", "col1", "col2", "col3"],
        "Table1.* plus the three qualified columns"
    );
    assert!(result.rows.is_empty(), "the fixture tables carry no rows");

    let events = core.dispatch(Command::RunQuery {
        query: "selectquery".to_owned(),
    });
    assert!(
        matches!(&events[..], [Event::QueryRan { rows: 0, .. }]),
        "{events:?}"
    );
}

#[test]
fn control_flow_flavors_answer_a_precise_unsupported() {
    let core = core();
    for name in [
        "CrosstabQuery",
        "DeleteQuery",
        "UpdateQuery",
        "AppendQuery",
        "MakeTableQuery",
        "DataDefinitionQuery",
        "PassthroughQuery",
        "UnionQuery",
    ] {
        let reason = core.run_query(name).expect_err("only selects execute");
        assert!(
            reason.contains("select"),
            "{name} must name the executable subset: {reason}"
        );
    }
}
