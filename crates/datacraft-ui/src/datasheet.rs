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
mod header;
mod nav;

use grid::SELECTOR_WIDTH;
pub(crate) use grid::{CellEditSlot, EditIntent, Grid, grid_ui, grid_ui_callbacks};

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
    // The datasheet's view model (step 44.5, item M54): sort and column
    // filters are core state on the tab snapshot; core computes the
    // display-to-cache order (filters AND, typed compare sorts) so the
    // UI only maps. Intents carry display indices and translate here —
    // the cache order (the one the port addresses rows by) never leaks
    // into the paints.
    // The snapshot's table entry projects onto a TableDef (names +
    // kinds — everything the engine's predicates need), so the view
    // order computes against the same schema the snapshot shows.
    let table_def = app.snapshot().database.as_ref().and_then(|db| {
        db.tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
            .map(|entry| {
                datacraft_core::schema::TableDef::new(
                    entry.name.clone(),
                    entry
                        .fields
                        .iter()
                        .map(|field| {
                            datacraft_core::schema::FieldDef::new(field.name.clone(), field.kind)
                        })
                        .collect(),
                )
            })
    });
    let display = table_def
        .as_ref()
        .and_then(|table| {
            let sort = tab
                .sort_field
                .as_deref()
                .map(|field| (field, tab.sort_order.unwrap_or(SortOrder::Ascending)));
            datacraft_core::table_view::view_order(table, rows, sort, &tab.filters).ok()
        })
        .unwrap_or_else(|| (0..rows.len()).collect());
    let cache_of_display: Vec<usize> = display.clone();
    let display_of_cache: std::collections::HashMap<usize, usize> = display
        .iter()
        .enumerate()
        .map(|(display_row, cache_row)| (*cache_row, display_row))
        .collect();
    let cursor = app.record_cursor(tab.id);
    let body: Vec<Vec<String>> = display
        .iter()
        .map(|&cache_row| rows[cache_row].iter().map(value_label).collect())
        .collect();
    let row_count = body.len();
    let sorts: Vec<SortOrder> = columns
        .iter()
        .map(|column| {
            if tab.sort_field.as_deref() == Some(column.as_str()) {
                tab.sort_order.unwrap_or(SortOrder::None)
            } else {
                SortOrder::None
            }
        })
        .collect();
    let mut filter_drafts: Vec<String> = (0..columns.len())
        .map(|col| {
            let live = tab.filters.get(&columns[col]).cloned();
            let draft = app.filter_draft(tab.id, col);
            if draft.is_empty() {
                live.unwrap_or_default()
            } else {
                draft
            }
        })
        .collect();
    let mut widths = app.column_widths(tab.id, columns.len());
    let frozen = datacraft_core::grid::effective_frozen(app.frozen_columns(tab.id), columns.len());
    let delete_reason = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::DELETE_RECORD,
    )
    .copied();
    let edit_restricted = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::EDIT_RECORD,
    )
    .is_some();
    // The armed editor's buffer is mutably borrowed for the grid pass;
    // its intents apply afterwards (the borrow ends with the call).
    let editor = app.cell_edit_parts(tab.id);
    let edit = editor.map(|(row, col, buffer)| CellEditSlot {
        row: display_of_cache.get(&row).copied().unwrap_or(row),
        col,
        buffer,
    });
    let mut selected = None;
    let mut freeze_intent: Option<usize> = None;
    let mut delete_intent: Option<usize> = None;
    let mut pending_edit: Option<EditIntent> = None;
    let mut sort_intent: Option<(usize, SortOrder)> = None;
    let mut filter_intent: Option<(usize, Option<String>)> = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_freeze = |count: usize| freeze_intent = Some(count);
        let mut on_delete = |row: usize| delete_intent = Some(row);
        let mut on_edit = |intent: EditIntent| pending_edit = Some(intent);
        let mut on_sort = |column: usize, order: SortOrder| {
            sort_intent = Some((column, order));
        };
        let mut on_filter = |column: usize, criteria: Option<String>| {
            filter_intent = Some((column, criteria));
        };
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
                edit,
                edit_restricted,
            },
            crate::datasheet::grid_ui_callbacks(
                &mut on_select,
                Some(&mut on_sort),
                Some(&mut on_freeze),
                Some(&mut on_delete),
                Some(&mut on_edit),
                Some(&mut on_filter),
                &mut filter_drafts,
            ),
        );
    }
    if let Some(count) = freeze_intent {
        app.set_frozen_columns(tab.id, count);
    }
    if let Some((column, criteria)) = filter_intent
        && let Some(field) = columns.get(column)
    {
        app.dispatch(Command::SetTableFilter {
            table: tab.object_name.clone(),
            field: field.clone(),
            criteria,
        });
        app.set_filter_draft(tab.id, column, String::new());
    }
    if let Some((column, order)) = sort_intent
        && let Some(field) = columns.get(column)
    {
        app.dispatch(Command::SetTableSort {
            table: tab.object_name.clone(),
            field: field.clone(),
            order,
        });
    }
    match pending_edit {
        Some(EditIntent::Start { row, col }) => {
            let cache_row = cache_of_display.get(row).copied().unwrap_or(row);
            app.start_cell_edit(tab.id, cache_row, col);
        }
        Some(EditIntent::Commit { row, col }) => {
            let cache_row = cache_of_display.get(row).copied().unwrap_or(row);
            app.commit_cell_edit(tab.id, &tab.object_name, cache_row, col);
        }
        Some(EditIntent::Cancel) => app.cancel_cell_edit(tab.id),
        None => {}
    }
    if let Some(row) = delete_intent {
        let cache_row = cache_of_display.get(row).copied().unwrap_or(row);
        // Destructive-action confirmation through the dialog seam (step
        // 38.3, item M43); native dialogs speak the OS language.
        if app.dialogs.confirm(
            "Delete Record",
            "This permanently deletes the selected record from the database. Delete it?",
        ) {
            app.dispatch(Command::DeleteRow {
                table: tab.object_name.clone(),
                row: cache_row,
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
                edit: None,
                edit_restricted: true,
            },
            crate::datasheet::grid_ui_callbacks(
                &mut on_select,
                sort,
                Some(&mut on_freeze),
                None,
                None,
                None,
                &mut [],
            ),
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
