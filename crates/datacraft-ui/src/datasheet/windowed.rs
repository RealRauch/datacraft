//! Windowed rendering of a large table (phase 101, step 101.5; item
//! M132).
//!
//! A table above [`crate::row_window::WINDOW_THRESHOLD`] renders from its
//! bounded window cache: the row count sizes the virtual list, loaded
//! windows paint real cells, everything else shows the placeholder. The
//! grid reports its visible range every frame and the app loads the
//! windows it needs; "Load all rows" upgrades the tab explicitly, and a
//! filter, a sort or a staged batch upgrades on its own.

use eframe::egui;

use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;
use datacraft_core::schema::FieldValue;
use datacraft_core::selection::Selection;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::datasheet::grid_model::{Grid, GridCallbacks};
use crate::i18n::{t, tr};
use crate::labels::value_label_for;

/// Placeholder of a row whose window is not loaded.
const PENDING_ROW: &str = "…";

/// Renders the windowed datasheet of one table tab.
pub(crate) fn windowed_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    // A filter, a sort or a staged batch needs the whole table: upgrade
    // now, the next frame renders the full datasheet.
    if !tab.filters.is_empty()
        || !tab.value_filters.is_empty()
        || tab.sort_field.is_some()
        || app.has_staged_edits(tab.id)
    {
        app.load_all_rows(tab.id, &tab.object_name);
        ui.label(t("Loading all rows…"));
        return;
    }
    let Some(entry) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        })
        .cloned()
    else {
        ui.label(t("Table no longer exists"));
        return;
    };
    let order = app.column_order(tab.id, entry.fields.len());
    let columns: Vec<String> = order
        .iter()
        .map(|&storage| entry.fields[storage].name.clone())
        .collect();
    // The list keeps its true height without reading a row: unloaded rows
    // carry one placeholder cell.
    let (total, loaded, body) = {
        let Some(window) = app.row_window(tab.id) else {
            return;
        };
        let loaded: usize = window.loaded.values().map(Vec::len).sum();
        let body: Vec<Vec<String>> = (0..window.total)
            .map(|row| match window.row(row) {
                Some(values) => order
                    .iter()
                    .map(|&storage| match values.get(storage) {
                        Some(FieldValue::Null) | None => t("(NULL)").to_owned(),
                        Some(value) => value_label_for(
                            value,
                            entry.fields.get(storage).map(|field| &field.kind),
                        ),
                    })
                    .collect(),
                None => vec![PENDING_ROW.to_owned()],
            })
            .collect();
        (window.total, loaded, body)
    };
    let mut widths = app.column_widths(tab.id, columns.len());
    // The windowed state line sits above the grid: the grid fills the
    // remaining height, so a footer would be pushed out of the pane.
    ui.horizontal_wrapped(|ui| {
        ui.weak(tr(
            "Large table — {loaded} of {total} rows are loaded in windows.",
            &[
                ("loaded", &loaded.to_string()),
                ("total", &total.to_string()),
            ],
        ));
        if ui
            .button(t("Load all rows"))
            .on_hover_text(t(
                "Reads every row: filters, sorts and edits need the whole table.",
            ))
            .clicked()
        {
            app.load_all_rows(tab.id, &tab.object_name);
        }
    });
    ui.separator();
    let sorts = vec![SortOrder::None; columns.len()];
    let filters = std::collections::BTreeMap::new();
    let no_targets: Vec<Vec<String>> = vec![Vec::new(); columns.len()];
    let no_counts = |_: usize, _: usize, _: usize| None;
    let selection = Selection::collapse(0, 0);
    let pending = Vec::new();
    let mut selected = None;
    let mut visible = None;
    let mut on_select = |row: usize| selected = Some(row);
    let mut on_visible = |first: usize, last: usize| visible = Some((first, last));
    crate::datasheet::grid_ui(
        ui,
        Grid {
            metrics: GridMetrics::default(),
            columns: &columns,
            body: &body,
            cursor: app.record_cursor(tab.id),
            selection: &selection,
            widths: &mut widths,
            sort_orders: &sorts,
            filters: &filters,
            follow_targets: &no_targets,
            follow_counts: &no_counts,
            id_salt: format!("windowed.{}", tab.ui_test_id),
            frozen: 0,
            delete_reason: None,
            edit: None,
            edit_restricted: true,
            pending: &pending,
            locked: true,
            max_height: None,
        },
        GridCallbacks {
            on_select: &mut on_select,
            on_selection: None,
            on_select_column: None,
            on_copy: None,
            on_paste: None,
            on_fill: None,
            on_clear: None,
            on_zoom: None,
            on_sort: None,
            on_freeze: None,
            on_delete: None,
            on_edit: None,
            on_filter_dialog: None,
            on_value_list: None,
            on_move_column: None,
            on_visible_rows: Some(&mut on_visible),
            on_follow: None,
            on_follow_menu: None,
            filter_builder: None,
        },
    );
    if let Some(row) = selected {
        app.set_record_cursor(tab.id, row, total);
    }
    if let Some((first, last)) = visible {
        app.ensure_row_windows(tab.id, &tab.object_name, first, last);
    }
}
