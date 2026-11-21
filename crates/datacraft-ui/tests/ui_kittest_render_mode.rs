//! Low-power rendering mode suite (step 92.6, item L51).
//!
//! The mode is a policy switch: in low power the shell paints the
//! frame-statistics overlay; in the normal mode it stays quiet. The
//! policy model itself (zero animations, the ~30 fps interaction cap)
//! is pinned by the unit rows in `render_mode.rs`.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::render_mode::RenderMode;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn app() -> App {
    let port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 40 })],
    )]);
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
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

/// The low-power mode paints the frame-statistics overlay naming the
/// mode (step 92.6, item L51).
#[test]
fn low_power_paints_the_frame_statistics_overlay() {
    let mut app = app();
    app.render_mode = RenderMode::LowPower;
    let mut harness = shell_harness(app);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    assert!(
        harness
            .query_by_label_contains("Low-power rendering")
            .is_some(),
        "the overlay names the mode"
    );
}

/// The normal mode paints no overlay (step 92.6, item L51).
#[test]
fn normal_paints_no_overlay() {
    let mut harness = shell_harness(app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    assert!(
        harness
            .query_by_label_contains("Low-power rendering")
            .is_none(),
        "the overlay stays quiet in the normal mode"
    );
}
