//! Native-query flow through the core (coverage gate additions).
//!
//! The core runs mappable native queries over port rows, exposes the
//! read-only inventory and opens native-query tabs; everything outside the
//! decoded subset answers a precise error instead of silence.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, NativeQuery, NativeQueryColumn, NativeQueryKind};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn query(name: &str, criteria: Vec<String>) -> NativeQuery {
    NativeQuery {
        name: name.to_owned(),
        kind: NativeQueryKind::Select,
        tables: vec!["Customers".to_owned()],
        columns: vec![
            NativeQueryColumn {
                table: Some("Customers".to_owned()),
                field: Some("name".to_owned()),
                expression: None,
                descending: false,
            },
            NativeQueryColumn {
                table: Some("Customers".to_owned()),
                field: Some("id".to_owned()),
                expression: None,
                descending: true,
            },
        ],
        criteria,
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }
}

fn core() -> Core {
    let table = TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
        ],
    );
    let mut port = MockPort::with_tables(vec![table]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Integer(2),
            FieldValue::Text("beta".to_owned()),
        ]],
    );
    port.with_native_queries(vec![query("CustomerList", Vec::new())]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core
}

#[test]
fn mappable_native_queries_run_and_open() {
    let mut core = core();
    let result = core.run_query("customerlist").expect("mappable query runs");
    assert_eq!(result.columns, ["name", "id"]);

    let inventory = core
        .native_query("CustomerList")
        .expect("inventory is exposed");
    assert_eq!(inventory.tables, ["Customers"]);

    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Query,
        "CustomerList",
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::TabOpened { .. })),
        "{events:?}"
    );
    assert_eq!(core.inspect().tabs[0].object_kind, "query");
}

#[test]
fn queries_outside_the_subset_answer_a_precise_error() {
    let mut port = MockPort::with_tables(vec![TableDef::new("Customers", vec![])]);
    port.with_native_queries(vec![query("Filtered", vec!["id > 1".to_owned()])]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    let reason = core
        .run_query("Filtered")
        .expect_err("criteria are outside the mapped subset");
    assert!(reason.contains("outside the decoded subset"), "{reason}");
    let reason = core.run_query("Missing").expect_err("unknown query");
    assert!(reason.contains("not found"), "{reason}");
}
