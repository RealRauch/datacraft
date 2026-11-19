//! The status-line truncation contract (step 91.4, item L52): a long
//! error clips inline to the available width with an ellipsis — the full
//! text rides the hover tooltip, the click still opens the message
//! history. Pinned on the status bar and the backstage status line, at
//! both pinned window sizes. The label rect rides the layout probe under
//! the stable `status.notice` id (the H52/T10 seam).

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

/// A failing read with an identifiable tail: the truncated inline text
/// keeps the head and cuts exactly this tail off.
fn app_with_long_error() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "T",
        vec![FieldDef::new("v", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_read_failure(format!("{}-tail", "a".repeat(300)));
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    // The statement materializes rows through the port — the failing read
    // becomes the last error with the long detail.
    app.dispatch(Command::RunSql {
        sql: "SELECT * FROM T".into(),
    });
    assert!(app.last_error().is_some(), "the read failure surfaces");
    app
}

fn shell_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

/// The inline error clips with an ellipsis: the shown text keeps the
/// head, the tail is cut, the label rect stays inside the window width.
#[test]
fn a_long_error_truncates_on_the_status_bar_at_both_pinned_sizes() {
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        let mut harness = shell_harness(app_with_long_error());
        harness.set_size(size);
        harness.run();
        let rect = layout_probe::drain()
            .iter()
            .find(|(id, _)| id == "status.notice")
            .map(|(_, rect)| *rect)
            .expect("the status bar records its notice rect");
        assert!(
            rect.width() <= size.x as f32,
            "the notice label stays inside the window at {size:?}: {rect:?}"
        );
        let dump = tree_dump(&harness);
        assert!(
            dump.lines().any(|line| line.ends_with("…")),
            "the shown text ends with an ellipsis at {size:?}:\n{dump}"
        );
        assert!(
            !dump.contains("-tail"),
            "the tail is cut off inline at {size:?}:\n{dump}"
        );
    }
}

/// The backstage status line carries the same contract.
#[test]
fn a_long_error_truncates_on_the_backstage_status_line() {
    let mut app = app_with_long_error();
    app.backstage_page = BackstagePage::Info;
    app.active_ribbon_tab = "file";
    let mut harness = shell_harness(app);
    harness.set_size(egui::vec2(960.0, 600.0));
    harness.run();
    let rect = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "status.notice")
        .map(|(_, rect)| *rect)
        .expect("the backstage page records its status rect");
    assert!(
        rect.width() <= 960.0,
        "the backstage notice stays inside the page: {rect:?}"
    );
    let dump = tree_dump(&harness);
    assert!(
        dump.lines().any(|line| line.ends_with("…")),
        "the shown text ends with an ellipsis:\n{dump}"
    );
    assert!(!dump.contains("-tail"), "the tail is cut off inline:\n{dump}");
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order.
fn tree_dump(harness: &Harness<'_, App>) -> String {
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
