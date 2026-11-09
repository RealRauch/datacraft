//! Button-action panel in the form designer (phase 64, item M69; ADR-8).
//!
//! The picker offers the fixed bus vocabulary — open form, open report,
//! run query, apply filter — plus the empty action. Applying dispatches
//! exactly one command; the core validates and canonicalizes the target.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::{ButtonAction, ControlData, ControlDef, ControlKind, SetControlData};
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::i18n::t;

/// Localized labels of the picker; the index is the draft kind.
const KINDS: [&str; 5] = [
    "None",
    "Open form",
    "Open report",
    "Run query",
    "Apply filter",
];

/// The selected button's action panel; no-op for other control kinds.
pub(crate) fn button_action_ui(
    app: &mut App,
    tab_id: TabId,
    form: &str,
    control: &ControlDef,
    ui: &mut egui::Ui,
) {
    let ControlKind::Button { action, .. } = &control.kind else {
        return;
    };
    let mut draft = app.button_action_draft(tab_id, &control.name, action.as_ref());
    ui.separator();
    ui.label(egui::RichText::new(t("Button action")).strong());
    egui::ComboBox::from_label(t("Action"))
        .selected_text(t(KINDS[draft.kind]))
        .show_ui(ui, |ui| {
            for (index, label) in KINDS.iter().enumerate() {
                if ui.selectable_label(draft.kind == index, t(label)).clicked() {
                    draft.kind = index;
                    app.set_button_action_draft(tab_id, &control.name, draft.clone());
                }
            }
        });
    if draft.kind == 4 {
        let label = ui.label(t("Criteria"));
        let response = ui
            .add(egui::TextEdit::singleline(&mut draft.criteria).desired_width(200.0))
            .labelled_by(label.id);
        if response.changed() {
            app.set_button_action_draft(tab_id, &control.name, draft.clone());
        }
    } else if draft.kind != 0 {
        let label = ui.label(t("Target"));
        let response = ui
            .add(egui::TextEdit::singleline(&mut draft.target).desired_width(200.0))
            .labelled_by(label.id);
        if response.changed() {
            app.set_button_action_draft(tab_id, &control.name, draft.clone());
        }
    }
    if ui.button(t("Apply")).clicked() {
        let action = match draft.kind {
            0 => None,
            1 => Some(ButtonAction::OpenForm {
                form: draft.target.clone(),
            }),
            2 => Some(ButtonAction::OpenReport {
                report: draft.target.clone(),
            }),
            3 => Some(ButtonAction::RunQuery {
                query: draft.target.clone(),
            }),
            _ => Some(ButtonAction::ApplyFilter {
                criteria: draft.criteria.clone(),
            }),
        };
        let events = app.dispatch(Command::SetFormControlData(SetControlData {
            form: form.to_owned(),
            control: control.name.clone(),
            data: ControlData::Button { action },
        }));
        if events
            .iter()
            .any(|event| matches!(event, Event::ControlDataSet { .. }))
        {
            app.clear_button_action_draft(tab_id, &control.name);
        }
    }
    ui.weak(t(
        "A form, report or query name; the filter takes an Access WHERE predicate.",
    ));
}
