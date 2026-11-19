//! Version display contract (step 91.1, item M109) — a focused harness:
//! the product version is visible on the backstage Info page, with and
//! without an open database, sourced from the shared
//! `datacraft_core::VERSION`.

use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn info_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::busy::busy_frame(app, ui);
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    )
}

/// The Info page carries the localized version label and the version value
/// also before any database is open — a bug report needs the anchor there.
#[test]
fn info_page_shows_the_version_label_and_value() {
    let settings = datacraft_ui::settings::Settings {
        language: datacraft_ui::settings::LanguageSetting::English,
        ..datacraft_ui::settings::Settings::default()
    };
    let mut app = App::new(Box::new(MockPort::new()));
    app.apply_settings(settings);
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Info;
    let mut harness = info_harness(app);
    harness.run();
    let label = harness.get_by_label_contains("Version");
    assert!(label.rect().width() > 0.0, "the version label renders");
    let value = harness.get_by_label_contains(datacraft_core::VERSION);
    assert!(value.rect().width() > 0.0, "the version value renders");
}
