//! Runtime command-button actions (phase 64, item M69; ADR-8).
//!
//! A click dispatches exactly one bus command: open a form/report/query or
//! apply the form's own filter. An empty action keeps the button enabled
//! (Access has no auto-disable) and answers precisely in the status bar.

use eframe::egui;

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::form::{ButtonAction, ControlDef, ControlKind, FormFilter};

use crate::app::App;
use crate::i18n::tr;

/// Renders one command button and dispatches its action on click.
pub(crate) fn button_ui(app: &mut App, form: &str, control: &ControlDef, ui: &mut egui::Ui) {
    let ControlKind::Button { caption, action } = &control.kind else {
        return;
    };
    if ui.button(caption).clicked() {
        dispatch(app, form, control, action.as_ref());
    }
}

fn dispatch(app: &mut App, form: &str, control: &ControlDef, action: Option<&ButtonAction>) {
    let Some(action) = action else {
        app.last_error = Some(tr(
            "Button {button} has no action configured.",
            &[("button", &control.name)],
        ));
        app.last_notice = None;
        return;
    };
    let command = match action {
        ButtonAction::OpenForm { form: target } => {
            Command::OpenObject(ObjectId::new(ObjectKind::Form, target.clone()))
        }
        ButtonAction::OpenReport { report } => {
            Command::OpenObject(ObjectId::new(ObjectKind::Report, report.clone()))
        }
        ButtonAction::RunQuery { query } => {
            Command::OpenObject(ObjectId::new(ObjectKind::Query, query.clone()))
        }
        ButtonAction::ApplyFilter { criteria } => Command::SetFormFilter(FormFilter {
            form: form.to_owned(),
            criteria: Some(criteria.clone()),
        }),
    };
    app.dispatch(command);
}
