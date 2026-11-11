//! Tool-window placement suite (step 69.7, item M85).
//!
//! egui opens windows top-left by default; the review asked for centered
//! initial positions. The pin is the title node's rect at a fixed size.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn dialog_harness(
    app: App,
    mut draw: impl FnMut(&mut egui::Ui, &mut App) + 'static,
) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            draw(ui, app);
        },
        app,
    )
}

/// Neither window title may sit in the top-left corner of an 800x600 shell.
fn assert_centered(harness: &Harness<'static, App>, title: &str) {
    let rect = harness.get_by_label(title).rect();
    assert!(
        rect.min.x > 50.0,
        "{title} opens centered horizontally: {rect:?}"
    );
    assert!(
        rect.min.y > 50.0,
        "{title} opens centered vertically: {rect:?}"
    );
}

/// The messages dialog opens centered.
#[test]
fn the_messages_dialog_opens_centered() {
    let mut app = App::new(Box::new(MockPort::new()));
    app.open_notices();
    let mut harness = dialog_harness(app, |ui, app| {
        datacraft_ui::notices_dialog::notices_dialog(app, ui);
    });
    harness.set_size(egui::vec2(800.0, 600.0));
    harness.run();
    assert_centered(&harness, "Messages");
}

/// The relationship mapper opens centered.
#[test]
fn the_relationship_mapper_opens_centered() {
    let mut port = MockPort::with_tables(vec![datacraft_core::schema::TableDef::new(
        "Customers",
        vec![datacraft_core::schema::FieldDef::new(
            "Id",
            datacraft_core::schema::FieldKind::Number(
                datacraft_core::schema::NumericKind::LongInteger,
            ),
        )],
    )]);
    port.with_relationships(vec![datacraft_core::port::Relationship {
        table: "Customers".into(),
        column: "Id".into(),
        related_table: "Customers".into(),
        related_column: "Id".into(),
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleRelationshipMapper);
    let mut harness = dialog_harness(app, |ui, app| {
        datacraft_ui::relationships::mapper_window(app, ui);
    });
    harness.set_size(egui::vec2(800.0, 600.0));
    harness.run();
    assert_centered(&harness, "Relationships");
}
