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
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;
use datacraft_core::selection::Selection;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::t;
use crate::labels::value_label;
use crate::theme::tokens;

mod filter_builder;
pub(crate) mod filter_dialog;
pub(crate) mod follow;
mod grid;
mod grid_cells;
mod grid_keys;
mod grid_model;
mod header;
mod nav;
mod query_results;
mod selector;
mod submit;
mod trailing;
mod value_viewer;

pub(crate) use grid::grid_ui;
pub(crate) use grid_model::{CellEditSlot, EditIntent, Grid, grid_ui_callbacks};
pub(crate) use query_results::query_results_ui;

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
    // The edit buffer's pending view (step 85.2, item H61): staged rows
    // render their pending image, staged inserts append after them. The
    // `slots` vector maps every displayed row onto its buffer row —
    // cache rows keep their index, void slots (discarded inserts) are
    // skipped so the grid never shows a hole.
    let staged = app.staged_rows(tab.id);
    let mut slots: Vec<usize> = display.clone();
    if let Some(buffer) = staged {
        slots.extend((rows.len()..buffer.row_count()).filter(|slot| {
            buffer.row_state(*slot) == datacraft_core::edit_buffer::RowState::Inserted
        }));
    }
    let body: Vec<Vec<String>> = staged
        .map(|buffer| {
            slots
                .iter()
                .map(|&slot| {
                    buffer
                        .row_image(slot)
                        .unwrap_or_else(|| rows.get(slot).map(Vec::as_slice).unwrap_or(&[]))
                        .iter()
                        .map(value_label)
                        .collect()
                })
                .collect()
        })
        .unwrap_or_else(|| {
            display
                .iter()
                .map(|&cache_row| rows[cache_row].iter().map(value_label).collect())
                .collect()
        });
    // Amber-marker positions per displayed row (step 85.2, item H61).
    let pending_rows: Vec<grid_model::PendingRow> = staged
        .map(|buffer| {
            slots
                .iter()
                .map(|&slot| grid_model::PendingRow {
                    state: buffer.row_state(slot),
                    changed: buffer.changed_cells(slot),
                })
                .collect()
        })
        .unwrap_or_default();
    let cursor = app.record_cursor(tab.id);
    let row_count = body.len();
    // The tab's selection (step 86.1, item M101): clamped against the
    // currently loaded bounds at read time — the survival rule.
    let selection = app.selection(tab.id, row_count, columns.len());
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
    // A submission in flight locks this grid (step 85.2, item H61) —
    // read before the editor's mutable borrow.
    let locked = app.snapshot().pending_write.is_some();
    // The follow targets (step 87.1, item M103): per column, the grid
    // paints one menu entry each; empty hides the affordance.
    let follow_labels = follow::follow_labels(app, tab, &columns);
    // The count suffix of one menu entry (step 87.2, item M103): the
    // frame-local context clones only while a follow context menu is
    // open — the callback reads it without borrowing `App`.
    let counts_context = follow::counts_context(app, tab, &columns, &slots);
    let follow_counts = move |row: usize, col: usize, index: usize| {
        counts_context
            .as_ref()
            .and_then(|context| context.suffix(row, col, index))
    };
    // The armed editor's buffer is mutably borrowed for the grid pass;
    // its intents apply afterwards (the borrow ends with the call). The
    // editor's row is a display index; the grid pass paints the same
    // indices.
    let editor = app.cell_edit_parts(tab.id);
    let edit = editor.map(|(row, col, buffer)| CellEditSlot { row, col, buffer });
    let mut selected = None;
    let mut selection_intent: Option<Selection> = None;
    let mut select_column_intent: Option<(usize, bool)> = None;
    let mut copy_intent = false;
    let mut paste_intent = false;
    let mut fill_intent = false;
    let mut clear_intent = false;
    let mut zoom_intent: Option<(usize, usize)> = None;
    let mut freeze_intent: Option<usize> = None;
    let mut delete_intent: Option<usize> = None;
    let mut pending_edit: Option<EditIntent> = None;
    let mut sort_intent: Option<(usize, SortOrder)> = None;
    let mut dialog_intent: Option<usize> = None;
    let mut follow_intent: Option<(usize, usize, usize)> = None;
    let mut follow_menu_intent: Option<(usize, usize)> = None;
    let mut builder_open_intent = false;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_selection = |new_selection: Selection| selection_intent = Some(new_selection);
        let mut on_select_column = |column: usize, extend: bool| {
            select_column_intent = Some((column, extend));
        };
        let mut on_copy = || copy_intent = true;
        let mut on_paste = || paste_intent = true;
        let mut on_fill = || fill_intent = true;
        let mut on_clear = || clear_intent = true;
        let mut on_zoom = |row: usize, col: usize| zoom_intent = Some((row, col));
        let mut on_freeze = |count: usize| freeze_intent = Some(count);
        let mut on_delete = |row: usize| delete_intent = Some(row);
        let mut on_edit = |intent: EditIntent| pending_edit = Some(intent);
        let mut on_sort = |column: usize, order: SortOrder| {
            sort_intent = Some((column, order));
        };
        let mut on_filter_dialog = |column: usize| {
            dialog_intent = Some(column);
        };
        // The grid's row viewport reserves the trailing panels' height
        // budget (step 48.2, item M61): without it the grid consumes the
        // full workspace height and pushes the row-count label and the
        // new-record row behind the status bar.
        let metrics = GridMetrics::default();
        let grid_height = (ui.available_height() - trailing::reserve_height(ui))
            .max(metrics.header_height + metrics.row_height);
        ui.allocate_ui(egui::vec2(ui.available_width(), grid_height), |ui| {
            grid_ui(
                ui,
                Grid {
                    metrics,
                    columns: &columns,
                    body: &body,
                    cursor,
                    selection: &selection,
                    widths: &mut widths,
                    sort_orders: &sorts,
                    filters: &tab.filters,
                    follow_targets: &follow_labels,
                    follow_counts: &follow_counts,
                    id_salt: format!("datasheet.grid.{}", tab.ui_test_id),
                    frozen,
                    delete_reason,
                    edit,
                    edit_restricted,
                    pending: &pending_rows,
                    locked,
                },
                crate::datasheet::grid_ui_callbacks(
                    &mut on_select,
                    Some(&mut on_selection),
                    Some(&mut on_select_column),
                    Some(&mut on_copy),
                    Some(&mut on_paste),
                    Some(&mut on_fill),
                    Some(&mut on_clear),
                    Some(&mut on_zoom),
                    Some(&mut on_sort),
                    Some(&mut on_freeze),
                    Some(&mut on_delete),
                    Some(&mut on_edit),
                    Some(&mut on_filter_dialog),
                    Some(&mut |row: usize, col: usize, index: usize| {
                        follow_intent = Some((row, col, index));
                    }),
                    Some(&mut |row: usize, col: usize| {
                        follow_menu_intent = Some((row, col));
                    }),
                    Some(&mut || builder_open_intent = true),
                ),
            );
        });
    }
    if let Some(count) = freeze_intent {
        app.set_frozen_columns(tab.id, count);
    }
    // Selection storage (step 86.1, item M101): keyboard and mouse
    // intents report the new selection; the header's column affordances
    // extend or start a column selection from the current anchor.
    if let Some(new_selection) = selection_intent {
        app.set_selection(tab.id, new_selection);
    }
    if let Some((column, extend)) = select_column_intent {
        let current = app.selection(tab.id, row_count, columns.len());
        let new_selection = if extend {
            current.extend_columns(column)
        } else {
            Selection::select_column(column)
        };
        app.set_selection(tab.id, new_selection);
    }
    // Clipboard intents (step 86.2, item M101): copy builds the TSV from
    // the displayed body; paste validates and writes through the edit
    // path (the rejection precedes every cell change).
    if copy_intent {
        match datacraft_core::clipboard::copy_tsv(&body, &selection, row_count, columns.len()) {
            Ok(text) => app.set_clipboard_text(text),
            Err(reason) => app.record_error(reason.to_string()),
        }
    }
    if paste_intent {
        app.paste_clipboard(tab.id, &tab.object_name, &slots);
    }
    if fill_intent {
        app.fill_selection(tab.id, &tab.object_name, &slots, selection);
    }
    if clear_intent {
        app.clear_selection(tab.id, &tab.object_name, &slots, selection);
    }
    if let Some((row, col)) = zoom_intent {
        app.open_value_viewer(tab.id, row, col);
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
    if let Some(column) = dialog_intent {
        app.open_filter_dialog(tab.id, column);
    }
    // The menu is open (step 87.2, item M103): queue its count reads —
    // duplicates answer from the map, one bounded read runs per frame.
    // A closed menu clears the marker, so the context stops cloning.
    if let Some((row, col)) = follow_menu_intent {
        follow::request_counts(app, tab, &columns, &slots, row, col);
    } else {
        app.clear_count_menu_tab();
    }
    app.poll_referenced_counts();
    if builder_open_intent {
        app.open_filter_builder(tab);
    }
    if let Some((row, col, index)) = follow_intent {
        // The follow reads the row image (the slots map the display row
        // onto the cache row; staged rows carry their pending image).
        let slot = slots.get(row).copied().unwrap_or(row);
        let values = app.row_image(tab.id, slot).unwrap_or_default();
        follow::follow_ui(app, tab, &columns, values, col, index);
    }
    match pending_edit {
        Some(EditIntent::Start { row, col }) => {
            let slot = slots.get(row).copied().unwrap_or(row);
            // Arming the editor collapses the selection onto the edited
            // cell (step 86.1, item M101).
            app.set_selection(tab.id, Selection::collapse(row, col));
            app.start_cell_edit(tab.id, row, slot, col);
        }
        Some(EditIntent::Commit { row, col }) => {
            let slot = slots.get(row).copied().unwrap_or(row);
            app.commit_cell_edit(tab.id, &tab.object_name, slot, col);
        }
        Some(EditIntent::Cancel) => app.cancel_cell_edit(tab.id),
        None => {}
    }
    if let Some(row) = delete_intent {
        let slot = slots.get(row).copied().unwrap_or(row);
        // Destructive-action confirmation through the dialog seam (step
        // 38.3, item M43; parked on a later frame, step 81.2, M97);
        // native dialogs speak the OS language. On an atomic-batch
        // driver the deletion stages into the edit buffer (step 85.2,
        // item H61) and removes nothing until the batch applies.
        let (message, intent) = if app.batch_writes() {
            (
                "This stages the record for deletion — it is removed when the batch applies. Delete it?"
                    .to_owned(),
                crate::app::dialog_flow::DialogIntent::StageRowDelete {
                    tab_id: tab.id,
                    slot,
                },
            )
        } else {
            (
                "This permanently deletes the selected record from the database.                           Delete it?"
                    .to_owned(),
                crate::app::dialog_flow::DialogIntent::DeleteRow {
                    table: tab.object_name.clone(),
                    row: slot,
                },
            )
        };
        app.request_dialog(
            crate::dialogs::DialogRequest::Confirm {
                title: "Delete Record".to_owned(),
                message,
            },
            intent,
        );
    }
    app.set_column_widths(tab.id, widths.clone());
    if let Some(row) = selected {
        app.set_record_cursor(tab.id, row, row_count);
    }
    // The Escape path discards the active record's staged changes — it
    // addresses the buffer slot, not the (sorted) display row.
    let cursor_row = app.record_cursor(tab.id);
    let cursor_slot = slots.get(cursor_row).copied().unwrap_or(cursor_row);
    app.set_cursor_slot(tab.id, cursor_slot);
    trailing::trailing_ui(app, tab, row_count, &columns, &widths, ui);
    filter_dialog::filter_dialog_ui(app, tab, &columns, ui);
    filter_builder::filter_builder_ui(app, tab, &columns, ui);
    submit::submission_ui(app, tab, &columns, ui);
    // The cell value viewer (step 86.4, item M102) rides the trailing
    // windows — after the in-place editor's commit, so a viewer Apply is
    // the newer intent and stages over it.
    value_viewer::value_viewer_ui(app, tab, &columns, &slots, ui);
}
