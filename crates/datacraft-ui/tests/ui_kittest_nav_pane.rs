//! The navigation-pane affordance contracts (step 91.7, items M115 and
//! L55–L58): the sort menu names its pane-wide scope and marks the active
//! order, a row opens from its whole rect (icon included), the native
//! inventory renders in the pane's row language, the category header is
//! a plain label until real categories exist, and view creation lives in
//! the Views group menu instead of a detached pane-bottom button.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, NativeObject, NativeObjectKind, NativeView};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn app_with_objects() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    port.with_native_objects(vec![NativeObject {
        name: "Category Picker".into(),
        kind: NativeObjectKind::Form,
    }]);
    port.with_native_views(vec![NativeView {
        name: "adults".into(),
        sql: "CREATE VIEW adults AS SELECT id FROM Customers".into(),
        fields: vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn nav_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
        },
        app,
    );
    // Tall enough that the whole scroll content is inside the viewport —
    // egui culls off-viewport nodes from the accessibility tree.
    harness.set_size(egui::vec2(600.0, 1600.0));
    harness
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
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

/// Secondary click at `pos` (the collapsing header is painted text).
fn right_click(harness: &mut Harness<'static, App>, pos: egui::Pos2) {
    harness.hover_at(pos);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
}

/// Primary click at `pos` (for offsets inside a row rect).
fn left_click_at(harness: &mut Harness<'static, App>, pos: egui::Pos2) {
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
}

/// The sort menu names its pane-wide scope and marks the active order —
/// plain buttons read per-group while acting globally (M115).
#[test]
fn the_nav_sort_menu_names_its_scope_and_marks_the_active_order() {
    let mut harness = nav_harness(app_with_objects());
    harness.run();
    let target = {
        let header = harness.get_by_label("Tables");
        header.rect().center()
    };
    right_click(&mut harness, target);
    harness.run();

    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Sort all sections by Name") && dump.contains("Sort all sections by Type"),
        "the entries name the pane-wide scope:\n{dump}"
    );
    // The active order carries the selected state; the other does not.
    let name_entry = harness
        .get_all_by_label("Sort all sections by Name")
        .find(|node| format!("{:?}", node.accesskit_node().role()) == "Button")
        .expect("the name entry");
    assert!(
        format!("{:?}", name_entry.accesskit_node().toggled()).contains("True"),
        "the active order is marked"
    );
    let type_entry = harness
        .get_all_by_label("Sort all sections by Type")
        .find(|node| format!("{:?}", node.accesskit_node().role()) == "Button")
        .expect("the type entry");
    assert!(
        !format!("{:?}", type_entry.accesskit_node().toggled()).contains("True"),
        "the inactive order carries no mark"
    );

    // Switching the order moves the mark.
    harness.get_by_label("Sort all sections by Type").click();
    harness.run();
    assert_eq!(
        harness.state().nav_sort,
        datacraft_core::nav::NavSortOrder::Type
    );
    let target2 = {
        let header = harness.get_by_label("Tables");
        header.rect().center()
    };
    right_click(&mut harness, target2);
    harness.run();
    let type_entry = harness
        .get_all_by_label("Sort all sections by Type")
        .find(|node| format!("{:?}", node.accesskit_node().role()) == "Button")
        .expect("the type entry");
    assert!(
        format!("{:?}", type_entry.accesskit_node().toggled()).contains("True"),
        "the mark follows the order"
    );
}

/// The whole row is the click target — the icon included (L55): a click
/// over the icon, left of the label node, opens the object.
#[test]
fn the_whole_nav_row_opens_the_object() {
    let mut harness = nav_harness(app_with_objects());
    harness.run();
    let icon_pos = {
        let row = harness.get_by_label("Customers");
        egui::pos2(row.rect().left() + 4.0, row.rect().center().y)
    };
    left_click_at(&mut harness, icon_pos);
    harness.run();
    let tabs = &harness.state().snapshot().tabs;
    assert_eq!(tabs.len(), 1, "the icon-area click opens the object");
    assert_eq!(tabs[0].object_name, "Customers");
}

/// The native inventory renders in the pane's row language (L56): no
/// disabled-button chrome, the names appear as plain rows.
#[test]
fn native_entries_render_in_the_pane_row_language() {
    let mut harness = nav_harness(app_with_objects());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Category Picker"),
        "the native entry is listed:\n{dump}"
    );
    assert!(
        !dump
            .lines()
            .any(|line| line.starts_with("Button|") && line.contains("Category Picker")),
        "the native entry carries no button chrome:\n{dump}"
    );
    assert!(
        !dump
            .lines()
            .any(|line| line.contains("Category Picker") && line.ends_with(" disabled")),
        "the native entry is not a disabled control:\n{dump}"
    );
}

/// The category header is a plain labelled header (L57): no menu opens
/// that cannot do anything.
#[test]
fn the_category_header_is_a_plain_label() {
    let mut harness = nav_harness(app_with_objects());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("All Access Objects"),
        "the header is present:\n{dump}"
    );
    assert!(
        !dump
            .lines()
            .any(|line| line.contains("All Access Objects") && line.starts_with("Button")),
        "the header carries no menu chrome:\n{dump}"
    );
}

/// View creation lives in the Views group menu (L58): no detached
/// pane-bottom button, and the menu entry opens the editor.
#[test]
fn view_creation_lives_in_the_views_group_menu() {
    let mut harness = nav_harness(app_with_objects());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("New View"),
        "no detached button at the pane bottom:\n{dump}"
    );
    assert!(
        harness.state().view_draft().is_none(),
        "nothing opened on its own"
    );

    let target = {
        let header = harness.get_by_label("Views");
        header.rect().center()
    };
    right_click(&mut harness, target);
    harness.run();
    harness.get_by_label("New View").click();
    harness.run();
    assert!(
        harness.state().view_draft().is_some(),
        "the group menu opens the view editor"
    );
}
