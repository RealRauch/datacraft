//! Table datasheet sort/filter contracts (step 44.5, item M54): the
//! table view state is core state (`SetTableSort`/`SetTableFilter`/
//! `ClearTableFilters`), projected into the tab snapshot, and the
//! display order is computed in core (`table_view::view_order`) so no
//! client re-implements the row math.

use std::collections::BTreeMap;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::SortOrder;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::state::TabMode;
use datacraft_core::table_view::view_order;

fn orders_table() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("item", FieldKind::Text { max_len: 50 }),
            FieldDef::new("total", FieldKind::Currency),
        ],
    )
}

fn core_with_table() -> Core {
    let port = MockPort::with_tables(vec![orders_table()]);
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: std::path::PathBuf::from("shop.mdb"),
        password: None,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    core
}

fn open_table_tab(core: &mut Core) {
    core.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
}

/// Sorting and filtering are core state projected into the tab
/// snapshot; each setter is one command, and the clear commands empty
/// them again.
#[test]
fn table_view_commands_store_and_clear_sort_and_filters() {
    let mut core = core_with_table();
    open_table_tab(&mut core);

    let failed = core.dispatch(Command::SetTableSort {
        table: "Orders".into(),
        field: "total".into(),
        order: SortOrder::Descending,
    });
    assert!(!failed.iter().any(|e| matches!(e, Event::Failed { .. })));

    let failed = core.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "item".into(),
        criteria: Some("Like \"B*\"".into()),
    });
    assert!(!failed.iter().any(|e| matches!(e, Event::Failed { .. })));

    let tab = &core.inspect().tabs[0];
    assert_eq!(tab.sort_field.as_deref(), Some("total"));
    assert_eq!(tab.sort_order, Some(SortOrder::Descending));
    assert_eq!(
        tab.filters.get("item").map(String::as_str),
        Some("Like \"B*\""),
        "{:?}",
        tab.filters
    );

    // Clearing: sort order None empties the sort; a None criteria drops
    // that one filter; ClearTableFilters empties the rest.
    core.dispatch(Command::SetTableSort {
        table: "Orders".into(),
        field: "total".into(),
        order: SortOrder::None,
    });
    core.dispatch(Command::ClearTableFilters {
        table: "Orders".into(),
    });
    let tab = &core.inspect().tabs[0];
    assert_eq!(tab.sort_field, None);
    assert_eq!(tab.sort_order, None);
    assert!(tab.filters.is_empty(), "{:?}", tab.filters);
}

/// A single-column filter clears with a None criteria; other columns
/// keep theirs.
#[test]
fn a_none_criteria_clears_only_its_field() {
    let mut core = core_with_table();
    open_table_tab(&mut core);
    for (field, criteria) in [("item", "Like \"B*\""), ("total", "> 100")] {
        core.dispatch(Command::SetTableFilter {
            table: "Orders".into(),
            field: field.into(),
            criteria: Some(criteria.into()),
        });
    }
    core.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "item".into(),
        criteria: None,
    });
    let tab = &core.inspect().tabs[0];
    assert!(!tab.filters.contains_key("item"));
    assert!(tab.filters.contains_key("total"));
}

/// Violations answer precise failures: unknown tables and fields, and
/// criteria that do not parse for the column's kind.
#[test]
fn view_command_violations_fail_precisely() {
    let mut core = core_with_table();
    open_table_tab(&mut core);

    let events = core.dispatch(Command::SetTableSort {
        table: "Ghost".into(),
        field: "id".into(),
        order: SortOrder::Ascending,
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Failed { reason, .. } if reason.contains("Ghost"))),
        "{events:?}"
    );

    let events = core.dispatch(Command::SetTableSort {
        table: "Orders".into(),
        field: "ghost".into(),
        order: SortOrder::Ascending,
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Failed { reason, .. } if reason.contains("ghost"))),
        "{events:?}"
    );

    let events = core.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "id".into(),
        criteria: Some("no-parse-here".into()),
    });
    assert!(
        events.iter().any(|e| matches!(e, Event::Failed { .. })),
        "unparseable criteria fail at set time: {events:?}"
    );
    assert!(
        core.inspect().tabs[0].filters.is_empty(),
        "a failed set stores nothing"
    );
}

/// The display order computes in core: filters AND together, the sort
/// then orders the survivors with the engine's typed compare, and the
/// result is a display→cache index map.
#[test]
fn view_order_filters_then_sorts_into_display_indices() {
    let table = orders_table();
    let rows = vec![
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("bolt".into()),
            FieldValue::Currency(250),
        ],
        vec![
            FieldValue::Integer(2),
            FieldValue::Text("brake".into()),
            FieldValue::Currency(90),
        ],
        vec![
            FieldValue::Integer(3),
            FieldValue::Text("chain".into()),
            FieldValue::Currency(120),
        ],
    ];
    let mut filters = BTreeMap::new();
    filters.insert("item".to_owned(), "Like \"B*\"".to_owned());
    let order = view_order(
        &table,
        &rows,
        Some(("total", SortOrder::Ascending)),
        &filters,
    )
    .expect("runs");
    // Both "b" rows survive the filter, ordered by total ascending.
    assert_eq!(order, vec![1, 0], "brake(90) before bolt(250)");

    // Without a view model the identity order comes back.
    let identity = view_order(&table, &rows, None, &BTreeMap::new()).expect("runs");
    assert_eq!(identity, vec![0, 1, 2]);

    // Violations surface precisely.
    let mut filters = BTreeMap::new();
    filters.insert("ghost".to_owned(), "1".to_owned());
    let err = view_order(&table, &rows, None, &filters).expect_err("unknown field");
    assert!(
        matches!(
            err,
            datacraft_core::query_engine::EngineError::UnknownField { .. }
        ),
        "{err}"
    );
}

/// Closing the database drops the view state with the rest of the
/// session; a fresh open starts unsorted and unfiltered.
#[test]
fn closing_the_database_drops_the_table_view_state() {
    let mut core = core_with_table();
    open_table_tab(&mut core);
    core.dispatch(Command::SetTableSort {
        table: "Orders".into(),
        field: "total".into(),
        order: SortOrder::Descending,
    });
    core.dispatch(Command::CloseDatabase);
    core.dispatch(Command::OpenDatabase {
        path: std::path::PathBuf::from("shop.mdb"),
        password: None,
    });
    open_table_tab(&mut core);
    let tab = &core.inspect().tabs[0];
    assert_eq!(tab.sort_field, None);
    assert!(tab.filters.is_empty());
    let _ = TabMode::Datasheet; // state import kept for the tab projections
}
