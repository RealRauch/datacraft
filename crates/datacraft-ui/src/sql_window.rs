//! SQL window (step 13.3, item H37).
//!
//! A floating window following the relationship-mapper pattern: presence
//! comes from the snapshot, the editor text is presentation state, and
//! running dispatches `RunSql` through the command bus — the same command
//! agents send. The result grid is the shared datasheet grid.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;

use crate::app::App;
use crate::datasheet::Grid;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::theme::tokens;

/// Renders the SQL window (when open).
pub fn sql_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().sql_window_open {
        return;
    }
    let tokens = tokens(ui.visuals());
    let mut close = false;
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

    egui::Window::new(t("SQL"))
        .id(egui::Id::unique("datacraft.sql"))
        .default_width(720.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("Statement"));
                if ui.small_button(t("Close")).clicked() {
                    close = true;
                }
            });
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
                crate::datasheet::grid_ui(
                    ui,
                    Grid {
                        metrics: GridMetrics::default(),
                        columns: &result.columns,
                        body: &body,
                        cursor: 0,
                        widths: &mut widths,
                        sort_orders: &sorts,
                    },
                    &mut |_row: usize| {},
                    None,
                );
                ui.label(
                    egui::RichText::new(plural(
                        result.rows.len(),
                        "{count} result row",
                        "{count} result rows",
                    ))
                    .small(),
                );
            }
        });

    app.set_sql_column_widths(widths);
    app.set_sql_text(text);
    if run {
        app.run_sql();
    }
    if close {
        app.dispatch(Command::ToggleSqlWindow);
    }
}
