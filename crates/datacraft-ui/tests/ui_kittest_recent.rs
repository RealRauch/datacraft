//! Recent-database backstage contract (step 28.2, item U15) — a focused
//! harness so the main kittest suite stays below the file-size gate.

use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn backstage_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    )
}

#[test]
fn backstage_recent_database_opens_in_one_click() {
    let path = std::env::temp_dir().join(format!("datacraft-recent-{}.mdb", std::process::id()));
    std::fs::write(&path, b"x").expect("seed the recent path");
    let mut settings = datacraft_ui::settings::Settings {
        language: datacraft_ui::settings::LanguageSetting::English,
        ..datacraft_ui::settings::Settings::default()
    };
    settings.record_recent(&path);
    let mut app = App::new(Box::new(MockPort::new()));
    app.apply_settings(settings);
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Home;
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label_contains("datacraft-recent-").click();
    harness.run();
    let database = harness
        .state()
        .snapshot()
        .database
        .as_ref()
        .expect("the recent entry opened the database");
    assert_eq!(database.path, path);
    let _ = std::fs::remove_file(&path);
}

/// Step 38.2 (item M41): the recent row carries a localized "Modified"
/// timestamp so the list reads like the Excel backstage.
#[test]
fn recent_rows_show_the_modified_date() {
    let path =
        std::env::temp_dir().join(format!("datacraft-recent-mod-{}.mdb", std::process::id()));
    std::fs::write(&path, b"x").expect("seed the recent path");
    let mut settings = datacraft_ui::settings::Settings {
        language: datacraft_ui::settings::LanguageSetting::English,
        ..datacraft_ui::settings::Settings::default()
    };
    settings.record_recent(&path);
    let mut app = App::new(Box::new(MockPort::new()));
    app.apply_settings(settings);
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Home;
    let mut harness = backstage_harness(app);
    harness.run();
    let modified = harness.get_by_label_contains("Modified");
    assert!(modified.rect().width() > 0.0, "the modified label renders");
    let _ = std::fs::remove_file(&path);
}
