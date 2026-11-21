//! Nav-pane search suite (step 92.3, item L48).
//!
//! Typing into the pane's search field narrows the visible objects across
//! categories, a query without matches shows the localized empty state,
//! and Clear restores the full list. The field is reached through its
//! probe rect (a placeholder-only `TextEdit` carries no label node), the
//! typed text rides egui's text event onto the focused widget.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn nav_app() -> App {
    let port = MockPort::with_tables(vec![
        TableDef::new(
            "Customers",
            vec![FieldDef::new("Name", FieldKind::Text { max_len: 40 })],
        ),
        TableDef::new(
            "ServiceOrders",
            vec![FieldDef::new(
                "Id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
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

/// Clicks into the search field (press + release at the probe rect) and
/// types `text`.
fn type_into_search(harness: &mut Harness<'static, App>, text: &str) {
    harness.run();
    let field = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "nav.search")
        .expect("the search field records its rect")
        .1;
    harness.hover_at(field.center());
    harness.drag_at(field.center());
    harness.drop_at(field.center());
    harness.run();
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.run();
}

/// Typing narrows the visible objects across categories — only the
/// matching name stays in the pane (step 92.3, item L48).
#[test]
fn typing_narrows_the_objects_across_sections() {
    let mut harness = shell_harness(nav_app());
    harness.set_size(egui::vec2(1280.0, 800.0));

    type_into_search(&mut harness, "cust");

    assert!(
        harness.query_by_label("Customers").is_some(),
        "the matching table stays visible"
    );
    assert!(
        harness.query_by_label("ServiceOrders").is_none(),
        "the non-matching table is filtered out"
    );
}

/// A query without matches shows the localized empty state instead of a
/// bare pane (step 92.3, item L48).
#[test]
fn a_query_without_matches_shows_the_empty_state() {
    let mut harness = shell_harness(nav_app());
    harness.set_size(egui::vec2(1280.0, 800.0));

    type_into_search(&mut harness, "zzz");

    assert!(
        harness.query_by_label("Customers").is_none(),
        "no object matches"
    );
    assert!(
        harness
            .query_by_label_contains("No objects match")
            .is_some(),
        "the empty state names the query"
    );
}

/// The Clear affordance restores the full list (step 92.3, item L48).
#[test]
fn clear_restores_the_full_list() {
    let mut harness = shell_harness(nav_app());
    harness.set_size(egui::vec2(1280.0, 800.0));

    type_into_search(&mut harness, "cust");
    assert!(harness.query_by_label("ServiceOrders").is_none());

    harness.get_by_label("Clear").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Customers").is_some()
            && harness.query_by_label("ServiceOrders").is_some(),
        "the clear affordance restores every object"
    );
}
