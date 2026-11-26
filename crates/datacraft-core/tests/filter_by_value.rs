//! Filter by value (phase 101, step 101.1; item M132; roadmap §16 stage
//! 1).
//!
//! A column's value list comes from its cells: bounded, deduplicated and
//! ordered by the datasheet's own total comparator. Picking one value
//! filters the view without typing criteria; the blank entry matches
//! `Null`. The filter is a typed value filter next to the criteria
//! filters — a column carries one of the two — and matching compares the
//! cell's rendered text, which is what the user picked from the list.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::state::ValueFilter;
use datacraft_core::table_view::{VALUE_LIST_LIMIT, view_order};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Qty", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn ready_core(rows: Vec<Vec<FieldValue>>) -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", rows);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

fn sample_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![FieldValue::Text("Bob".into()), FieldValue::Integer(1)],
        vec![FieldValue::Text("Ada".into()), FieldValue::Integer(3)],
        vec![FieldValue::Null, FieldValue::Integer(5)],
        vec![FieldValue::Text("Ada".into()), FieldValue::Integer(2)],
    ]
}

fn values_of(core: &mut Core, field: &str) -> Event {
    let events = core.dispatch(Command::ListColumnValues {
        table: "Customers".into(),
        field: field.into(),
    });
    events.into_iter().next().expect("one event")
}

#[test]
fn the_value_list_is_ordered_and_deduplicated_and_reports_blanks() {
    let mut core = ready_core(sample_rows());
    match values_of(&mut core, "Name") {
        Event::ColumnValuesListed {
            table,
            field,
            values,
            blank,
            truncated,
        } => {
            assert_eq!(table, "Customers");
            assert_eq!(field, "Name");
            assert_eq!(values, ["Ada", "Bob"], "ordered by the total comparator");
            assert!(blank, "the column carries a Null cell");
            assert!(!truncated);
        }
        other => panic!("expected the value list, got {other:?}"),
    }
}

#[test]
fn the_list_carries_no_blank_flag_for_a_null_free_column() {
    let mut core = ready_core(sample_rows());
    match values_of(&mut core, "Qty") {
        Event::ColumnValuesListed { values, blank, .. } => {
            assert_eq!(values, ["1", "2", "3", "5"], "the rendered numbers");
            assert!(!blank);
        }
        other => panic!("expected the value list, got {other:?}"),
    }
}

#[test]
fn the_list_is_bounded_and_reports_truncation() {
    let rows: Vec<Vec<FieldValue>> = (0..VALUE_LIST_LIMIT + 5)
        .map(|index| {
            vec![
                FieldValue::Text(format!("name{index:04}")),
                FieldValue::Integer(index as i64),
            ]
        })
        .collect();
    let mut core = ready_core(rows);
    match values_of(&mut core, "Name") {
        Event::ColumnValuesListed {
            values, truncated, ..
        } => {
            assert_eq!(values.len(), VALUE_LIST_LIMIT);
            assert!(truncated, "more values existed than the list carries");
            assert_eq!(values[0], "name0000");
            assert_eq!(
                values[VALUE_LIST_LIMIT - 1],
                format!("name{:04}", VALUE_LIST_LIMIT - 1)
            );
        }
        other => panic!("expected the value list, got {other:?}"),
    }
}

#[test]
fn filtering_by_a_value_keeps_only_its_rows() {
    let mut core = ready_core(sample_rows());
    let events = core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Name".into(),
        value: Some("Ada".into()),
    });
    assert!(events.is_empty(), "setting a filter is silent: {events:?}");
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    assert_eq!(
        view.value_filters.get("Name"),
        Some(&ValueFilter::Text("Ada".into()))
    );
    let order = view_order(
        &customers(),
        &sample_rows(),
        None,
        &view.filters,
        &view.value_filters,
    )
    .expect("the filtered order");
    assert_eq!(order, [1, 3], "both Ada rows survive, in cache order");
}

#[test]
fn filtering_by_the_blank_entry_keeps_the_null_rows() {
    let mut core = ready_core(sample_rows());
    core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Name".into(),
        value: None,
    });
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    assert_eq!(view.value_filters.get("Name"), Some(&ValueFilter::Blank));
    let order = view_order(
        &customers(),
        &sample_rows(),
        None,
        &view.filters,
        &view.value_filters,
    )
    .expect("the filtered order");
    assert_eq!(order, [2], "only the Null row survives");
}

#[test]
fn filtering_matches_the_rendered_text() {
    // Two rows whose cells differ but render alike share one list entry:
    // the filter matches what the user picked from the list.
    let rows = vec![
        vec![FieldValue::Text("a".into()), FieldValue::Float(1.0)],
        vec![FieldValue::Text("b".into()), FieldValue::Float(1.0)],
    ];
    let mut core = ready_core(rows.clone());
    core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Qty".into(),
        value: Some("1".into()),
    });
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    let order = view_order(
        &customers(),
        &rows,
        None,
        &view.filters,
        &view.value_filters,
    )
    .expect("the filtered order");
    assert_eq!(order, [0, 1], "both cells render as \"1\"");
}

#[test]
fn a_value_outside_the_list_is_refused() {
    let mut core = ready_core(sample_rows());
    match core
        .dispatch(Command::FilterByValue {
            table: "Customers".into(),
            field: "Name".into(),
            value: Some("Grace".into()),
        })
        .as_slice()
    {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::NotFound);
            assert!(reason.contains("Grace"), "{reason}");
            assert!(reason.contains("Name"), "{reason}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"));
    assert!(
        view.map(|view| view.value_filters.is_empty())
            .unwrap_or(true),
        "a refused filter leaves the view unchanged"
    );
}

#[test]
fn a_criteria_and_a_value_filter_replace_each_other() {
    let mut core = ready_core(sample_rows());
    core.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "Name".into(),
        criteria: Some("= \"Ada\"".into()),
    });
    core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Name".into(),
        value: Some("Bob".into()),
    });
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    assert!(view.filters.is_empty(), "the criteria made room");
    assert_eq!(
        view.value_filters.get("Name"),
        Some(&ValueFilter::Text("Bob".into()))
    );
    // And back: typing a criteria clears the picked value.
    core.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "Name".into(),
        criteria: Some("= \"Ada\"".into()),
    });
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    assert_eq!(view.filters.get("Name"), Some(&"= \"Ada\"".to_owned()));
    assert!(view.value_filters.is_empty());
}

#[test]
fn clearing_the_filter_clears_both_kinds() {
    let mut core = ready_core(sample_rows());
    core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Name".into(),
        value: Some("Ada".into()),
    });
    core.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "Qty".into(),
        criteria: Some("> 2".into()),
    });
    core.dispatch(Command::ClearTableFilters {
        table: "Customers".into(),
    });
    let view = core
        .state()
        .database
        .as_ref()
        .and_then(|db| db.table_views.get("Customers"))
        .expect("the view state");
    assert!(view.filters.is_empty());
    assert!(view.value_filters.is_empty());
}

#[test]
fn an_unknown_column_lists_and_filters_precisely() {
    let mut core = ready_core(sample_rows());
    match values_of(&mut core, "Missing") {
        Event::Failed { code, .. } => assert_eq!(code, FailureCode::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
    match core
        .dispatch(Command::FilterByValue {
            table: "Customers".into(),
            field: "Missing".into(),
            value: Some("x".into()),
        })
        .as_slice()
    {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a_blank_filter_is_refused_where_the_column_has_no_null() {
    let rows = vec![vec![FieldValue::Text("Ada".into()), FieldValue::Integer(1)]];
    let mut core = ready_core(rows);
    match core
        .dispatch(Command::FilterByValue {
            table: "Customers".into(),
            field: "Name".into(),
            value: None,
        })
        .as_slice()
    {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::NotFound);
            assert!(reason.contains("blank"), "{reason}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
}

#[test]
fn the_value_filter_rides_the_tab_snapshot() {
    let mut core = ready_core(sample_rows());
    core.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    core.dispatch(Command::FilterByValue {
        table: "Customers".into(),
        field: "Name".into(),
        value: Some("Ada".into()),
    });
    let snapshot = core.inspect();
    let tab = snapshot.tabs.first().expect("a table tab");
    assert_eq!(
        tab.value_filters.get("Name"),
        Some(&ValueFilter::Text("Ada".into()))
    );
    // The wire carries it as a tagged value.
    let wire = serde_json::to_value(&snapshot).expect("serialises");
    assert_eq!(
        wire["tabs"][0]["value_filters"]["Name"]["text"], "Ada",
        "{wire}"
    );
}
