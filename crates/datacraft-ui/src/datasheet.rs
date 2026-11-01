//! Datasheet view (step 6.2, item H23; Access fidelity 12.5).
//!
//! Virtualized rendering over the per-tab row cache: egui's `show_rows`
//! paints only the visible window (the grid-core math of step 6.1 governs
//! the shared row/height metrics). The look is the app icon's grammar:
//! a teal header band with paper text, ink hairlines between columns, and
//! a row-selector column — clicking a selector moves the status bar's
//! record cursor. Cell widgets derive their ids from the
//! `datasheet.<table>.cell.<row>.<col>` convention.

use eframe::egui;

use datacraft_core::grid::GridMetrics;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::theme::tokens;

/// Default column width (points) before any resize logic lands.
const DEFAULT_COLUMN_WIDTH: f32 = 140.0;
/// Width of the row-selector column (Access's record selector).
const SELECTOR_WIDTH: f32 = 48.0;

/// Renders the datasheet of one table tab.
pub fn datasheet_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let metrics = GridMetrics::default();
    let Some(columns) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.tables.iter().find(|t| t.ui_test_id == tab.ui_test_id))
        .map(|t| t.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>())
    else {
        ui.label(t("Table no longer exists"));
        return;
    };
    let Some(rows) = app.rows_for(tab.id) else {
        ui.label(t("Loading rows…"));
        return;
    };
    let cursor = app.record_cursor(tab.id);
    let body: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(value_label).collect())
        .collect();
    let row_count = body.len();
    grid_ui(ui, &metrics, &columns, &body, cursor, |_ui, row_idx| {
        app.set_record_cursor(tab.id, row_idx, row_count);
    });
    ui.label(egui::RichText::new(plural(row_count, "{count} row", "{count} rows")).small());
    new_row_ui(app, tab, &columns, ui);
}

/// The datasheet's new-record row (step 11.7): one text cell per column,
/// marked with the Access asterisk. "Add record" parses the cells and
/// inserts through the bus; failures land in the status bar.
fn new_row_ui(app: &mut App, tab: &TabSnapshot, columns: &[String], ui: &mut egui::Ui) {
    ui.separator();
    let mut cells = app.new_row_cells(tab.id, columns.len());
    let mut add = false;
    egui::ScrollArea::horizontal()
        .id_salt(format!("datasheet.newrow.{}", tab.ui_test_id))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(SELECTOR_WIDTH);
                ui.label(egui::RichText::new("*").strong());
                for (index, cell) in cells.iter_mut().enumerate() {
                    ui.add_sized(
                        [DEFAULT_COLUMN_WIDTH, 22.0],
                        egui::TextEdit::singleline(cell).hint_text(&columns[index]),
                    );
                }
            });
        });
    ui.horizontal(|ui| {
        ui.add_space(SELECTOR_WIDTH);
        if ui.button(t("Add record")).clicked() {
            add = true;
        }
        ui.weak(t("Empty cells are Null; AutoNumber assigns itself."));
    });
    app.set_new_row_cells(tab.id, cells);
    if add {
        app.insert_new_row(tab.id, &tab.object_name);
    }
}

/// Renders the result grid of one query tab (Datasheet mode).
pub fn query_results_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let metrics = GridMetrics::default();
    let Some(result) = app.results_for(tab.id) else {
        ui.centered_and_justified(|ui| ui.weak(t("The query produced no runnable result yet.")));
        return;
    };
    let cursor = app.record_cursor(tab.id);
    let columns = result.columns.clone();
    let body: Vec<Vec<String>> = result
        .rows
        .iter()
        .map(|row| row.iter().map(value_label).collect())
        .collect();
    let row_count = body.len();
    grid_ui(ui, &metrics, &columns, &body, cursor, |_ui, row_idx| {
        app.set_record_cursor(tab.id, row_idx, row_count);
    });
    ui.label(
        egui::RichText::new(plural(
            row_count,
            "{count} result row",
            "{count} result rows",
        ))
        .small(),
    );
}

/// The shared Access grid: teal header band, ink hairlines, a clickable
/// row selector (reports the clicked row to `on_select`) and a washed
/// cursor row. Pure paint over pre-rendered cell strings.
pub(crate) fn grid_ui(
    ui: &mut egui::Ui,
    metrics: &GridMetrics,
    columns: &[String],
    body: &[Vec<String>],
    cursor: usize,
    mut on_select: impl FnMut(&mut egui::Ui, usize),
) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());

    // Header band: accent fill, on-accent text — the icon's header-row
    // grammar, in the contrast-checked accent variant.
    let header_top = ui.cursor().top();
    let band_width = SELECTOR_WIDTH + DEFAULT_COLUMN_WIDTH * columns.len().max(1) as f32;
    ui.horizontal(|ui| {
        ui.add_space(band_width);
    });
    let band = egui::Rect::from_min_size(
        egui::pos2(ui.cursor().left(), header_top),
        egui::vec2(band_width, metrics.header_height),
    );
    ui.painter().rect_filled(band, 0.0, tokens.accent_fill);
    for (col, name) in columns.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            band.left_top() + egui::vec2(SELECTOR_WIDTH + DEFAULT_COLUMN_WIDTH * col as f32, 0.0),
            egui::vec2(DEFAULT_COLUMN_WIDTH, metrics.header_height),
        );
        ui.painter().text(
            cell.left_center() + egui::vec2(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            tokens.on_accent,
        );
    }

    // Virtualized rows: egui computes the visible window; we paint cells.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, metrics.row_height, body.len(), |ui, range| {
            for row_idx in range {
                let is_cursor = row_idx == cursor.min(body.len().saturating_sub(1));
                ui.push_id(format!("row{row_idx}"), |ui| {
                    let inner = ui
                        .horizontal(|ui| {
                            let selector = ui.add_sized(
                                [SELECTOR_WIDTH, metrics.row_height],
                                egui::Button::selectable(
                                    is_cursor,
                                    egui::RichText::new(format!("{:>6}", row_idx + 1)).color(
                                        if is_cursor {
                                            tokens.accent_text
                                        } else {
                                            tokens.text
                                        },
                                    ),
                                ),
                            );
                            if selector.clicked() {
                                on_select(ui, row_idx);
                            }
                            for (col, text) in body[row_idx].iter().enumerate() {
                                let cell = egui::Rect::from_min_size(
                                    selector.rect.right_top()
                                        + egui::vec2(DEFAULT_COLUMN_WIDTH * col as f32, 0.0),
                                    egui::vec2(DEFAULT_COLUMN_WIDTH, metrics.row_height),
                                );
                                ui.painter_at(cell).text(
                                    cell.left_center() + egui::vec2(4.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    text,
                                    font.clone(),
                                    tokens.text,
                                );
                                let x = cell.right();
                                ui.painter().line_segment(
                                    [egui::pos2(x, cell.top()), egui::pos2(x, cell.bottom())],
                                    egui::Stroke::new(1.0, tokens.border),
                                );
                            }
                        })
                        .response;
                    if is_cursor {
                        ui.painter()
                            .rect_filled(inner.rect, 0.0, tokens.accent_faint);
                    }
                });
            }
        });
}
