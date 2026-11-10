//! Backstage source-entry contract (step 65.3, item L8): the Open page
//! accepts a path or a `file://` URI, a URI session reports its
//! `SourceSpec` in the snapshot, and the Info page names the source
//! beside the path.

use datacraft_core::command::{Command, SourceSpec};
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn backstage_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::backstage::backstage_ui(app, ui)
        },
        app,
    )
}

fn open_page_harness() -> Harness<'static, App> {
    let mut app = App::new(Box::new(MockPort::new()));
    app.backstage_page = BackstagePage::Open;
    backstage_harness(app)
}

fn fixture_uri() -> &'static str {
    if cfg!(windows) {
        "file:///C:/dir/shop.mock"
    } else {
        "file:///dir/shop.mock"
    }
}

#[test]
fn the_open_page_accepts_a_file_uri_and_the_session_reports_the_source() {
    let mut harness = open_page_harness();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Database path")
        .set_text_value_accesskit(fixture_uri());
    harness.run();
    harness.get_by_label("Open Database").click();
    harness.run();
    let db = harness
        .state()
        .snapshot()
        .database
        .clone()
        .expect("the uri open must succeed");
    assert_eq!(
        db.source,
        Some(SourceSpec::Uri(fixture_uri().to_owned())),
        "the session must report the source spec it was opened with"
    );
}

#[test]
fn the_info_page_names_the_source_of_a_uri_session() {
    let mut app = App::new(Box::new(MockPort::new()));
    let events = app.dispatch(Command::OpenDatabase {
        path: std::path::PathBuf::new(),
        password: None,
        source: Some(SourceSpec::Uri(fixture_uri().to_owned())),
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    app.backstage_page = BackstagePage::Info;
    let harness = backstage_harness(app);
    harness.get_by_label("Source");
    harness.get_by_label(fixture_uri());
}
