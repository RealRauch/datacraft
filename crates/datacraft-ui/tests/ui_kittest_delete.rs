//! Datasheet row-deletion suite (step 40.3, item M44).
//!
//! The row selector's context menu and the Del key on the selected record
//! delete through exactly one `DeleteRow` dispatch over the real write
//! path — gated behind the destructive-action confirmation of the dialog
//! seam (step 38.3, item M43) and the `write_rows` capability. Assertions
//! stay on the cached rows and the semantic tree.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with the Customers tab open over four records; names carry their
/// index so cells stay uniquely addressable by label.
fn delete_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    let mut app = App::new(Box::new(port));
    app.set_dialogs(Box::new(ScriptedDialogs::new()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

fn delete_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

/// Secondary click on the row-2 record selector (addressable by label at
/// scroll offset zero).
fn right_click_selector(harness: &mut Harness<'static, App>) {
    let rect = harness.get_by_label("     2").rect();
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

fn cached_row_count(harness: &Harness<'static, App>, tab: TabId) -> usize {
    harness.state().rows_for(tab).map(Vec::len).unwrap_or(0)
}

/// The row selector's context menu deletes through the confirmation: with
/// a scripted "yes", exactly one `DeleteRow` dispatch removes the record.
#[test]
fn delete_record_menu_removes_the_row_after_confirmation() {
    let (mut app, tab) = delete_app(4);
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    let mut harness = delete_harness(app);
    harness.run();
    assert_eq!(cached_row_count(&harness, tab), 4);

    right_click_selector(&mut harness);
    harness.get_by_label("Delete Record").click();
    harness.run();
    harness.run();

    assert_eq!(
        cached_row_count(&harness, tab),
        3,
        "the confirmed deletion removes exactly one record"
    );
    let dump_ids: String = harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        !dump_ids.contains("alpha1"),
        "the deleted record's cell leaves the datasheet: {dump_ids}"
    );
}

/// With a scripted "no" the row stays; no dispatch happens.
#[test]
fn cancelling_the_confirmation_keeps_the_row() {
    let (mut app, tab) = delete_app(4);
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(false)));
    let mut harness = delete_harness(app);
    harness.run();

    right_click_selector(&mut harness);
    harness.get_by_label("Delete Record").click();
    harness.run();
    harness.run();

    assert_eq!(
        cached_row_count(&harness, tab),
        4,
        "a cancelled confirmation never dispatches"
    );
}

/// Without the write capability the menu entry renders disabled with the
/// reason, and even a queued "yes" never dispatches.
#[test]
fn delete_record_is_disabled_for_read_only_sources() {
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(caps);
    port.with_row_data(
        "Customers",
        (0..4)
            .map(|index| {
                vec![
                    datacraft_core::schema::FieldValue::Integer(index as i64),
                    datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
                ]
            })
            .collect(),
    );
    let mut restricted = App::new(Box::new(port));
    restricted.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    restricted.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    restricted.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = restricted.snapshot().tabs[0].id;
    let mut harness = delete_harness(restricted);
    harness.run();

    right_click_selector(&mut harness);
    harness.run();
    let disabled: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let n = node.accesskit_node();
            let label = n.label().unwrap_or_default();
            (label == "Delete Record" && n.is_disabled()).then(|| label.to_owned())
        })
        .collect();
    assert!(
        !disabled.is_empty(),
        "the delete entry renders disabled for read-only sources"
    );

    // The disabled entry cannot dispatch; the queued "yes" is unconsumed.
    harness.get_by_label("Delete Record").click();
    harness.run();
    harness.run();
    assert_eq!(
        cached_row_count(&harness, tab),
        4,
        "a disabled delete affordance never dispatches"
    );
}

/// The Del key on the selected record deletes through the same
/// confirmation (step 40.3, item M44).
#[test]
fn delete_key_removes_the_selected_record_after_confirmation() {
    let (mut app, tab) = delete_app(4);
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    let mut harness = delete_harness(app);
    harness.run();

    // Select the second record (arms the grid), then press Del.
    harness.get_by_label("     2").click();
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();
    harness.run();

    assert_eq!(
        cached_row_count(&harness, tab),
        3,
        "the Del key deletes the selected record after confirmation"
    );
    let dump_ids: String = harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        !dump_ids.contains("alpha1"),
        "the selected record leaves the datasheet: {dump_ids}"
    );
}
