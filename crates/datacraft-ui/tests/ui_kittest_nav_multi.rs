//! Nav-pane multi-select suite (step 92.4, item L48).
//!
//! Ctrl+click toggles rows into a selection without opening, Shift+click
//! ranges from the anchor within one section, a plain click opens and
//! resets, and the context menu acts on the selection ("Open N
//! selected"). Clicks ride egui pointer events so the modifiers travel
//! with them.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn multi_app() -> App {
    let port = MockPort::with_tables(vec![
        TableDef::new(
            "Customers",
            vec![FieldDef::new("Name", FieldKind::Text { max_len: 40 })],
        ),
        TableDef::new(
            "Orders",
            vec![FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger))],
        ),
        TableDef::new(
            "ServiceOrders",
            vec![FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger))],
        ),
    ]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn shell_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            // The language static is process-wide; pin English so the
            // label queries stay deterministic.
            datacraft_ui::i18n::set_lang(datacraft_ui::i18n::Lang::English);
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

/// Presses and releases the primary button at the row with `modifiers`
/// (a kittest click carries no modifiers — multi-select needs them).
fn click_row(harness: &mut Harness<'static, App>, label: &str, modifiers: egui::Modifiers) {
    let pos = harness.get_by_label(label).rect().center();
    harness.hover_at(pos);
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.run();
    harness.run();
}

/// Right-clicks the row (the context menu opens on the next frames).
fn right_click_row(harness: &mut Harness<'static, App>, label: &str) {
    let pos = harness.get_by_label(label).rect().center();
    harness.hover_at(pos);
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    harness.run();
}

/// Ctrl+click selects instead of opening, and the context menu acts on
/// the selection: "Open 2 selected" opens both rows as tabs
/// (step 92.4, item L48).
#[test]
fn ctrl_click_selects_and_the_menu_opens_the_selection() {
    let mut harness = shell_harness(multi_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    click_row(&mut harness, "Customers", egui::Modifiers::COMMAND);
    click_row(&mut harness, "ServiceOrders", egui::Modifiers::COMMAND);
    assert_eq!(
        harness.state().snapshot().tabs.len(),
        0,
        "a ctrl-click selects instead of opening"
    );

    right_click_row(&mut harness, "Customers");
    harness.get_by_label("Open 2 selected").click();
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().snapshot().tabs.len(),
        2,
        "the selection opens as tabs"
    );
}

/// Shift+click ranges from the anchor within the section's display order
/// (step 92.4, item L48).
#[test]
fn shift_click_ranges_a_section_from_the_anchor() {
    let mut harness = shell_harness(multi_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    // The plain click opens and anchors; the shift-click extends the
    // selection to the section range.
    click_row(&mut harness, "Orders", egui::Modifiers::NONE);
    assert_eq!(harness.state().snapshot().tabs.len(), 1);
    click_row(&mut harness, "ServiceOrders", egui::Modifiers::SHIFT);

    right_click_row(&mut harness, "Orders");
    harness.get_by_label("Open 2 selected").click();
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().snapshot().tabs.len(),
        3,
        "the anchor tab plus the ranged pair"
    );
}

/// A plain click opens the row and resets the selection — the menu acts
/// on the single object again (step 92.4, item L48).
#[test]
fn a_plain_click_opens_and_resets_the_selection() {
    let mut harness = shell_harness(multi_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    click_row(&mut harness, "Customers", egui::Modifiers::COMMAND);
    click_row(&mut harness, "ServiceOrders", egui::Modifiers::COMMAND);
    click_row(&mut harness, "Customers", egui::Modifiers::NONE);
    assert_eq!(
        harness.state().snapshot().tabs.len(),
        1,
        "the plain click opens"
    );

    right_click_row(&mut harness, "ServiceOrders");
    assert!(
        harness.query_by_label("Open 2 selected").is_none(),
        "the selection reset — no collective action remains"
    );
}
