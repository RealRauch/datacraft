//! Combo row-source panel in the form designer (phase 64, item M70;
//! ADR-8): the row source is a table or saved query, the display field
//! names the column that feeds the list. Applying dispatches exactly one
//! command; the core validates and canonicalizes both names.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlData, ControlDef, ControlKind, SetControlData};
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::i18n::t;

/// The selected combo's row-source panel; no-op for other control kinds.
pub(crate) fn combo_source_ui(
    app: &mut App,
    tab_id: TabId,
    form: &str,
    control: &ControlDef,
    ui: &mut egui::Ui,
) {
    let ControlKind::ComboBox {
        row_source,
        display_field,
        ..
    } = &control.kind
    else {
        return;
    };
    let (mut source, mut field) =
        app.combo_source_draft(tab_id, &control.name, row_source, display_field);
    ui.separator();
    ui.label(egui::RichText::new(t("Combo box")).strong());
    let source_label = ui.label(t("Row source"));
    let source_response = ui
        .add(egui::TextEdit::singleline(&mut source).desired_width(200.0))
        .labelled_by(source_label.id);
    let field_label = ui.label(t("Display field"));
    let field_response = ui
        .add(egui::TextEdit::singleline(&mut field).desired_width(200.0))
        .labelled_by(field_label.id);
    if source_response.changed() || field_response.changed() {
        app.set_combo_source_draft(tab_id, &control.name, source.clone(), field.clone());
    }
    if ui.button(t("Apply")).clicked() {
        let events = app.dispatch(Command::SetFormControlData(SetControlData {
            form: form.to_owned(),
            control: control.name.clone(),
            data: ControlData::ComboSource {
                row_source: source.clone(),
                display_field: field.clone(),
            },
        }));
        if events
            .iter()
            .any(|event| matches!(event, Event::ControlDataSet { .. }))
        {
            app.clear_combo_source_draft(tab_id, &control.name);
        }
    }
    ui.weak(t(
        "A table or saved query; every row is listed, duplicates included.",
    ));
}
