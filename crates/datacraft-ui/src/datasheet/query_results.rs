//! Query results grid (Datasheet mode of a query tab).
//!
//! Split from `datasheet.rs` for the file-size gate (step 86.1): the
//! result grid is the shared grid with the query's sort model — header
//! clicks and the header context menu write the column's QBE sort cell,
//! exactly one `SetQbeCell` per action (step 21.4).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::{QbeRow, QbeValue, SortOrder};
use datacraft_core::selection::Selection;
use datacraft_core::snapshot::TabSnapshot;

use super::{Grid, grid_ui};
use crate::app::App;
use crate::i18n::{plural, t};
use crate::labels::value_label;
use crate::theme::tokens;

/// Renders the result grid of one query tab (Datasheet mode). Header
/// clicks and the header context menu write the column's QBE sort cell —
/// exactly one `SetQbeCell` per action (step 21.4).
pub(crate) fn query_results_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
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
    // The result grid shares the selection model (step 86.1, item M101).
    let selection = app.selection(tab.id, row_count, columns.len());
    let (sort_orders, field_indices) = query_sort_state(app, &tab.object_name, columns.len());
    let mut widths = app.column_widths(tab.id, columns.len());
    let frozen = datacraft_core::grid::effective_frozen(app.frozen_columns(tab.id), columns.len());
    let mut selected = None;
    let mut selection_intent: Option<Selection> = None;
    let mut select_column_intent: Option<(usize, bool)> = None;
    let mut copy_intent = false;
    let mut sort_intent: Option<(usize, SortOrder)> = None;
    {
        let mut on_select = |row: usize| selected = Some(row);
        let mut on_selection = |new_selection: Selection| selection_intent = Some(new_selection);
        let mut on_select_column = |column: usize, extend: bool| {
            select_column_intent = Some((column, extend));
        };
        // Copy works on the read-only result grid; paste never fires
        // (edit_restricted keeps the key unconsumed).
        let mut on_copy = || copy_intent = true;
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
                selection: &selection,
                widths: &mut widths,
                sort_orders: &sort_orders,
                filters: &std::collections::BTreeMap::new(),
                follow_targets: &[],
                follow_counts: &|_: usize, _: usize, _: usize| None,
                id_salt: format!("query.grid.{}", tab.ui_test_id),
                frozen,
                delete_reason: None,
                edit: None,
                edit_restricted: true,
                pending: &[],
                locked: false,
                max_height: None,
            },
            crate::datasheet::grid_ui_callbacks(
                &mut on_select,
                Some(&mut on_selection),
                Some(&mut on_select_column),
                Some(&mut on_copy),
                None,
                None,
                None,
                None,
                sort,
                Some(&mut on_freeze),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
    }
    app.set_column_widths(tab.id, widths);
    if let Some(row) = selected {
        app.set_record_cursor(tab.id, row, row_count);
    }
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
    if copy_intent {
        match datacraft_core::clipboard::copy_tsv(&body, &selection, row_count, columns.len()) {
            Ok(text) => app.set_clipboard_text(text),
            Err(reason) => app.record_error(reason.to_string()),
        }
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
