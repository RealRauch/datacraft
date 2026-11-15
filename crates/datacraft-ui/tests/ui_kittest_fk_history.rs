//! Foreign-key navigation history and referenced-by counts (step 87.2,
//! item M103).
//!
//! Back returns to the source table with its stored filters — reopening
//! a closed source tab; forward returns to the followed target; a
//! source with pending edits is only activated (its staged edits
//! survive untouched); the reverse menu entries carry capped counts
//! that name their truncation and clear with the tab.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort, Relationship, Support};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::{App, ReferencedCount};
use datacraft_ui::clipboard::ScriptedClipboard;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn order_relationship() -> Relationship {
    Relationship {
        table: "Orders".into(),
        column: "customer_id".into(),
        related_table: "Customers".into(),
        related_column: "id".into(),
    }
}

/// App with Customers and Orders (orders 10→1, 11→2), the `table` tab
/// open. `batch` forces the atomic-batch capability for staged edits.
fn shop_app(table: &str, batch: bool, order_rows: Vec<Vec<FieldValue>>) -> App {
    let mut port = MockPort::with_tables(vec![customers(), orders()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
        ],
    );
    port.with_row_data("Orders", order_rows);
    if !batch {
        let caps = DriverCapabilities::full();
        port.with_capabilities(caps);
    } else {
        let mut caps = DriverCapabilities::full();
        caps.atomic_batch = true;
        port.with_capabilities(caps);
    }
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(ScriptedClipboard::default()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        table,
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

/// Filters the Orders tab to id = 10 and follows the key cell to
/// Customers (the 87.1 flow). Returns the app state after the follow.
fn follow_orders_to_customers(harness: &mut Harness<'static, App>) {
    // The key cell of record 1 (selector click parks the cursor on
    // record 2 first, so the cell click only selects).
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(harness, "1");
    harness.get_by_label("Follow to Customers").click();
    harness.run();
    harness.run();
}

#[test]
fn back_returns_to_the_source_table_with_its_filters() {
    let mut app = shop_app(
        "Orders",
        false,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(2)],
        ],
    );
    // The source's own filter at navigation time.
    app.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "id".into(),
        criteria: Some("= 10".into()),
    });
    let mut harness = harness(app);
    harness.run();
    follow_orders_to_customers(&mut harness);

    harness.get_by_label("Back").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let orders = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .expect("back reopened the source tab");
    assert_eq!(
        snapshot.active_tab,
        Some(orders.id),
        "back activates the source tab"
    );
    assert_eq!(
        orders.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 10".to_owned())]),
        "back restores the source's filter"
    );
}

#[test]
fn back_reopens_a_closed_source_tab_with_its_filters() {
    let mut app = shop_app(
        "Orders",
        false,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(2)],
        ],
    );
    app.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "id".into(),
        criteria: Some("= 10".into()),
    });
    let mut harness = harness(app);
    harness.run();
    follow_orders_to_customers(&mut harness);

    // Close the source tab (the follow target stays).
    let snapshot = harness.state().snapshot();
    let customers_id = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .map(|tab| tab.id)
        .expect("the Customers tab is open");
    harness.state_mut().dispatch(Command::CloseOtherTabs {
        tab_id: customers_id,
    });
    harness.run();
    assert_eq!(
        harness.state().snapshot().tabs.len(),
        1,
        "only the follow target remains"
    );

    harness.get_by_label("Back").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let orders = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .expect("back reopened the closed source tab");
    assert_eq!(snapshot.active_tab, Some(orders.id));
    assert_eq!(
        orders.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 10".to_owned())]),
        "the stored filter is restored with the reopened tab"
    );
}

#[test]
fn forward_returns_to_the_followed_target() {
    let app = shop_app(
        "Orders",
        false,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(2)],
        ],
    );
    let mut harness = harness(app);
    harness.run();
    follow_orders_to_customers(&mut harness);
    harness.get_by_label("Back").click();
    harness.run();
    harness.run();

    harness.get_by_label("Forward").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let customers = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .expect("forward returned to the followed target");
    assert_eq!(snapshot.active_tab, Some(customers.id));
    assert_eq!(
        customers.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 1".to_owned())]),
        "forward restores the followed state's filter"
    );
}

#[test]
fn back_only_activates_a_source_with_pending_edits() {
    let mut app = shop_app(
        "Orders",
        true,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(2)],
        ],
    );
    app.dispatch(Command::SetTableFilter {
        table: "Orders".into(),
        field: "id".into(),
        criteria: Some("= 10".into()),
    });
    let mut harness = harness(app);
    harness.run();
    // Stage an edit through the 85 buffer: the clipboard pastes one
    // cell over the id of record 2 (atomic-batch driver).
    harness.get_by_label("     1").click();
    harness.run();
    harness.get_by_label("10").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::V);
    harness.run();

    follow_orders_to_customers(&mut harness);
    let snapshot = harness.state().snapshot();
    let orders_id = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .map(|tab| tab.id)
        .expect("the source is open");

    harness.get_by_label("Back").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let orders = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .expect("back activated the source");
    assert_eq!(snapshot.active_tab, Some(orders.id));
    assert!(
        harness.state().has_staged_edits(orders_id),
        "the staged edit survives the activation"
    );
    assert_eq!(
        orders.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 10".to_owned())]),
        "a pending-edits source keeps its live filter state"
    );
}

#[test]
fn referenced_by_counts_show_in_the_menu() {
    let app = shop_app(
        "Customers",
        false,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(1)],
        ],
    );
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(&mut harness, "1");
    harness.run();
    harness.run();

    assert!(
        harness
            .query_by_label_contains("Referenced by Orders (2)")
            .is_some(),
        "the menu names the capped count of referencing rows"
    );
    let count = harness.state().referenced_by_count(
        "Orders",
        &[("customer_id".to_owned(), "= 1".to_owned())],
    );
    assert_eq!(count, Some(ReferencedCount::Exact(2)));
}

#[test]
fn truncated_counts_name_the_cap() {
    let rows: Vec<Vec<FieldValue>> = (0..5000)
        .map(|index| {
            vec![
                FieldValue::Integer(index),
                FieldValue::Integer(1),
            ]
        })
        .collect();
    let app = shop_app("Customers", false, rows);
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(&mut harness, "1");
    harness.run();
    harness.run();

    assert!(
        harness
            .query_by_label_contains("Referenced by Orders (≥ 4096)")
            .is_some(),
        "the menu names the truncation at the cap"
    );
    let count = harness.state().referenced_by_count(
        "Orders",
        &[("customer_id".to_owned(), "= 1".to_owned())],
    );
    assert_eq!(count, Some(ReferencedCount::Truncated(4096)));
}

#[test]
fn counts_clear_with_the_tab() {
    let app = shop_app(
        "Customers",
        false,
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(1)],
        ],
    );
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     2").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(&mut harness, "1");
    harness.run();
    harness.run();
    assert!(
        harness
            .state()
            .referenced_by_count("Orders", &[("customer_id".to_owned(), "= 1".to_owned())])
            .is_some(),
        "the count is present before the close"
    );

    harness.state_mut().dispatch(Command::CloseAllTabs);
    harness.run();

    assert!(
        harness
            .state()
            .referenced_by_count("Orders", &[("customer_id".to_owned(), "= 1".to_owned())])
            .is_none(),
        "closing the tab drops the pending counts"
    );
}
