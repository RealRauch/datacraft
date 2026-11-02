//! Native saved-query execution (step 27.1, item M12).
//!
//! The decoder has carried the full MSysQueries vocabulary since M3; this
//! suite pins the executable subset: select queries over one or more
//! tables with equality joins (inner/left/right), Access boolean criteria,
//! wildcard projection and ORDER BY. Everything outside that subset must
//! answer a precise `Unsupported` instead of a silent empty result.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{
    MockPort, NativeJoin, NativeJoinKind, NativeOrder, NativeQuery, NativeQueryColumn,
    NativeQueryKind,
};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn text(name: &str) -> FieldDef {
    FieldDef::new(name, FieldKind::Text { max_len: 40 })
}

fn number(name: &str) -> FieldDef {
    FieldDef::new(name, FieldKind::Number(NumericKind::LongInteger))
}

fn column(expression: &str) -> NativeQueryColumn {
    NativeQueryColumn {
        table: None,
        field: None,
        expression: Some(expression.to_owned()),
        descending: false,
    }
}

fn join(expression: &str, kind: NativeJoinKind) -> NativeJoin {
    NativeJoin {
        expression: expression.to_owned(),
        kind,
    }
}

fn query(tables: &[&str], columns: Vec<NativeQueryColumn>) -> NativeQuery {
    NativeQuery {
        name: "Q".to_owned(),
        kind: NativeQueryKind::Select,
        tables: tables.iter().map(|table| (*table).to_owned()).collect(),
        columns,
        criteria: Vec::new(),
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }
}

fn shop_core(native: NativeQuery) -> Core {
    let customers = TableDef::new("Customers", vec![number("id"), text("name")]);
    let orders = TableDef::new(
        "Orders",
        vec![number("id"), number("customer_id"), number("total")],
    );
    let mut port = MockPort::with_tables(vec![customers, orders]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("alpha".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("beta".into())],
            vec![FieldValue::Integer(3), FieldValue::Text("gamma".into())],
        ],
    );
    port.with_row_data(
        "Orders",
        vec![
            vec![
                FieldValue::Integer(10),
                FieldValue::Integer(1),
                FieldValue::Integer(100),
            ],
            vec![
                FieldValue::Integer(11),
                FieldValue::Integer(1),
                FieldValue::Integer(250),
            ],
            vec![
                FieldValue::Integer(12),
                FieldValue::Integer(2),
                FieldValue::Integer(50),
            ],
            vec![
                FieldValue::Integer(13),
                FieldValue::Integer(9),
                FieldValue::Integer(999),
            ],
        ],
    );
    port.with_native_queries(vec![native]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core
}

#[test]
fn joined_select_applies_where_and_order() {
    let mut native = query(
        &["Customers", "Orders"],
        vec![column("Customers.*"), column("Orders.total")],
    );
    native.joins.push(join(
        "Customers.id = Orders.customer_id",
        NativeJoinKind::Inner,
    ));
    native.criteria.push("Orders.total > 60".to_owned());
    native.order_by.push(NativeOrder {
        field: "Orders.total".to_owned(),
        descending: true,
    });
    let mut core = shop_core(native);

    let result = core.run_query("q").expect("the joined select runs");
    assert_eq!(result.columns, ["id", "name", "total"]);
    assert_eq!(
        result.rows,
        vec![
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("alpha".into()),
                FieldValue::Integer(250),
            ],
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("alpha".into()),
                FieldValue::Integer(100),
            ],
        ]
    );

    let events = core.dispatch(Command::RunQuery {
        query: "Q".to_owned(),
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::QueryRan { rows: 2, .. })),
        "{events:?}"
    );
}

#[test]
fn left_join_keeps_unmatched_preserved_rows() {
    let mut native = query(
        &["Customers", "Orders"],
        vec![column("Customers.*"), column("Orders.total")],
    );
    native.joins.push(join(
        "Customers.id = Orders.customer_id",
        NativeJoinKind::Left,
    ));
    native.order_by.push(NativeOrder {
        field: "Customers.id".to_owned(),
        descending: false,
    });
    let core = shop_core(native);

    let result = core.run_query("Q").expect("the left join runs");
    assert_eq!(result.columns, ["id", "name", "total"]);
    assert_eq!(
        result.rows,
        vec![
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("alpha".into()),
                FieldValue::Integer(100),
            ],
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("alpha".into()),
                FieldValue::Integer(250),
            ],
            vec![
                FieldValue::Integer(2),
                FieldValue::Text("beta".into()),
                FieldValue::Integer(50),
            ],
            vec![
                FieldValue::Integer(3),
                FieldValue::Text("gamma".into()),
                FieldValue::Null,
            ],
        ]
    );
}

#[test]
fn right_join_preserves_the_right_operand() {
    let mut native = query(
        &["Orders", "Customers"],
        vec![column("Orders.total"), column("Customers.name")],
    );
    native.joins.push(join(
        "Orders.customer_id = Customers.id",
        NativeJoinKind::Right,
    ));
    let core = shop_core(native);

    let result = core.run_query("Q").expect("the right join runs");
    assert_eq!(result.columns, ["total", "name"]);
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Integer(100), FieldValue::Text("alpha".into())],
            vec![FieldValue::Integer(250), FieldValue::Text("alpha".into())],
            vec![FieldValue::Integer(50), FieldValue::Text("beta".into())],
            vec![FieldValue::Null, FieldValue::Text("gamma".into())],
        ]
    );
}

#[test]
fn criteria_accept_access_boolean_syntax() {
    let mut native = query(&["Customers"], vec![column("Customers.name")]);
    native
        .criteria
        .push("(Customers.name = \"alpha\" Or Customers.name In (\"gamma\"))".to_owned());
    let core = shop_core(native);
    let result = core.run_query("Q").expect("the criteria run");
    assert_eq!(result.columns, ["name"]);
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Text("alpha".into())],
            vec![FieldValue::Text("gamma".into())],
        ]
    );
}

#[test]
fn bracketed_references_and_is_not_null_run() {
    let mut native = query(&["Customers"], vec![column("[Customers].[name]")]);
    native
        .criteria
        .push("[Customers].[name] Is Not Null And [Customers].[id] > 1".to_owned());
    let core = shop_core(native);
    let result = core.run_query("Q").expect("the bracketed references run");
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Text("beta".into())],
            vec![FieldValue::Text("gamma".into())],
        ]
    );
}

#[test]
fn wildcard_projection_expands_each_table() {
    let mut native = query(&["Customers", "Orders"], vec![column("Customers.*")]);
    native.joins.push(join(
        "Customers.id = Orders.customer_id",
        NativeJoinKind::Inner,
    ));
    let core = shop_core(native);
    let result = core.run_query("Q").expect("the star projection runs");
    assert_eq!(result.columns, ["id", "name"]);
    assert_eq!(result.rows.len(), 3, "only matching customers join");
}

#[test]
fn an_empty_result_is_not_an_error() {
    let mut native = query(&["Customers"], vec![column("Customers.*")]);
    native.criteria.push("Customers.id > 100".to_owned());
    let core = shop_core(native);
    let result = core.run_query("Q").expect("an empty result is valid");
    assert!(result.rows.is_empty());
    assert_eq!(result.columns, ["id", "name"]);
}

#[test]
fn non_select_kinds_are_unsupported() {
    for kind in [
        NativeQueryKind::Crosstab,
        NativeQueryKind::Delete,
        NativeQueryKind::Update,
        NativeQueryKind::Append,
        NativeQueryKind::MakeTable,
        NativeQueryKind::DataDefinition,
        NativeQueryKind::Passthrough,
        NativeQueryKind::Union,
    ] {
        let mut native = query(&["Customers"], vec![column("Customers.*")]);
        native.kind = kind;
        let core = shop_core(native);
        let reason = core.run_query("Q").expect_err("only selects execute");
        assert!(
            reason.contains("select"),
            "kind {kind:?} must name the executable subset: {reason}"
        );
    }
}

#[test]
fn grouping_and_parameters_are_unsupported() {
    let mut grouped = query(&["Customers"], vec![column("Customers.name")]);
    grouped.group_by.push("Customers.name".to_owned());
    let reason = shop_core(grouped)
        .run_query("Q")
        .expect_err("grouping is outside the subset");
    assert!(reason.contains("group"), "{reason}");

    let mut having = query(&["Customers"], vec![column("Customers.name")]);
    having.having.push("Count(*) > 1".to_owned());
    let reason = shop_core(having)
        .run_query("Q")
        .expect_err("HAVING is outside the subset");
    assert!(
        reason.contains("having") || reason.contains("group"),
        "{reason}"
    );

    let mut parameterized = query(&["Customers"], vec![column("Customers.name")]);
    parameterized.parameters.push("Which".to_owned());
    let reason = shop_core(parameterized)
        .run_query("Q")
        .expect_err("parameters are outside the subset");
    assert!(reason.contains("parameter"), "{reason}");
}

#[test]
fn select_modifiers_are_unsupported() {
    let mut distinct = query(&["Customers"], vec![column("Customers.name")]);
    distinct.select_flags = 0x02;
    let reason = shop_core(distinct)
        .run_query("Q")
        .expect_err("distinct is outside the subset");
    assert!(reason.contains("distinct"), "{reason}");

    let mut top = query(&["Customers"], vec![column("Customers.name")]);
    top.select_flags = 0x10;
    let reason = shop_core(top)
        .run_query("Q")
        .expect_err("top is outside the subset");
    assert!(
        reason.contains("top") || reason.contains("modifier"),
        "{reason}"
    );
}

#[test]
fn computed_columns_are_unsupported() {
    let native = query(&["Customers"], vec![column("Max(Customers.id)")]);
    let reason = shop_core(native)
        .run_query("Q")
        .expect_err("computed columns are outside the subset");
    assert!(reason.contains("computed"), "{reason}");
}

#[test]
fn unknown_source_tables_are_named() {
    let native = query(&["Ghost"], vec![column("Ghost.*")]);
    let core = shop_core(native);
    let reason = core.run_query("Q").expect_err("the table does not exist");
    assert!(reason.contains("Ghost"), "{reason}");
}

#[test]
fn ambiguous_bare_fields_are_refused() {
    let mut native = query(&["Customers", "Orders"], vec![column("id")]);
    native.joins.push(join(
        "Customers.id = Orders.customer_id",
        NativeJoinKind::Inner,
    ));
    let core = shop_core(native);
    let reason = core.run_query("Q").expect_err("id exists in both tables");
    assert!(reason.contains("ambiguous"), "{reason}");
}

#[test]
fn joins_must_be_qualified_equalities() {
    let mut inequality = query(&["Customers", "Orders"], vec![column("Customers.name")]);
    inequality.joins.push(join(
        "Customers.id > Orders.customer_id",
        NativeJoinKind::Inner,
    ));
    let reason = shop_core(inequality)
        .run_query("Q")
        .expect_err("only equality joins execute");
    assert!(reason.contains("equality"), "{reason}");

    let mut bare = query(&["Customers", "Orders"], vec![column("Customers.name")]);
    bare.joins
        .push(join("id = customer_id", NativeJoinKind::Inner));
    let reason = shop_core(bare)
        .run_query("Q")
        .expect_err("join operands must be qualified");
    assert!(reason.contains("qualified"), "{reason}");
}

#[test]
fn duplicate_from_tables_are_refused() {
    let native = query(&["Customers", "Customers"], vec![column("Customers.*")]);
    let reason = shop_core(native)
        .run_query("Q")
        .expect_err("self-joins need an alias and stay unsupported");
    assert!(
        reason.contains("duplicate") || reason.contains("self"),
        "{reason}"
    );
}
