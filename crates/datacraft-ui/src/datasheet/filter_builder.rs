//! The filter builder window (step 87.4, item M104): a draft/apply
//! distinction — editing mutates the per-tab draft, nothing runs until
//! Apply; the parenthesized model form renders as the preview (the
//! builder-to-criteria round trip is visible); the text mode edits the
//! raw criteria as the escape hatch; and the sort line dispatches the
//! existing single-column sort. A draft the dialect cannot express
//! refuses precisely and keeps the window open — an invalid draft
//! never runs.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::filter_builder::{
    BuilderCondition, BuilderOp, BuilderSort, Connective, FilterNode, serialize_filters,
};
use datacraft_core::qbe::SortOrder;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::app::filter_builder::{BuilderPane, FilterBuilderDraft};
use crate::i18n::{t, tr};

/// The condition operators the builder offers (the criteria dialect's
/// supported subset, step 87.3).
const OPERATORS: [BuilderOp; 7] = [
    BuilderOp::Equal,
    BuilderOp::NotEqual,
    BuilderOp::Less,
    BuilderOp::LessEqual,
    BuilderOp::Greater,
    BuilderOp::GreaterEqual,
    BuilderOp::Like,
];

/// The builder window over one table tab. Rendered only while the tab
/// carries a draft — the window's chrome never floats without one (the
/// layout snapshot caught the empty frame).
pub(crate) fn filter_builder_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    ui: &mut egui::Ui,
) {
    if app.filter_builder_draft(tab.id).is_none() {
        return;
    }
    let mut apply = false;
    let mut clear = false;
    let mut cancel = false;
    let title = tr("Filter Builder — {table}", &[("table", &tab.object_name)]);
    egui::Window::new(title)
        .id(egui::Id::unique(format!(
            "datacraft.filter_builder.{}",
            tab.ui_test_id
        )))
        .default_width(640.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(640.0, 420.0),
        ))
        .show(ui, |ui| {
            let Some(draft) = app.filter_builder_draft(tab.id) else {
                return;
            };
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(draft.mode == BuilderPane::Builder, t("Builder"))
                    .clicked()
                {
                    draft.mode = BuilderPane::Builder;
                }
                if ui
                    .selectable_label(draft.mode == BuilderPane::Text, t("Text"))
                    .clicked()
                {
                    draft.mode = BuilderPane::Text;
                }
            });
            ui.separator();
            match draft.mode {
                BuilderPane::Builder => builder_pane(draft, columns, ui),
                BuilderPane::Text => text_pane(draft, columns, ui),
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(t("Apply")).clicked() {
                    apply = true;
                }
                if ui.button(t("Clear Filter")).clicked() {
                    clear = true;
                }
                if ui.button(t("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    // The dispatch phase (the draft borrow ended with the window pass):
    // Apply serializes the active pane through the core model — a
    // refusal records the precise message and keeps the window open.
    if apply {
        match app.filter_builder_draft(tab.id).map(|draft| draft.mode) {
            Some(BuilderPane::Builder) => apply_builder(app, tab, columns),
            _ => apply_text(app, tab, columns),
        }
    }
    if clear {
        clear_all(app, tab);
        app.close_filter_builder(tab.id);
    }
    if cancel {
        app.close_filter_builder(tab.id);
    }
}

/// The structured pane: one condition row per draft entry (column
/// picker, operator picker, labelled value edit), the parenthesized
/// model form as the preview, and the sort line.
fn builder_pane(draft: &mut FilterBuilderDraft, columns: &[String], ui: &mut egui::Ui) {
    let default_column = columns.first().cloned().unwrap_or_default();
    let mut remove: Option<usize> = None;
    for row in 0..draft.conditions.len() {
        ui.horizontal(|ui| {
            let condition = &mut draft.conditions[row];
            egui::ComboBox::from_label(tr("Column — {row}", &[("row", &(row + 1).to_string())]))
                .selected_text(&condition.column)
                .show_ui(ui, |ui| {
                    for column in columns {
                        ui.selectable_label(
                            condition.column.eq_ignore_ascii_case(column),
                            column.as_str(),
                        )
                        .clicked()
                        .then(|| condition.column = column.clone());
                    }
                });
            egui::ComboBox::from_label(tr("Operator — {row}", &[("row", &(row + 1).to_string())]))
                .selected_text(condition.op.criteria_prefix())
                .show_ui(ui, |ui| {
                    for op in OPERATORS {
                        ui.selectable_label(condition.op == op, op.criteria_prefix())
                            .clicked()
                            .then(|| condition.op = op);
                    }
                });
            let value_label = ui.label(tr("Value — {row}", &[("row", &(row + 1).to_string())]));
            let value_edit = ui.add(egui::TextEdit::singleline(&mut condition.literal));
            value_edit.labelled_by(value_label.id);
            if ui.button(t("Remove")).clicked() {
                remove = Some(row);
            }
        });
    }
    if let Some(row) = remove {
        draft.conditions.remove(row);
    }
    if ui.button(t("Add condition")).clicked() {
        draft.conditions.push(BuilderCondition {
            column: default_column.clone(),
            op: BuilderOp::Equal,
            literal: String::new(),
        });
    }
    // The preview: the parenthesized model form — the same string the
    // behaviour doc pins (the round trip is visible before Apply).
    let model = and_model(&draft.conditions);
    ui.label(egui::RichText::new(model.to_string()).weak());
    // The sort line (the existing single-column path, step 87.3).
    let sort = draft.sort.get_or_insert_with(|| BuilderSort {
        column: default_column.clone(),
        order: SortOrder::None,
    });
    ui.horizontal(|ui| {
        ui.label(t("Sort"));
        egui::ComboBox::from_label("Sort column")
            .selected_text(&sort.column)
            .show_ui(ui, |ui| {
                for column in columns {
                    ui.selectable_label(sort.column.eq_ignore_ascii_case(column), column.as_str())
                        .clicked()
                        .then(|| sort.column = column.clone());
                }
            });
        egui::ComboBox::from_label("Sort order")
            .selected_text(sort_text(sort.order))
            .show_ui(ui, |ui| {
                for order in [SortOrder::Ascending, SortOrder::Descending, SortOrder::None] {
                    ui.selectable_label(sort.order == order, sort_text(order))
                        .clicked()
                        .then(|| sort.order = order);
                }
            });
    });
}

/// The raw-criteria pane: one labelled text edit per column — the
/// dialect's own syntax, unchanged (the escape hatch).
fn text_pane(draft: &mut FilterBuilderDraft, columns: &[String], ui: &mut egui::Ui) {
    for column in columns {
        ui.horizontal(|ui| {
            let entry = draft.text.entry(column.clone()).or_default();
            let label = ui.label(tr("criteria — {column}", &[("column", column.as_str())]));
            let edit = ui.add(egui::TextEdit::singleline(entry));
            edit.labelled_by(label.id);
        });
    }
}

/// The builder pane's dispatch: serialize the draft through the core
/// model, replace the tab's live filter set with the result, and apply
/// the sort line. A refusal records the precise message and the window
/// stays open (an invalid draft never runs).
fn apply_builder(app: &mut App, tab: &TabSnapshot, columns: &[String]) {
    let conditions = match app.filter_builder_draft(tab.id) {
        Some(draft) => draft.conditions.clone(),
        None => return,
    };
    let model = and_model(&conditions);
    let filters = match serialize_filters(&model) {
        Ok(filters) => filters,
        Err(reason) => {
            app.record_error(reason.to_string());
            return;
        }
    };
    apply_filter_map(app, tab, &filters);
    let sort = app
        .filter_builder_draft(tab.id)
        .and_then(|draft| draft.sort.clone());
    if let Some(sort) = sort
        && !sort.column.is_empty()
    {
        app.dispatch(Command::SetTableSort {
            table: tab.object_name.clone(),
            field: sort.column,
            order: sort.order,
        });
    }
    app.close_filter_builder(tab.id);
    let _ = columns;
}

/// The text pane's dispatch: every column's raw criteria replaces the
/// live filter (empty clears) — the dialect and its wildcard rule are
/// untouched.
fn apply_text(app: &mut App, tab: &TabSnapshot, columns: &[String]) {
    let text = match app.filter_builder_draft(tab.id) {
        Some(draft) => draft.text.clone(),
        None => return,
    };
    let mut filters = std::collections::BTreeMap::new();
    for column in columns {
        match text.get(column).map(String::as_str) {
            Some(criteria) if !criteria.trim().is_empty() => {
                filters.insert(column.clone(), criteria.trim().to_owned());
            }
            _ => {}
        }
    }
    apply_filter_map(app, tab, &filters);
    app.close_filter_builder(tab.id);
}

/// Replaces the tab's live filter set: the previously filtered columns
/// clear first, then the map applies (the follow path's rule — the
/// draft must not land in a skewed view).
fn apply_filter_map(
    app: &mut App,
    tab: &TabSnapshot,
    filters: &std::collections::BTreeMap<String, String>,
) {
    let snapshot = app.snapshot();
    let live: Vec<String> = snapshot
        .tabs
        .iter()
        .find(|candidate| candidate.id == tab.id)
        .map(|candidate| candidate.filters.keys().cloned().collect())
        .unwrap_or_default();
    for field in live {
        app.dispatch(Command::SetTableFilter {
            table: tab.object_name.clone(),
            field,
            criteria: None,
        });
    }
    for (field, criteria) in filters {
        app.dispatch(Command::SetTableFilter {
            table: tab.object_name.clone(),
            field: field.clone(),
            criteria: Some(criteria.clone()),
        });
    }
}

/// Clears every filter of the tab and the sort.
fn clear_all(app: &mut App, tab: &TabSnapshot) {
    let snapshot = app.snapshot();
    let live: Vec<String> = snapshot
        .tabs
        .iter()
        .find(|candidate| candidate.id == tab.id)
        .map(|candidate| candidate.filters.keys().cloned().collect())
        .unwrap_or_default();
    for field in live {
        app.dispatch(Command::SetTableFilter {
            table: tab.object_name.clone(),
            field,
            criteria: None,
        });
    }
    app.dispatch(Command::SetTableSort {
        table: tab.object_name.clone(),
        field: tab.object_name.clone(),
        order: SortOrder::None,
    });
}

/// The draft's conditions as the core model's AND group.
fn and_model(conditions: &[BuilderCondition]) -> FilterNode {
    FilterNode::Group {
        connective: Connective::And,
        children: conditions
            .iter()
            .cloned()
            .map(FilterNode::Condition)
            .collect(),
    }
}

fn sort_text(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Ascending => "Sort Ascending",
        SortOrder::Descending => "Sort Descending",
        SortOrder::None => "Clear Sort",
    }
}
