//! Filter builder UI suite (step 87.4, item M104).
//!
//! The builder window edits a draft — editing dispatches nothing until
//! Apply; Apply serializes through the core model (the dialect fence
//! refuses an invalid draft precisely and keeps the dialog open); the
//! text mode edits the raw criteria as the escape hatch; reopening
//! seeds from the live filter map (the builder-to-criteria round
//! trip); and the sort line dispatches the existing single-column
//! sort. Assertions stay on the snapshot and the semantic tree.

use std::collections::BTreeMap;

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new(
                "amount",
                FieldKind::Number(NumericKind::LongInteger),
            ),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn orders_app() -> App {
    let mut port = MockPort::with_tables(vec![orders()]);
    port.with_row_data(
        "Orders",
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(100), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(11), FieldValue::Integer(250), FieldValue::Text("Grace".into())],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    app
}

fn harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let snapshot = app.snapshot();
            let tab = snapshot
                .tabs
                .iter()
                .find(|tab| Some(tab.id) == snapshot.active_tab)
                .cloned();
            if let Some(tab) = tab {
                datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
            }
        },
        app,
    )
}

fn right_click_cell(harness: &mut Harness<'static, App>, label: &str) {
    let rect = harness.get_by_label(label).rect();
    harness.hover_at(rect.center());
    let press = |pressed| egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
}

/// Opens the builder from the header context menu of the first column.
fn open_builder(harness: &mut Harness<'static, App>) {
    harness.get_by_label("     1").click();
    harness.run();
    right_click_cell(harness, "10");
    harness.get_by_label("Filter Builder").click();
    harness.run();
    harness.run();
}

/// The live filter map of the Orders tab.
fn orders_filters(harness: &Harness<'static, App>) -> BTreeMap<String, String> {
    harness
        .state()
        .snapshot()
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .map(|tab| tab.filters.clone())
        .expect("the Orders tab is open")
}

/// A builder condition line: the column picker, operator picker and the
/// value edit of the row at `row`.
fn fill_condition(
    harness: &mut Harness<'static, App>,
    row: usize,
    column: &str,
    op: &str,
    value: &str,
) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("Value — {row}"))
        .set_text_value_accesskit(value);
    harness.run();
    // The column pickers read "Column — {row}"; open and choose.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, &format!("Column — {row}"))
        .click();
    harness.run();
    harness.get_by_label(column).click();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, &format!("Operator — {row}"))
        .click();
    harness.run();
    harness.get_by_label(op).click();
    harness.run();
}

#[test]
fn builder_applies_conditions_and_round_trips() {
    let app = orders_app();
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);

    fill_condition(&mut harness, 1, "amount", ">", "100");
    harness.get_by_label("Apply").click();
    harness.run();
    harness.run();

    let filters = orders_filters(&harness);
    assert_eq!(
        filters,
        BTreeMap::from([("amount".to_owned(), "> 100".to_owned())]),
        "apply dispatches the serialized criteria"
    );

    // The round trip: reopening seeds from the live map and renders the
    // parenthesized form of the model.
    open_builder(&mut harness);
    assert!(
        harness.query_by_label_contains("amount > 100").is_some(),
        "the seeded draft renders the model's parenthesized form"
    );
}

#[test]
fn editing_the_draft_dispatches_nothing() {
    let app = orders_app();
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);
    fill_condition(&mut harness, 1, "amount", ">", "100");
    harness.run();

    assert!(
        orders_filters(&harness).is_empty(),
        "editing the draft never runs a query"
    );
}

#[test]
fn text_mode_edits_the_raw_criteria() {
    let app = orders_app();
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);
    harness.get_by_label("Text").click();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "criteria — amount")
        .set_text_value_accesskit("> 100");
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();
    harness.run();

    let filters = orders_filters(&harness);
    assert_eq!(
        filters,
        BTreeMap::from([("amount".to_owned(), "> 100".to_owned())]),
        "the text mode keeps the raw dialect"
    );
}

#[test]
fn unmodelable_live_filter_seeds_the_text_mode() {
    let mut app = orders_app();
    app.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "amount".into(),
        criteria: Some("Between 1 and 9".into()),
    });
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);

    // The live criteria is outside the builder's model — the window
    // opens in the text mode carrying the raw form.
    let value = harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "criteria — amount")
        .accesskit_node()
        .value()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(value, "Between 1 and 9", "the text mode seeds the raw form");
}

#[test]
fn duplicate_columns_apply_refuses_precisely() {
    let app = orders_app();
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);
    fill_condition(&mut harness, 1, "amount", "=", "1");
    fill_condition(&mut harness, 2, "amount", "=", "9");
    harness.get_by_label("Apply").click();
    harness.run();
    harness.run();

    assert!(
        orders_filters(&harness).is_empty(),
        "an invalid draft never runs"
    );
    let reason = harness.state().last_error().expect("the precise message");
    assert!(
        reason.contains("amount"),
        "the refusal names the column: {reason}"
    );
}

#[test]
fn sort_from_the_builder_dispatches() {
    let app = orders_app();
    let mut harness = harness(app);
    harness.run();
    open_builder(&mut harness);
    // The sort line's column picker, then the ascending direction.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Sort column")
        .click();
    harness.run();
    harness.get_by_label("name").click();
    harness.run();
    harness.get_by_label("Sort Ascending").click();
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();
    harness.run();

    let tab = harness
        .state()
        .snapshot()
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .expect("the Orders tab is open")
        .clone();
    assert_eq!(
        tab.sort_field.as_deref(),
        Some("name"),
        "the builder's sort line dispatches the existing sort command"
    );
    assert_eq!(
        tab.sort_order,
        Some(datacraft_core::qbe::SortOrder::Ascending),
    );
}
