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
use datacraft_core::grid::{DEFAULT_COLUMN_WIDTH, GridMetrics};
use datacraft_core::qbe::{QbeRow, QbeValue, SortOrder};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::theme::tokens;

mod grid;
mod nav;

use grid::SELECTOR_WIDTH;
pub(crate) use grid::{Grid, grid_ui};

/// Renders the datasheet of one table tab.
pub fn datasheet_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(columns) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        })
        .map(|t| t.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>())
    else {
        ui.label(t("Table no longer exists"));
        return;
    };
    let Some(rows) = app.rows_for(tab.id) else {
        // A failed load surfaces its reason (step 39.4, item M31); without
        // one the cache is still on its first pass.
        if let Some(reason) = app.load_error(tab.id) {
            ui.colored_label(tokens(ui.visuals()).danger, reason.to_owned());
        } else {
            ui.label(t("Loading rows…"));
        }
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
    let frozen = datacraft_core::grid::effective_frozen(app.frozen_columns(tab.id), columns.len());
    let delete_reason = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::DELETE_RECORD,
    )
    .copied();
    let mut selected = None;
    let mut freeze_intent: Option<usize> = None;
    let mut delete_intent: Option<usize> = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_freeze = |count: usize| freeze_intent = Some(count);
        let mut on_delete = |row: usize| delete_intent = Some(row);
        grid_ui(
            ui,
            Grid {
                metrics: GridMetrics::default(),
                columns: &columns,
                body: &body,
                cursor,
                widths: &mut widths,
                sort_orders: &sorts,
                id_salt: format!("datasheet.grid.{}", tab.ui_test_id),
                frozen,
                delete_reason,
            },
            &mut on_select,
            None,
            Some(&mut on_freeze),
            Some(&mut on_delete),
        );
    }
    if let Some(count) = freeze_intent {
        app.set_frozen_columns(tab.id, count);
    }
    if let Some(row) = delete_intent {
        // Destructive-action confirmation through the dialog seam (step
        // 38.3, item M43); native dialogs speak the OS language.
        if app.dialogs.confirm(
            "Delete Record",
            "This permanently deletes the selected record from the database. Delete it?",
        ) {
            app.dispatch(Command::DeleteRow {
                table: tab.object_name.clone(),
                row,
            });
        }
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
    // Capability-disabled for the open source (ADR-7): the read-only
    // new-record row renders muted with the reason instead of editable
    // cells.
    let new_record_reason = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::NEW_RECORD,
    );
    let mut cells = app.new_row_cells(tab.id, columns.len());
    let mut add = false;
    egui::ScrollArea::horizontal()
        .id_salt(format!("datasheet.newrow.{}", tab.ui_test_id))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(SELECTOR_WIDTH);
                ui.label(egui::RichText::new("*").strong());
                for (index, cell) in cells.iter_mut().enumerate() {
                    let edit = egui::TextEdit::singleline(cell).hint_text(&columns[index]);
                    if new_record_reason.is_some() {
                        ui.add_enabled(
                            false,
                            edit.desired_width(
                                widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                            ),
                        );
                    } else {
                        ui.add_sized(
                            [
                                widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                                22.0,
                            ],
                            edit,
                        );
                    }
                }
            });
        });
    ui.horizontal(|ui| {
        ui.add_space(SELECTOR_WIDTH);
        let add_response = match new_record_reason {
            Some(reason) => ui
                .add_enabled(false, egui::Button::new(t("Add record")))
                .on_hover_text(crate::availability::reason_text(reason)),
            None => ui.button(t("Add record")),
        };
        if add_response.clicked() {
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
        // A failed run surfaces its reason (step 39.4, item M31).
        if let Some(reason) = app.load_error(tab.id) {
            ui.colored_label(tokens(ui.visuals()).danger, reason.to_owned());
        } else {
            ui.centered_and_justified(|ui| {
                ui.weak(t("The query produced no runnable result yet."))
            });
        }
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
    let frozen = datacraft_core::grid::effective_frozen(app.frozen_columns(tab.id), columns.len());
    let mut selected = None;
    let mut sort_intent: Option<(usize, SortOrder)> = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_freeze = |count: usize| app.set_frozen_columns(tab.id, count);
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
                id_salt: format!("query.grid.{}", tab.ui_test_id),
                frozen,
                delete_reason: None,
            },
            &mut on_select,
            sort,
            Some(&mut on_freeze),
            None,
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
