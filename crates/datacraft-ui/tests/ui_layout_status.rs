//! Status-bar layout and interaction suite (split from `ui_layout.rs`,
//! L30; step 69.5, item M83).
//!
//! The shell's status bar: the tab-close label, the filter chip with its
//! per-filter menu and the Access lock-file notice. The shell frame and
//! rect helpers are local copies so the main layout suite stays below the
//! size gate.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn app_with_tables() -> App {
    let port = MockPort::with_tables(vec![
        TableDef::new(
            "ServiceOrders",
            vec![
                FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("Description", FieldKind::Text { max_len: 50 }),
            ],
        ),
        TableDef::new(
            "Customers",
            vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
        ),
    ]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
        source: None,
    });
    app
}

/// Renders the full shell at `size` and drains the rect log plus the
/// semantic tree.
fn shell_frame(app: App, size: egui::Vec2) -> (Vec<(String, egui::Rect)>, String) {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
            layout_probe::record("probe.window", ui.max_rect());
        },
        app,
    );
    harness.set_size(size);
    harness.run();
    harness.run();
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}{disabled}", node.role()));
    }
    (layout_probe::drain(), lines.join("\n"))
}

fn panel_rect(rects: &[(String, egui::Rect)], id: &str) -> egui::Rect {
    rects
        .iter()
        .find(|(name, _)| name == id)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("rect {id:?} recorded"))
}

fn tree_dump_shell(harness: &Harness<'_, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        // The record-navigation counter stores its text as the node's
        // value (step 48.1, M60), so the dump must carry both.
        lines.push(format!("{:?}|{label}|{value}", node.role()));
    }
    lines.join(
        "
",
    )
}

/// The tab strip's close affordance is a labelled button painted by the
/// icon dispatcher — the shipped font set has no "✕" glyph, so the text
/// button rendered tofu in the live app (step 44.2, H53 sweep).
#[test]
fn the_tab_close_affordance_is_labelled_close() {
    let mut app = app_with_tables();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "ServiceOrders",
    )));
    let (_rects, dump) = shell_frame(app, egui::vec2(1280.0, 800.0));
    assert!(
        dump.contains("Button|Close"),
        "the tab close affordance carries the localized Close label:\n{dump}"
    );
    assert!(
        !dump.contains("Button|✕"),
        "the close affordance must not paint the tofu-boxed ✕ character:\n{dump}"
    );
}

/// The status bar's filter chip is live when the active table carries
/// column filters (step 44.5, M54): it names the count, lists every
/// filter with a remove action and keeps the clear-all entry (69.5, M83).
#[test]
fn the_status_filter_chip_clears_the_active_filters() {
    let mut app = app_with_tables();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "ServiceOrders",
    )));
    app.dispatch(Command::SetTableFilter {
        table: "ServiceOrders".into(),
        field: "Id".into(),
        criteria: Some("> 1".into()),
    });
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    );
    harness.run();
    let chip = harness.get_by_label("Filter (1)");
    assert!(chip.rect().width() > 0.0, "the chip names the filter count");
    chip.click();
    harness.run();
    harness.get_by_label("Clear all filters").click();
    harness.run();
    let tab = &harness.state().snapshot().tabs[0];
    assert!(
        tab.filters.is_empty(),
        "the clear-all entry removed every filter: {:?}",
        tab.filters
    );
    let dump = tree_dump_shell(&harness);
    assert!(
        !dump.contains("Filter (1)"),
        "the chip is gone once no filter is active:
{dump}"
    );
}

/// The Access lock-file notice (step 53.2, M59): the session opened
/// read-only and the persistent notice stands beside the database
/// context. The full-shell semantic tree culls static text (buttons
/// survive, labels do not — the same blindness the M60/M61 probes worked
/// around), so presence pins through the layout_probe rect seam; the
/// notice text is pinned end-to-end by the Info page kittest (the same
/// tr call) and the i18n kernel tests.
#[test]
fn the_status_bar_renders_the_access_lock_file_notice() {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    let mut caps = datacraft_core::port::DriverCapabilities::full();
    caps.write_rows = false;
    caps.create_table = false;
    caps.drop_table = false;
    caps.alter_table = false;
    caps.atomic_batch = false;
    port.with_capabilities(caps);
    port.with_lock_file("orders.ldb");
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "orders.mdb".into(),
        password: None,
        source: None,
    });
    let (rects, _dump) = shell_frame(app, egui::vec2(1280.0, 800.0));
    let status = panel_rect(&rects, "shell.status_panel");
    let notice = panel_rect(&rects, "status.lock_notice");
    assert!(
        notice.top() >= status.top() && notice.bottom() <= status.bottom() + 1.0,
        "the notice renders inside the status bar: notice {notice:?} vs status {status:?}"
    );
    assert!(
        notice.width() > 40.0,
        "the notice carries visible text: {notice:?}"
    );
}
