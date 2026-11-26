//! Find and replace over the visible rows (phase 101, step 101.3; item
//! M132).
//!
//! The window carries the query; the core computes the matches over the
//! datasheet's current display order and the replaced text. Replace All
//! stages every match into the tab's edit buffer, so the batch applies as
//! one atomic write on Ctrl+S — the window never writes to the database
//! itself.

use eframe::egui;

use datacraft_core::schema::{FieldKind, FieldValue, TableDef};
use datacraft_core::snapshot::TabSnapshot;
use datacraft_core::table_view::{FindRequest, find_matches};

use crate::app::App;
use crate::i18n::{t, tr};

/// The find/replace window's draft (session state).
#[derive(Debug, Clone, Default)]
pub struct FindReplaceDraft {
    /// The needle.
    pub find: String,
    /// What a match becomes.
    pub replace: String,
    /// Case-sensitive comparison.
    pub match_case: bool,
    /// The whole cell must equal the needle.
    pub whole_field: bool,
    /// One column to search; `None` searches every text column.
    pub column: Option<String>,
    /// How many cells the last Replace All staged.
    pub staged: Option<usize>,
    /// Why the last run failed, if it did.
    pub error: Option<String>,
}

impl App {
    /// Opens (or focuses) the find/replace window for the active tab.
    pub(crate) fn open_find_replace(&mut self) {
        if self.find_replace.is_none() {
            self.find_replace = Some(FindReplaceDraft::default());
        }
    }
}

/// Renders the open window, when one is open.
pub(crate) fn find_replace_ui(app: &mut App, ctx: &egui::Context) {
    if app.find_replace.is_none() {
        return;
    }
    let Some(tab) = app.snapshot().tabs.first().cloned() else {
        app.find_replace = None;
        return;
    };
    let columns: Vec<String> = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        })
        .map(|table| {
            table
                .fields
                .iter()
                .filter(|field| matches!(field.kind, FieldKind::Text { .. } | FieldKind::Memo))
                .map(|field| field.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let mut close = false;
    let mut replace_all = false;
    let mut draft = app.find_replace.take().expect("checked above");
    egui::Window::new(t("Find and Replace"))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_min_width(300.0);
            egui::Grid::new("find_replace.fields")
                .num_columns(2)
                .show(ui, |ui| {
                    let find_label = ui.label(t("Find"));
                    ui.add(egui::TextEdit::singleline(&mut draft.find))
                        .labelled_by(find_label.id);
                    ui.end_row();
                    let replace_label = ui.label(t("Replace with"));
                    ui.add(egui::TextEdit::singleline(&mut draft.replace))
                        .labelled_by(replace_label.id);
                    ui.end_row();
                });
            ui.checkbox(&mut draft.match_case, t("Match case"));
            ui.checkbox(&mut draft.whole_field, t("Whole field"));
            egui::ComboBox::from_label(t("Column"))
                .selected_text(
                    draft
                        .column
                        .clone()
                        .unwrap_or_else(|| t("All text columns").to_owned()),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(draft.column.is_none(), t("All text columns"))
                        .clicked()
                    {
                        draft.column = None;
                    }
                    for column in &columns {
                        if ui
                            .selectable_label(
                                draft.column.as_deref() == Some(column.as_str()),
                                column.as_str(),
                            )
                            .clicked()
                        {
                            draft.column = Some(column.clone());
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button(t("Replace All")).clicked() {
                    replace_all = true;
                }
                if ui.button(t("Close")).clicked() {
                    close = true;
                }
            });
            if let Some(count) = draft.staged {
                ui.weak(tr(
                    "Replacements staged: {count}",
                    &[("count", &count.to_string())],
                ));
            }
            if let Some(reason) = &draft.error {
                ui.colored_label(crate::theme::tokens(ui.visuals()).danger, reason.clone());
            }
        });
    if replace_all {
        let request = FindRequest {
            text: draft.find.clone(),
            replace_with: draft.replace.clone(),
            column: draft.column.clone(),
            match_case: draft.match_case,
            whole_field: draft.whole_field,
        };
        match stage_replacements(app, &tab, &request) {
            Ok(count) => {
                draft.staged = Some(count);
                draft.error = None;
            }
            Err(reason) => {
                draft.error = Some(reason);
                draft.staged = None;
            }
        }
    }
    if close {
        app.find_replace = None;
    } else {
        app.find_replace = Some(draft);
    }
}

/// Stages every match of the visible rows; returns how many cells it
/// staged. A match the buffer refuses reports its reason instead.
fn stage_replacements(
    app: &mut App,
    tab: &TabSnapshot,
    request: &FindRequest,
) -> Result<usize, String> {
    let Some(database) = app.snapshot().database.as_ref() else {
        return Err("no database is open".to_owned());
    };
    let Some(entry) = database
        .tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        .cloned()
    else {
        return Err(format!("unknown table {:?}", tab.object_name));
    };
    // The snapshot entry projects onto the engine's schema vocabulary
    // (names + kinds) — the same projection the datasheet renders from.
    let table = TableDef::new(
        entry.name,
        entry
            .fields
            .iter()
            .map(|field| datacraft_core::schema::FieldDef::new(field.name.clone(), field.kind))
            .collect(),
    );
    let Some(rows) = app.rows_for(tab.id).cloned() else {
        return Err("the rows are not loaded yet".to_owned());
    };
    let order = visible_order(&table, &rows, tab)?;
    let matches = find_matches(&table, &rows, &order, request)?;
    let mut staged = 0;
    for found in matches {
        let Some(mut image) = app.row_image(tab.id, found.row) else {
            continue;
        };
        image[found.column] = FieldValue::Text(found.replaced);
        if app.stage_cell_update(tab.id, found.row, image) {
            staged += 1;
        }
    }
    Ok(staged)
}

/// The datasheet's current display order (filters and sort applied).
fn visible_order(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    tab: &TabSnapshot,
) -> Result<Vec<usize>, String> {
    let sort = tab.sort_field.as_deref().map(|field| {
        (
            field,
            tab.sort_order
                .unwrap_or(datacraft_core::qbe::SortOrder::Ascending),
        )
    });
    datacraft_core::table_view::view_order(table, rows, sort, &tab.filters, &tab.value_filters)
        .map_err(|error| error.to_string())
}
