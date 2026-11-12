//! File facts on the Info page (phase 73, step 73.3; item M91).
//!
//! The Info page names the driver facts and the health verdict; an
//! unhealthy file shows its precise reason instead of hiding it.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, SourceFact, SourceFacts};
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn facts(health: Option<&str>) -> SourceFacts {
    SourceFacts {
        facts: vec![
            SourceFact {
                name: "sqlite_version".into(),
                value: "3.53.4".into(),
            },
            SourceFact {
                name: "page_size".into(),
                value: "4096".into(),
            },
            SourceFact {
                name: "journal_mode".into(),
                value: "wal".into(),
            },
        ],
        health: health.map(str::to_owned),
    }
}

fn info_harness(health: Option<&str>) -> Harness<'static, App> {
    let mut port = MockPort::new();
    port.with_facts(facts(health));
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.sqlite".into(),
        password: None,
        source: None,
    });
    app.backstage_page = BackstagePage::Info;
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::busy::busy_frame(app, ui);
            datacraft_ui::backstage::backstage_ui(app, ui)
        },
        app,
    )
}

#[test]
fn the_info_page_names_the_driver_facts() {
    let harness = info_harness(None);
    harness.get_by_label("SQLite version");
    harness.get_by_label("3.53.4");
    harness.get_by_label("Page size");
    harness.get_by_label("4096");
    harness.get_by_label("Journal mode");
    harness.get_by_label("wal");
    harness.get_by_label("Integrity check");
    harness.get_by_label("ok");
}

#[test]
fn an_unhealthy_file_shows_the_precise_reason() {
    let reason = "quick_check: *** in database main *** page 3: btree init";
    let harness = info_harness(Some(reason));
    harness.get_by_label("Integrity check");
    harness.get_by_label(reason);
}
