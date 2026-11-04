//! VBA static-analysis viewer (step 41.3, item M47).
//!
//! The viewer's Analyze affordance dispatches `AnalyzeVba` once on
//! demand and the right pane renders every `VbaAnalysis` section as a
//! read-only list. The kittest drives the real bus over a `MockPort`
//! serving fixture-shaped VBA content — datacraft-ui cannot decode the
//! programmability fixture itself (the dependency direction forbids the
//! jet crate here); the fixture's decode path is pinned in
//! datacraft-jet, the analysis engine in datacraft-core.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, NativeQuery, NativeQueryColumn, NativeQueryKind};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![FieldDef::new(
            "customer_id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )
}

fn project() -> VbaProject {
    VbaProject {
        modules: vec![
            VbaModule {
                name: "Utilities".to_owned(),
                kind: VbaModuleKind::Standard,
                source: concat!(
                    "Attribute VB_Name = \"Utilities\"\n",
                    "Public Sub LoadCustomers()\n",
                    "    DoCmd.RunSQL \"SELECT * FROM Customers\"\n",
                    "    CurrentDb.QueryDefs(\"[OrdersByCity]\")\n",
                    "    Helper()\n",
                    "End Sub\n",
                )
                .to_owned(),
            },
            VbaModule {
                name: "Helper".to_owned(),
                kind: VbaModuleKind::Standard,
                source: concat!(
                    "Attribute VB_Name = \"Helper\"\n",
                    "Public Sub Helper()\n",
                    "End Sub\n",
                )
                .to_owned(),
            },
        ],
    }
}

/// App with the two tables, one native query, the project and the
/// Utilities module tab open.
fn analysis_app() -> (App, datacraft_core::snapshot::TabSnapshot) {
    let mut port = MockPort::with_tables(vec![customers(), orders()]);
    port.with_vba(project());
    port.with_native_queries(vec![NativeQuery {
        name: "OrdersByCity".to_owned(),
        kind: NativeQueryKind::Select,
        tables: vec!["Orders".to_owned()],
        columns: vec![NativeQueryColumn {
            table: Some("Orders".to_owned()),
            field: Some("customer_id".to_owned()),
            expression: None,
            descending: false,
        }],
        criteria: vec![],
        group_by: vec![],
        having: vec![],
        joins: vec![],
        order_by: vec![],
        select_flags: 0,
        parameters: vec![],
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "programmability.accdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Module,
        "Utilities",
    )));
    let tab = app.snapshot().tabs[0].clone();
    (app, tab)
}

fn analysis_harness(
    app: App,
    tab: datacraft_core::snapshot::TabSnapshot,
) -> Harness<'static, (App, datacraft_core::snapshot::TabSnapshot)> {
    Harness::new_ui_state(
        move |ui, (app, tab)| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::vba_viewer::vba_viewer_ui(app, tab, ui);
        },
        (app, tab),
    )
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// Clicking Analyze dispatches one `AnalyzeVba` over the bus and the
/// pane renders every analysis section: call graph, SQL statements,
/// table references, query references and unused modules.
#[test]
fn the_analysis_view_renders_every_section_of_the_result() {
    let (app, tab) = analysis_app();
    let mut harness = analysis_harness(app, tab);
    harness.run();
    let before = tree_dump(&harness);
    assert!(
        !before.contains("Call graph"),
        "no analysis renders before the run:\n{before}"
    );

    harness.get_by_label("Analyze").click();
    harness.run();

    let dump = tree_dump(&harness);
    for section in [
        "Call graph",
        "SQL statements",
        "Table references",
        "Query references",
        "Unused modules",
    ] {
        assert!(
            dump.contains(section),
            "section {section:?} missing:\n{dump}"
        );
    }
    assert!(
        dump.contains("Utilities → Helper"),
        "the call graph lists the module's callee:\n{dump}"
    );
    assert!(
        dump.contains("SELECT * FROM Customers"),
        "the embedded SQL statement is listed:\n{dump}"
    );
    assert!(
        dump.contains("OrdersByCity"),
        "the referenced query is listed:\n{dump}"
    );
    assert!(
        dump.contains("Utilities"),
        "the uncalled standard module is listed as unused:\n{dump}"
    );
    assert!(harness.state().0.last_error().is_none());
}
