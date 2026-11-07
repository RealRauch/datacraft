//! Layout contracts over the rect seam (step 44.1, items H52 + T10).
//!
//! The semantic tree is blind to pixel geometry — AccessKit nodes exist
//! even when a widget paints off-window or under another panel (H52 hid
//! the whole Queries group this way). These tests assert the recorded
//! paint rects: a ribbon group never exceeds its content, every group
//! stays inside the window, and panels never overlap the status bar.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
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
    });
    app
}

fn layout_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            datacraft_ui::ribbon::ribbon_ui(app, ui);
            layout_probe::record("probe.window", ui.max_rect());
        },
        app,
    )
}

fn ribbon_rects(mut app: App, ribbon_tab: &'static str) -> Vec<(String, egui::Rect)> {
    app.active_ribbon_tab = ribbon_tab;
    let mut harness = layout_harness(app);
    harness.run();
    layout_probe::drain()
}

/// A group's width is bounded by its content: one action never exceeds
/// the single-button floor plus margin, two stay near two buttons
/// (H52: the first group stretched across the whole row and pushed the
/// other groups off-window).
fn assert_groups_are_content_sized(rects: &[(String, egui::Rect)]) {
    for (id, rect) in rects {
        if !id.starts_with("ribbon.") {
            continue;
        }
        let width = rect.width();
        let actions = id.matches('.').count() - 1; // ribbon.<tab>.<group>
        let bound = 160.0 + 90.0 * actions as f32;
        assert!(
            width <= bound,
            "group {id} is {width:.0}px wide (bound {bound}) — it stretches across the row and hides the groups beside it"
        );
    }
}

#[test]
fn database_tools_groups_are_content_sized() {
    let rects = ribbon_rects(app_with_tables(), "database_tools");
    assert_groups_are_content_sized(&rects);
}

#[test]
fn every_ribbon_group_stays_inside_the_window() {
    // Both acceptance sizes (H52/T10): the shipped default and the
    // minimum plausible window.
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        for tab in ["database_tools", "create", "home"] {
            let mut app = app_with_tables();
            app.active_ribbon_tab = tab;
            let mut harness = layout_harness(app);
            harness.set_size(size);
            harness.run();
            let rects = layout_probe::drain();
            let window = rects
                .iter()
                .find(|(id, _)| id == "probe.window")
                .map(|(_, rect)| *rect)
                .expect("window rect recorded");
            for (id, rect) in &rects {
                if !id.starts_with("ribbon.") {
                    continue;
                }
                assert!(
                    rect.right() <= window.right() + 1.0,
                    "group {id} on {tab} paints at {rect:?}, beyond the window's right edge {window:?} at {size:?} — its actions are invisible in the live app (H52)"
                );
            }
        }
    }
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

#[test]
fn panels_never_overlap_the_status_bar() {
    // Both acceptance sizes (H53): the shipped default and the minimum
    // plausible window — the user saw datasheet rows behind the footer.
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        let mut app = app_with_tables();
        app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            datacraft_core::ObjectKind::Table,
            "ServiceOrders",
        )));
        let (rects, _dump) = shell_frame(app, size);
        let status = panel_rect(&rects, "shell.status_panel");
        let workspace = panel_rect(&rects, "shell.workspace_panel");
        let ribbon = panel_rect(&rects, "shell.ribbon_panel");
        let navpane = panel_rect(&rects, "shell.navpane_panel");
        assert!(
            workspace.bottom() <= status.top() + 1.0,
            "the workspace content bottom ({:?}) crosses the status bar top ({status:?}) at {size:?} — rows disappear behind the footer (H53)",
            workspace.bottom()
        );
        assert!(
            workspace.top() >= ribbon.bottom() - 1.0,
            "the workspace content top ({:?}) crosses the ribbon bottom ({ribbon:?}) at {size:?}",
            workspace.top()
        );
        assert!(
            workspace.left() >= navpane.right() - 1.0,
            "the workspace content left ({:?}) crosses the nav pane right ({navpane:?}) at {size:?}",
            workspace.left()
        );
    }
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
/// column filters (step 44.5, M54): it names the count and one click
/// clears every filter over the bus.
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
    let tab = &harness.state().snapshot().tabs[0];
    assert!(
        tab.filters.is_empty(),
        "the chip click cleared every filter: {:?}",
        tab.filters
    );
    let dump = tree_dump_shell(&harness);
    assert!(
        !dump.contains("Filter (1)"),
        "the chip is gone once no filter is active:
{dump}"
    );
}

/// An app with a 60-row ServiceOrders table open — enough rows that the
/// virtualized grid overflows the viewport and pushes the trailing
/// panels against the workspace bottom.
fn filled_datasheet_app() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "ServiceOrders",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Description", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    port.with_row_data(
        "ServiceOrders",
        (1..=60)
            .map(|id| {
                vec![
                    FieldValue::Integer(id),
                    FieldValue::Text(format!("service order {id}")),
                ]
            })
            .collect(),
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "ServiceOrders",
    )));
    app
}

/// The datasheet's trailing panels — the row-count label, the new-record
/// edit row and the Add-record button — stay above the status bar (step
/// 48.2, M61): the grid must reserve their space budget instead of
/// consuming the full workspace height and pushing them behind the
/// footer. With the row behind the status bar the add path is
/// unreachable — the user could not add a record at all (report
/// 2026-11-07).
#[test]
fn datasheet_trailing_elements_stay_above_the_status_bar() {
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        let (rects, _dump) = shell_frame(filled_datasheet_app(), size);
        let status = panel_rect(&rects, "shell.status_panel");
        for id in [
            "datasheet.row_count",
            "datasheet.new_row_cells",
            "datasheet.new_row_add",
        ] {
            let rect = panel_rect(&rects, id);
            assert!(
                rect.bottom() <= status.top() + 1.0,
                "{id} paints at {rect:?}, below the status bar top ({status:?}) at {size:?} — the new-record row is unreachable (M61)"
            );
        }
    }
}

/// The status bar's record navigation reads left-to-right (step 45.2,
/// user report): the decrement (previous) button sits left of the
/// increment (next) button — the outer right-to-left status layout
/// mirrored the pair.
#[test]
fn record_navigation_places_previous_left_of_next() {
    // The navigation box renders only over a non-empty record domain —
    // the fixture needs live rows (the table-only mock has none).
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("item", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    port.with_row_data(
        "Orders",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("bolt".to_owned())],
            vec![FieldValue::Integer(2), FieldValue::Text("nut".to_owned())],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
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
    let previous = harness.get_by_label("◀").rect();
    let next = harness.get_by_label("▶").rect();
    assert!(
        previous.left() < next.left(),
        "previous ({previous:?}) must sit left of next ({next:?})"
    );
}

/// The record-navigation counter reads the cursor the arrows write (step
/// 48.1, item M60, user report 2026-11-06): clicking next on a 2-row
/// table moves the counter to "2 of 2" and the record cursor to 1. The
/// counter used to read the form cursor, which never moves for tables,
/// so every click recomputed the same target from cursor 0 and the
/// arrows appeared dead.
#[test]
fn record_navigation_clicks_move_the_table_counter() {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("item", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    port.with_row_data(
        "Orders",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("bolt".to_owned())],
            vec![FieldValue::Integer(2), FieldValue::Text("nut".to_owned())],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
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
    harness.get_by_label("▶").click();
    harness.run();
    let dump = tree_dump_shell(&harness);
    assert!(
        dump.contains("2 of 2"),
        "one next click must advance the counter past the stale form cursor:\n{dump}"
    );
    let tab = harness.state().snapshot().tabs[0].clone();
    assert_eq!(
        harness.state().record_cursor(tab.id),
        1,
        "the arrow wrote the record cursor"
    );
}
