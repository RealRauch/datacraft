//! SQL window (step 13.3, item H37; dock pilot step 92.2, item M108).
//!
//! Presence comes from the snapshot, the editor text is presentation
//! state, and running dispatches `RunSql` through the command bus — the
//! same command agents send. The result grid is the shared datasheet
//! grid. Since the dock host (M108) the pane renders inside the host's
//! panel when docked right/bottom and keeps the floating window while it
//! floats.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;

use crate::app::App;
use crate::datasheet::Grid;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::panes::{DockMode, dock_menu};
use crate::theme::tokens;

/// Height bound of the embedded result area (step 91.2, item M110): ten
/// body rows at the shipped rung — small results keep the window at its
/// size, larger ones scroll inside the bound.
const RESULT_MAX_ROWS: f32 = 10.0;

/// Renders the floating SQL window (when open and not docked).
pub fn sql_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().sql_window_open {
        return;
    }
    if app.pane_geometry("sql").mode != DockMode::Float {
        return; // the dock host's panel renders the content (M108)
    }
    let mut close = false;

    let response = egui::Window::new(t("SQL"))
        .id(egui::Id::unique("datacraft.sql"))
        .default_width(720.0)
        .resizable(true)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(720.0, 420.0),
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("Statement"));
                dock_menu(&mut *app, ui, "sql");
                if ui.small_button(t("Close")).clicked() {
                    close = true;
                }
            });
            sql_body(app, ui);
        });
    // Layout probe (the H52/T10 seam): the window's outer rect — the
    // height-budget contract (step 91.2, item M110) asserts on it.
    if let Some(response) = &response {
        crate::layout_probe::record("sql.window", response.response.rect);
    }

    if close {
        app.dispatch(Command::ToggleSqlWindow);
    }
}

/// The docked pane content (the host header carries title, dock menu and
/// close; step 92.2, item M108).
pub fn sql_pane_content(app: &mut App, ui: &mut egui::Ui) {
    sql_body(app, ui);
}

/// The shared body: editor, run row, fences, error and the result grid.
fn sql_body(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    let metrics = GridMetrics::default();
    let mut run = false;
    let mut text = app.sql_text().to_owned();
    let error = app.sql_error().map(str::to_owned);
    let result = app.sql_result().cloned();
    let columns = result
        .as_ref()
        .map(|result| result.columns.len())
        .unwrap_or(0);
    let mut widths = app.sql_column_widths(columns);
    let sorts = vec![SortOrder::None; columns];

    ui.add(
        egui::TextEdit::multiline(&mut text)
            .font(egui::TextStyle::Monospace)
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .hint_text("SELECT … FROM …"),
    );
    ui.horizontal(|ui| {
        if ui.button(t("Run")).clicked() {
            run = true;
        }
        ui.weak(t("one SELECT/WITH statement · read-only"));
    });
    ui.weak(t(
        "SQLite semantics: Currency materializes unscaled, LIKE uses % wildcards, text literals never match numeric columns, integer overflow promotes to REAL",
    ));
    if let Some(reason) = &error {
        ui.colored_label(tokens.danger, reason);
    }
    if let Some(result) = &result {
        ui.separator();
        let body: Vec<Vec<String>> = result
            .rows
            .iter()
            .map(|row| row.iter().map(value_label).collect())
            .collect();
        // The result grid shares the selection model (step 86.1,
        // item M101); the window keeps one session selection.
        let selection = app.sql_selection(result.rows.len(), result.columns.len());
        let mut selection_intent = None;
        let mut select_column_intent = None;
        crate::datasheet::grid_ui(
            ui,
            Grid {
                metrics,
                columns: &result.columns,
                body: &body,
                cursor: 0,
                selection: &selection,
                widths: &mut widths,
                sort_orders: &sorts,
                filters: &std::collections::BTreeMap::new(),
                follow_targets: &[],
                follow_counts: &|_: usize, _: usize, _: usize| None,
                id_salt: "sql.grid".to_owned(),
                frozen: 0,
                delete_reason: None,
                edit: None,
                edit_restricted: true,
                pending: &[],
                locked: false,
                max_height: Some(metrics.row_height * RESULT_MAX_ROWS),
            },
            crate::datasheet::grid_ui_callbacks(
                &mut |_row: usize| {},
                Some(&mut |new_selection| selection_intent = Some(new_selection)),
                Some(&mut |column: usize, extend: bool| {
                    select_column_intent = Some((column, extend));
                }),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        if let Some(new_selection) = selection_intent {
            app.set_sql_selection(new_selection);
        }
        if let Some((column, extend)) = select_column_intent {
            let current = app.sql_selection(result.rows.len(), result.columns.len());
            let new_selection = if extend {
                current.extend_columns(column)
            } else {
                datacraft_core::selection::Selection::select_column(column)
            };
            app.set_sql_selection(new_selection);
        }
        ui.label(
            egui::RichText::new(plural(
                result.rows.len(),
                "{count} result row",
                "{count} result rows",
            ))
            .small(),
        );
    }

    app.set_sql_column_widths(widths);
    app.set_sql_text(text);
    if run {
        app.run_sql();
    }
}
