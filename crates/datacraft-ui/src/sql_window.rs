//! SQL window (step 13.3, item H37).
//!
//! A floating window following the relationship-mapper pattern: presence
//! comes from the snapshot, the editor text is presentation state, and
//! running dispatches `RunSql` through the command bus — the same command
//! agents send. The result grid is the shared datasheet grid.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::grid::GridMetrics;

use crate::app::App;
use crate::i18n::{plural, t};
use crate::labels::value_label;

/// Renders the SQL window (when open).
pub fn sql_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().sql_window_open {
        return;
    }
    let mut close = false;
    let mut run = false;
    let mut text = app.sql_text().to_owned();
    let error = app.sql_error().map(str::to_owned);
    let result = app.sql_result().cloned();

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
            if let Some(reason) = &error {
                ui.colored_label(egui::Color32::from_rgb(0xB0, 0x2A, 0x2A), reason);
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
                    &GridMetrics::default(),
                    &result.columns,
                    &body,
                    0,
                    |_ui, _row| {},
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

    app.set_sql_text(text);
    if run {
        app.run_sql();
    }
    if close {
        app.dispatch(Command::ToggleSqlWindow);
    }
}
