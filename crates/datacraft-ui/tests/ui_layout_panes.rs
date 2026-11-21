//! Docked-pane layout contracts (step 92.2, item M108) — split from
//! `ui_layout.rs` below the file-size gate: the workspace never overlaps
//! a docked pane, at both pinned window sizes and both dock edges.

use datacraft_core::command::Command;
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::panes::DockMode;
use eframe::egui;

#[path = "layout_support.rs"]
mod layout_support;

use layout_support::{app_with_tables, panel_rect, shell_frame};

/// The docked SQL pane reserves its edge: the workspace never overlaps it
/// (step 92.2, item M108) — at both pinned window sizes.
#[test]
fn the_docked_sql_pane_never_overlaps_the_workspace() {
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        let mut app = app_with_tables();
        app.dispatch(Command::OpenObject(ObjectId::new(
            ObjectKind::Table,
            "ServiceOrders",
        )));
        app.dispatch(Command::ToggleSqlWindow);
        app.set_pane_mode("sql", DockMode::Right);
        let (rects, _dump) = shell_frame(app, size);
        let window = panel_rect(&rects, "probe.window");
        let pane = panel_rect(&rects, "panes.sql_panel");
        let workspace = panel_rect(&rects, "shell.workspace_panel");
        assert!(
            workspace.right() <= pane.left() + 1.0,
            "the workspace content right ({workspace:?}) crosses the docked pane ({pane:?}) at {size:?} — the pane hides workspace content (M108)"
        );
        assert!(
            pane.right() <= window.right() + 1.0,
            "the docked pane leaves the window at {size:?}: {pane:?} in {window:?}"
        );
    }
}

/// The bottom-docked pane stacks above the status bar and below the
/// workspace (step 92.2, item M108).
#[test]
fn the_bottom_docked_sql_pane_stays_between_workspace_and_status() {
    for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0)] {
        let mut app = app_with_tables();
        app.dispatch(Command::OpenObject(ObjectId::new(
            ObjectKind::Table,
            "ServiceOrders",
        )));
        app.dispatch(Command::ToggleSqlWindow);
        app.set_pane_mode("sql", DockMode::Bottom);
        let (rects, _dump) = shell_frame(app, size);
        let pane = panel_rect(&rects, "panes.sql_panel");
        let workspace = panel_rect(&rects, "shell.workspace_panel");
        let status = panel_rect(&rects, "shell.status_panel");
        assert!(
            workspace.bottom() <= pane.top() + 1.0,
            "the workspace content bottom ({workspace:?}) crosses the bottom-docked pane ({pane:?}) at {size:?} (M108)"
        );
        assert!(
            pane.bottom() <= status.top() + 1.0,
            "the bottom-docked pane ({pane:?}) crosses the status bar ({status:?}) at {size:?} (M108)"
        );
    }
}
