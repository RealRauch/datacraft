//! Datasheet view (step 6.2, item H23; Access fidelity 12.5; datasheet UX in
//! step 21.4, item U8).
//!
//! Virtualized rendering over the per-tab row cache: egui's `show_rows`
//! paints only the visible window (the grid-core math of step 6.1 governs
//! the shared row/height metrics, column resize math and minimum widths).
//! The look is the app icon's grammar: a teal header band with paper text,
//! ink hairlines between columns, alternating row shading, a row-selector
//! column and an accent edit marker on the selected row. Column widths are
//! per-session presentation state on `App`; a header click sorts through
//! the QBE sort model — one `SetQbeCell` command per action. Cell widgets
//! derive their ids from the `datasheet.<table>.cell.<row>.<col>`
//! convention.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::grid::{DEFAULT_COLUMN_WIDTH, GridMetrics, resize_column};
use datacraft_core::qbe::{QbeRow, QbeValue, SortOrder};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::theme::tokens;

/// Width of the row-selector column (Access's record selector).
const SELECTOR_WIDTH: f32 = 48.0;
/// Width of a column-separator drag handle.
const HANDLE_WIDTH: f32 = 5.0;

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller.
pub(crate) struct Grid<'a> {
    pub metrics: GridMetrics,
    pub columns: &'a [String],
    pub body: &'a [Vec<String>],
    pub cursor: usize,
    pub widths: &'a mut [f32],
    pub sort_orders: &'a [SortOrder],
}

/// Renders the datasheet of one table tab.
pub fn datasheet_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
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
    let sorts = vec![SortOrder::None; columns.len()];
    let mut widths = app.column_widths(tab.id, columns.len());
    let mut selected = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        grid_ui(
            ui,
            Grid {
                metrics: GridMetrics::default(),
                columns: &columns,
                body: &body,
                cursor,
                widths: &mut widths,
                sort_orders: &sorts,
            },
            &mut on_select,
            None,
        );
    }
    app.set_column_widths(tab.id, widths.clone());
    if let Some(row) = selected {
        app.set_record_cursor(tab.id, row, row_count);
    }
    ui.label(egui::RichText::new(plural(row_count, "{count} row", "{count} rows")).small());
    new_row_ui(app, tab, &columns, &widths, ui);
}

/// The datasheet's new-record row (step 11.7): one text cell per column,
/// marked with the Access asterisk. "Add record" parses the cells and
/// inserts through the bus; failures land in the status bar.
fn new_row_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    widths: &[f32],
    ui: &mut egui::Ui,
) {
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
                        [
                            widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                            22.0,
                        ],
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

/// Renders the result grid of one query tab (Datasheet mode). Header
/// clicks and the header context menu write the column's QBE sort cell —
/// exactly one `SetQbeCell` per action (step 21.4).
pub fn query_results_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
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
    let (sort_orders, field_indices) = query_sort_state(app, &tab.object_name, columns.len());
    let mut widths = app.column_widths(tab.id, columns.len());
    let mut selected = None;
    let mut sort_intent: Option<(usize, SortOrder)> = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_sort = |column: usize, order: SortOrder| {
            if let Some(field) = field_indices.get(column) {
                sort_intent = Some((*field, order));
            }
        };
        let sort = if field_indices.is_empty() {
            None
        } else {
            Some(&mut on_sort as &mut dyn FnMut(usize, SortOrder))
        };
        grid_ui(
            ui,
            Grid {
                metrics: GridMetrics::default(),
                columns: &columns,
                body: &body,
                cursor,
                widths: &mut widths,
                sort_orders: &sort_orders,
            },
            &mut on_select,
            sort,
        );
    }
    app.set_column_widths(tab.id, widths);
    if let Some(row) = selected {
        app.set_record_cursor(tab.id, row, row_count);
    }
    if let Some((field, order)) = sort_intent {
        app.dispatch(Command::SetQbeCell {
            query: tab.object_name.clone(),
            row: QbeRow::Sort,
            col: field,
            value: QbeValue::Text(order.to_string()),
        });
    }
    ui.label(
        egui::RichText::new(plural(
            row_count,
            "{count} result row",
            "{count} result rows",
        ))
        .small(),
    );
}

/// Result-column sort state of a sidecar query: per displayed column its
/// [`SortOrder`] and the QBE field index it addresses. Native queries have
/// no sidecar definition and therefore no sort targets.
fn query_sort_state(app: &App, query: &str, columns: usize) -> (Vec<SortOrder>, Vec<usize>) {
    let Some(def) = app.snapshot().database.as_ref().and_then(|db| {
        db.queries
            .iter()
            .find(|q| q.name.eq_ignore_ascii_case(query))
    }) else {
        return (vec![SortOrder::None; columns], Vec::new());
    };
    let indices: Vec<usize> = def
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.shown)
        .map(|(index, _)| index)
        .collect();
    let mut orders: Vec<SortOrder> = indices
        .iter()
        .map(|index| def.fields[*index].sort)
        .collect();
    orders.truncate(columns);
    orders.resize(columns, SortOrder::None);
    (orders, indices)
}

/// The shared Access grid: teal header band with resize handles and sort
/// affordances, alternating row shading, ink hairlines, a clickable row
/// selector (reports the clicked row to `on_select`) and an accent marker
/// on the cursor row.
pub(crate) fn grid_ui(
    ui: &mut egui::Ui,
    grid: Grid<'_>,
    on_select: &mut dyn FnMut(usize),
    on_sort: Option<&mut dyn FnMut(usize, SortOrder)>,
) {
    let Grid {
        metrics,
        columns,
        body,
        cursor,
        widths,
        sort_orders,
    } = grid;
    header_ui(ui, &metrics, columns, widths, sort_orders, on_sort);
    let offsets: Vec<f32> = {
        let mut offsets = Vec::with_capacity(widths.len());
        let mut acc = 0.0;
        for width in widths.iter() {
            offsets.push(acc);
            acc += width;
        }
        offsets
    };
    let total_width = SELECTOR_WIDTH + widths.iter().sum::<f32>();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());
    let mut clicked_row = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, metrics.row_height, body.len(), |ui, range| {
            for row_idx in range {
                let is_cursor = row_idx == cursor.min(body.len().saturating_sub(1));
                if row_idx % 2 == 1 {
                    let band = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(total_width, metrics.row_height),
                    );
                    ui.painter().rect_filled(band, 0.0, tokens.chrome);
                }
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
                                clicked_row = Some(row_idx);
                            }
                            for (col, text) in body[row_idx].iter().enumerate() {
                                let cell = egui::Rect::from_min_size(
                                    selector.rect.right_top()
                                        + egui::vec2(offsets.get(col).copied().unwrap_or(0.0), 0.0),
                                    egui::vec2(
                                        widths.get(col).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                                        metrics.row_height,
                                    ),
                                );
                                ui.painter_at(cell).text(
                                    cell.left_center() + egui::vec2(4.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    text,
                                    font.clone(),
                                    tokens.text,
                                );
                                ui.painter().line_segment(
                                    [egui::pos2(cell.right(), cell.top()), cell.right_bottom()],
                                    egui::Stroke::new(1.0, tokens.border),
                                );
                            }
                        })
                        .response;
                    if is_cursor {
                        ui.painter()
                            .rect_filled(inner.rect, 0.0, tokens.accent_faint);
                        let marker = egui::Rect::from_min_size(
                            inner.rect.left_top(),
                            egui::vec2(3.0, inner.rect.height()),
                        );
                        ui.painter().rect_filled(marker, 0.0, tokens.accent);
                    }
                });
            }
        });
    if let Some(row) = clicked_row {
        on_select(row);
    }
}

/// Header band: accent fill, column names with sort indicators, resize
/// handles on every right edge, and the sort affordances.
fn header_ui(
    ui: &mut egui::Ui,
    metrics: &GridMetrics,
    columns: &[String],
    widths: &mut [f32],
    sort_orders: &[SortOrder],
    mut on_sort: Option<&mut dyn FnMut(usize, SortOrder)>,
) {
    let tokens = tokens(ui.visuals());
    let font = egui::TextStyle::Body.resolve(ui.style());
    let sortable = on_sort.is_some();
    let mut pending: Option<(usize, SortOrder)> = None;
    ui.horizontal(|ui| {
        let (selector, _) = ui.allocate_exact_size(
            egui::vec2(SELECTOR_WIDTH, metrics.header_height),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(selector, 0.0, tokens.accent_fill);
        for (col, name) in columns.iter().enumerate() {
            let width = widths.get(col).copied().unwrap_or(DEFAULT_COLUMN_WIDTH);
            let cell_width = (width - HANDLE_WIDTH).max(1.0);
            let sense = if sortable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(cell_width, metrics.header_height), sense);
            ui.painter().rect_filled(rect, 0.0, tokens.accent_fill);
            let order = sort_orders.get(col).copied().unwrap_or_default();
            ui.painter().text(
                rect.left_center() + egui::vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{name}{}", sort_indicator(order)),
                font.clone(),
                tokens.on_accent,
            );
            if sortable {
                if response.clicked() {
                    pending = Some((col, order.toggled()));
                }
                response.context_menu(|ui| {
                    for target in [SortOrder::Ascending, SortOrder::Descending, SortOrder::None] {
                        let text = match target {
                            SortOrder::Ascending => t("Sort Ascending"),
                            SortOrder::Descending => t("Sort Descending"),
                            SortOrder::None => t("Clear Sort"),
                        };
                        if ui.selectable_label(order == target, text).clicked() {
                            pending = Some((col, target));
                            ui.close();
                        }
                    }
                });
            }
            let (handle, drag) = ui.allocate_exact_size(
                egui::vec2(HANDLE_WIDTH, metrics.header_height),
                egui::Sense::drag(),
            );
            if drag.hovered() || drag.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if drag.dragged()
                && let Some(width) = widths.get_mut(col)
            {
                *width = resize_column(*width, drag.drag_delta().x);
            }
            ui.painter().line_segment(
                [handle.left_top(), handle.left_bottom()],
                egui::Stroke::new(1.0, tokens.border),
            );
        }
    });
    if let Some((col, order)) = pending
        && let Some(callback) = on_sort.as_mut()
    {
        callback(col, order);
    }
}

/// Header sort glyph: unsorted shows nothing, sorted shows a caret.
fn sort_indicator(order: SortOrder) -> &'static str {
    match order {
        SortOrder::None => "",
        SortOrder::Ascending => " ▲",
        SortOrder::Descending => " ▼",
    }
}
