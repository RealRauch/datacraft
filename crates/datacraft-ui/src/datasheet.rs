//! Datasheet view (step 6.2, item H23).
//!
//! Virtualized rendering over the per-tab row cache: egui's `show_rows`
//! paints only the visible window (the grid-core math of step 6.1 governs
//! the shared row/height metrics); headers come from the snapshot's field
//! names, cells from `present::display_value`. Cell widgets derive their
//! ids from the `datasheet.<table>.cell.<row>.<col>` convention.

use eframe::egui;

use datacraft_core::grid::GridMetrics;
use datacraft_core::present::display_value;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;

/// Default column width (points) before any resize logic lands.
const DEFAULT_COLUMN_WIDTH: f32 = 140.0;

/// Renders the datasheet of one table tab.
pub fn datasheet_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let metrics = GridMetrics::default();
    let Some(fields) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.tables.iter().find(|t| t.ui_test_id == tab.ui_test_id))
        .map(|t| t.fields.clone())
    else {
        ui.label("Table no longer exists");
        return;
    };
    let Some(rows) = app.rows_for(tab.id) else {
        ui.label("Loading rows…");
        return;
    };

    // Header row: fixed-width labels, one per field.
    ui.horizontal(|ui| {
        ui.add_sized([48.0, metrics.header_height], egui::Label::new(""));
        for field in &fields {
            ui.add_sized(
                [DEFAULT_COLUMN_WIDTH, metrics.header_height],
                egui::Label::new(egui::RichText::new(&field.name).strong()),
            );
        }
    });
    ui.separator();

    // Virtualized rows: egui computes the visible window; we paint cells.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, metrics.row_height, rows.len(), |ui, range| {
            for row_idx in range {
                let row = &rows[row_idx];
                ui.push_id(format!("{}.row{row_idx}", tab.ui_test_id), |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [48.0, metrics.row_height],
                            egui::Label::new(egui::RichText::new(format!("{row_idx:>6}")).weak()),
                        );
                        for (col, value) in row.iter().enumerate() {
                            ui.push_id(
                                format!(
                                    "datasheet.{}.cell.{row_idx}.{col}",
                                    tab.object_name.to_lowercase()
                                ),
                                |ui| {
                                    ui.add_sized(
                                        [DEFAULT_COLUMN_WIDTH, metrics.row_height],
                                        egui::Label::new(display_value(value))
                                            .wrap_mode(egui::TextWrapMode::Truncate),
                                    );
                                },
                            );
                        }
                    });
                });
            }
        });
    ui.separator();
    ui.label(egui::RichText::new(format!(
        "{} rows — record navigation and editing arrive with the write path (Phase 6 design view / H9).",
        rows.len()
    )).small());
}
