//! Foreign-key follow suite (step 87.1, item M103).
//!
//! A key cell's context menu follows to the referenced parent
//! (composite keys filter all target columns), F4 follows the first
//! target, the reverse "referenced by" opens the child filtered, an
//! empty key answers the precise message without opening a tab, and a
//! driver without relationship support offers no follow at all.
//! Assertions stay on the snapshot and the semantic tree — never
//! pixels.

use datacraft_core::command::Command;
use datacraft_core::port::{
    DriverCapabilities, MockPort, Relationship, RelationshipGroup, Support,
};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
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

fn parents() -> TableDef {
    TableDef::new(
        "parents",
        vec![
            FieldDef::new("px", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("py", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn children() -> TableDef {
    TableDef::new(
        "children",
        vec![
            FieldDef::new("a", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("b", FieldKind::Number(NumericKind::LongInteger)),
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

/// App over the customers/orders shop with the `table` tab open. The
/// harness renders the active tab — after a follow the target tab is
/// the one on screen.
fn shop_app(
    table: &str,
    relationships: Vec<Relationship>,
    groups: Option<Vec<RelationshipGroup>>,
    restricted: bool,
) -> App {
    let mut port = MockPort::with_tables(vec![customers(), orders()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
        ],
    );
    port.with_row_data(
        "Orders",
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Integer(2)],
        ],
    );
    if !relationships.is_empty() {
        port.with_relationships(relationships);
    }
    if let Some(groups) = groups {
        port.with_relationship_groups(groups);
    }
    let mut caps = DriverCapabilities::full();
    if restricted {
        caps.relationships = Support::No;
    }
    port.with_capabilities(caps);
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

fn composite_app() -> App {
    let mut port = MockPort::with_tables(vec![parents(), children()]);
    port.with_row_data(
        "parents",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Integer(2)],
            vec![FieldValue::Integer(3), FieldValue::Integer(4)],
        ],
    );
    port.with_row_data(
        "children",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Integer(2)],
            vec![FieldValue::Integer(3), FieldValue::Integer(4)],
        ],
    );
    let entries = vec![
        Relationship {
            table: "children".into(),
            column: "a".into(),
            related_table: "parents".into(),
            related_column: "px".into(),
        },
        Relationship {
            table: "children".into(),
            column: "b".into(),
            related_table: "parents".into(),
            related_column: "py".into(),
        },
    ];
    port.with_relationships(entries.clone());
    port.with_relationship_groups(vec![RelationshipGroup {
        name: Some("r1".into()),
        table: "children".into(),
        related_table: "parents".into(),
        pairs: vec![("a".into(), "px".into()), ("b".into(), "py".into())],
    }]);
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(ScriptedClipboard::default()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.sqlite".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "children",
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

/// Right-clicks the cell with the exact label (the rename-dialog
/// helper's pointer-event shape).
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

/// Moves the record cursor onto `row_label`'s record (the row
/// selector's padded line number), so a cell click of another row only
/// selects — it never arms the in-place editor.
fn click_selector(harness: &mut Harness<'static, App>, line: &str) {
    harness.get_by_label(line).click();
    harness.run();
}

/// The Orders tab's first record's key cell: the selector line "2"
/// parks the cursor on record 2, then the customer cell selects row 1.
fn select_key_cell(harness: &mut Harness<'static, App>) {
    click_selector(harness, "     2");
    harness.get_by_label("1").click();
    harness.run();
}

#[test]
fn context_menu_follows_the_declared_key_to_the_filtered_parent() {
    let app = shop_app("Orders", vec![order_relationship()], None, false);
    let mut harness = harness(app);
    harness.run();
    select_key_cell(&mut harness);
    right_click_cell(&mut harness, "1");
    harness.get_by_label("Follow to Customers").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let customers_id = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .map(|tab| tab.id);
    assert_eq!(
        snapshot.active_tab, customers_id,
        "the follow activates the target tab"
    );
    let customers = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .expect("the Customers tab opened");
    assert_eq!(
        customers.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 1".to_owned())]),
        "the target opens filtered to the key value"
    );
    assert!(
        harness.query_by_label_contains("Ada").is_some(),
        "the referenced record is visible"
    );
    assert!(
        harness.query_by_label_contains("Grace").is_none(),
        "the filter removed the non-matching record"
    );
}

#[test]
fn f4_follows_the_first_declared_target() {
    let app = shop_app("Orders", vec![order_relationship()], None, false);
    let mut harness = harness(app);
    harness.run();
    select_key_cell(&mut harness);
    harness.key_press(egui::Key::F4);
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let customers = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .expect("F4 opened the target");
    assert_eq!(
        customers.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 1".to_owned())]),
        "F4 filters the target to the key value"
    );
}

#[test]
fn referenced_by_menu_opens_the_child_filtered() {
    let app = shop_app("Customers", vec![order_relationship()], None, false);
    let mut harness = harness(app);
    harness.run();
    click_selector(&mut harness, "     2");
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(&mut harness, "1");
    // The reverse entry carries its count once the bounded read
    // answered (one frame later — the right click's second run).
    harness.get_by_label("Referenced by Orders (1)").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let orders = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Orders")
        .expect("the child tab opened");
    assert_eq!(
        orders.filters,
        std::collections::BTreeMap::from([("customer_id".to_owned(), "= 1".to_owned())]),
        "the referencing children filter to the parent's key value"
    );
    assert!(
        harness.query_by_label_contains("11").is_none(),
        "the non-referencing record is filtered out"
    );
}

#[test]
fn composite_key_follow_filters_all_target_columns() {
    let app = composite_app();
    let mut harness = harness(app);
    harness.run();
    click_selector(&mut harness, "     2");
    harness.get_by_label("1").click();
    harness.run();
    right_click_cell(&mut harness, "1");
    harness.get_by_label("Follow to parents").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let parents = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "parents")
        .expect("the composite target opened");
    assert_eq!(
        parents.filters,
        std::collections::BTreeMap::from([
            ("px".to_owned(), "= 1".to_owned()),
            ("py".to_owned(), "= 2".to_owned()),
        ]),
        "the composite follow filters every target column"
    );
}

#[test]
fn a_null_key_answers_the_precise_error_without_opening_a_tab() {
    let mut port = MockPort::with_tables(vec![customers(), orders()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
        ],
    );
    port.with_row_data(
        "Orders",
        vec![
            vec![FieldValue::Integer(10), FieldValue::Integer(1)],
            vec![FieldValue::Integer(11), FieldValue::Null],
        ],
    );
    port.with_relationships(vec![order_relationship()]);
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(ScriptedClipboard::default()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));

    let mut harness = harness(app);
    harness.run();
    // The empty key cell carries no accessible label (an empty string is
    // no label) — it is the cell right of the second record's id cell,
    // clicked at its position (the freeze suite's positional precedent).
    click_selector(&mut harness, "     1");
    let rect = harness.get_by_label("11").rect();
    let pos = egui::pos2(
        rect.right() + datacraft_core::grid::DEFAULT_COLUMN_WIDTH / 2.0,
        rect.center().y,
    );
    harness.hover_at(pos);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
    harness.key_press(egui::Key::F4);
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    assert_eq!(snapshot.tabs.len(), 1, "no tab opened for an empty key");
    let reason = harness.state().last_error().expect("the precise message");
    assert!(
        reason.contains("customer_id") && reason.contains("empty"),
        "the message names the empty key column: {reason}"
    );
}

#[test]
fn inferred_references_follow_without_declared_keys() {
    let app = shop_app("Orders", Vec::new(), None, false);
    let mut harness = harness(app);
    harness.run();
    select_key_cell(&mut harness);
    right_click_cell(&mut harness, "1");
    harness.get_by_label("Follow to Customers").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    let customers = snapshot
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .expect("the inferred target opened");
    assert_eq!(
        customers.filters,
        std::collections::BTreeMap::from([("id".to_owned(), "= 1".to_owned())]),
        "the inferred reference filters the target"
    );
}

#[test]
fn restricted_relationship_support_hides_the_follow() {
    let app = shop_app("Orders", vec![order_relationship()], None, true);
    let mut harness = harness(app);
    harness.run();
    select_key_cell(&mut harness);
    right_click_cell(&mut harness, "1");
    harness.run();

    assert!(
        harness.query_by_label_contains("Follow to").is_none(),
        "a restricted driver offers no follow affordance"
    );
    harness.key_press(egui::Key::F4);
    harness.run();
    let snapshot = harness.state().snapshot();
    assert_eq!(
        snapshot.tabs.len(),
        1,
        "F4 does nothing on a restricted driver"
    );
}
