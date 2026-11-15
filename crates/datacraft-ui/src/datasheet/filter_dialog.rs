//! The column filter dialog (step 45.1, M55) — split from `datasheet.rs`
//! for the file-size gate (step 87.4, L30): egui menus do not host text
//! editing — clicking the in-menu editor closed the whole menu — so the
//! criteria editor lives in a small window (the mapper/sql window
//! pattern). The editor seeds from the live filter; Apply and Clear
//! dispatch the existing `SetTableFilter` commands. This dialog is the
//! filter builder's text mode (step 87.4, item M104): the raw dialect,
//! unchanged.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};

/// The criteria syntax the dialog accepts (step 48.3, item M62): the
/// M37-fenced dialect in one line — wildcards, quoting rule, comparison
/// operators and one example each. The English source is the i18n key;
/// the German catalog entry lives in `i18n/de/ui_chrome.rs`.
pub(crate) const FILTER_SYNTAX_HELP: &str = "Criteria: quoted Like patterns with the wildcards * (any run), ? (one character), # (one digit) — e.g. Like \"A*\" — or a comparison =, <>, <, <=, >, >= with a literal coerced to the column's kind, e.g. > 100.";

pub(crate) fn filter_dialog_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    ui: &mut egui::Ui,
) {
    let Some(column) = app.filter_dialog(tab.id) else {
        return;
    };
    let Some(field) = columns.get(column) else {
        app.close_filter_dialog(tab.id);
        return;
    };
    let title = tr("Filter — {field}", &[("field", field)]);
    let live = tab.filters.get(field).cloned().unwrap_or_default();
    let mut close = false;
    egui::Window::new(title)
        .id(egui::Id::unique(format!(
            "datacraft.filter.{}",
            tab.ui_test_id
        )))
        .default_width(320.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(320.0, 240.0),
        ))
        .show(ui, |ui| {
            let mut draft = {
                let buffered = app.filter_draft(tab.id, column);
                if buffered.is_empty() {
                    live.clone()
                } else {
                    buffered
                }
            };
            let criteria_label = ui.label(t("Filter criteria"));
            if ui
                .text_edit_singleline(&mut draft)
                .labelled_by(criteria_label.id)
                .changed()
            {
                app.set_filter_draft(tab.id, column, draft.clone());
            }
            // The dialog states the dialect it accepts (step 48.3, item
            // M62) — the user asked for the syntax in the walkthrough.
            ui.label(egui::RichText::new(t(FILTER_SYNTAX_HELP)).weak());
            ui.horizontal(|ui| {
                if ui.button(t("Apply Filter")).clicked() && !draft.trim().is_empty() {
                    app.dispatch(Command::SetTableFilter {
                        table: tab.object_name.clone(),
                        field: field.clone(),
                        criteria: Some(draft.clone()),
                    });
                    app.set_filter_draft(tab.id, column, String::new());
                    close = true;
                }
                if !live.is_empty() && ui.button(t("Clear Filter")).clicked() {
                    app.dispatch(Command::SetTableFilter {
                        table: tab.object_name.clone(),
                        field: field.clone(),
                        criteria: None,
                    });
                    app.set_filter_draft(tab.id, column, String::new());
                    close = true;
                }
            });
        });
    if close {
        app.close_filter_dialog(tab.id);
    }
}
