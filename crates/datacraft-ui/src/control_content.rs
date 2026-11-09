//! The selected control's content panel (step 63.2, item H56): every
//! placed control carries exactly one content property — the label text,
//! button caption, bound source field or embedded form — and this panel
//! edits it beside the designer palette. Applying dispatches exactly one
//! command; the preview/runtime re-renders from the refreshed snapshot.

use eframe::egui;

use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::i18n::{t, tr};

/// The kind-specific accessor of the one content property.
fn content_of(kind: &ControlKind) -> (&'static str, String) {
    match kind {
        ControlKind::Label { text } => ("Text", text.clone()),
        ControlKind::Button { caption } => ("Caption", caption.clone()),
        ControlKind::TextBox { bound_field }
        | ControlKind::CheckBox { bound_field }
        | ControlKind::ComboBox { bound_field, .. } => {
            ("Control source", bound_field.clone().unwrap_or_default())
        }
        ControlKind::SubForm { form } => ("Subform", form.clone()),
    }
}

/// The panel for the selected control; `apply` dispatches the designer's
/// own content command with `(control, value)`.
pub(crate) fn control_content_ui(
    app: &mut App,
    tab_id: TabId,
    selected: Option<String>,
    controls: &[ControlDef],
    allow_expression: bool,
    apply: &mut dyn FnMut(&mut App, &str, &str),
    ui: &mut egui::Ui,
) {
    let Some(name) = selected else {
        return;
    };
    let Some(control) = controls.iter().find(|c| c.name == name) else {
        return;
    };
    let (label_key, current) = content_of(&control.kind);
    let mut buffer = app
        .control_content_draft(tab_id, &control.name)
        .map(str::to_owned)
        .unwrap_or_else(|| current.clone());

    ui.separator();
    ui.label(egui::RichText::new(tr("Control {name}", &[("name", &control.name)])).strong());
    let label = ui.label(t(label_key));
    let field = ui.add(egui::TextEdit::singleline(&mut buffer).desired_width(200.0));
    if field.labelled_by(label.id).changed() {
        app.set_control_content_draft(tab_id, &control.name, buffer.clone());
    }
    ui.horizontal(|ui| {
        if ui.button(t("Apply")).clicked() {
            apply(app, &control.name, &buffer);
            app.clear_control_content_draft(tab_id);
        }
    });
    // The hint states the local rule (the core validates exactly this).
    let hint = match &control.kind {
        ControlKind::Label { .. } | ControlKind::Button { .. } => {
            t("The text renders when the view runs.")
        }
        ControlKind::SubForm { .. } => t("An existing form name."),
        _ if allow_expression => t("A field of the source table, or =expression."),
        _ => t("A field of the source table."),
    };
    ui.weak(hint);
}
