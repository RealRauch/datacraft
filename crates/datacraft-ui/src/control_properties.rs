//! The selected control's property sheet (phase 102, item M133).
//!
//! Beside the designer palette every placed control exposes its editable
//! properties: the name (buffered, one Apply), the geometry it occupies
//! (edited through the canvas commands) and the presentation properties
//! visible/enabled/bold/alignment. Each edit is exactly one bus command;
//! the core validates and the refreshed snapshot repaints the canvas.

use eframe::egui;

use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::{
    ControlDef, ControlProperty, DesignCommand, SetControlProperties, TextAlign,
};
use datacraft_core::ids::TabId;
use datacraft_core::{ObjectId, ObjectKind as Kind};

use crate::app::App;
use crate::i18n::{t, tr};

/// The buffered name of one selected control (the other properties apply
/// on click and need no draft).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PropertyDraft {
    /// The control the draft belongs to; a new selection re-seeds it.
    pub control: String,
    /// The buffered name.
    pub name: String,
}

/// Renders the property sheet of the selected control (no selection, no
/// sheet). `controls` is the owning object's control list.
pub fn control_properties_ui(
    app: &mut App,
    tab_id: TabId,
    object_kind: ObjectKind,
    object: &str,
    controls: &[ControlDef],
    selection: Option<String>,
    ui: &mut egui::Ui,
) {
    let Some(selected) = selection else {
        return;
    };
    let Some(control) = controls.iter().find(|control| control.name == selected) else {
        return;
    };
    let properties = control.properties;
    ui.separator();
    ui.label(egui::RichText::new(t("Properties")).strong());

    // Name: buffered like the content panel; one Apply dispatches one
    // rename command and the selection follows the new canonical name.
    let draft = app.property_draft(tab_id, control);
    let mut buffer = draft.name;
    let label = ui.label(t("Name"));
    let field = ui.add(egui::TextEdit::singleline(&mut buffer).desired_width(180.0));
    if field.labelled_by(label.id).changed() {
        app.set_property_draft(tab_id, control, buffer.clone());
    }
    if ui.button(t("Rename")).clicked() {
        apply(
            app,
            tab_id,
            object_kind,
            object,
            &control.name,
            ControlProperty::Name(buffer.clone()),
        );
        app.clear_property_draft(tab_id);
    }

    // Geometry: persisted with the control and edited through the canvas
    // (the resize handles of step 102.2 dispatch the same rect command).
    ui.weak(tr(
        "Position {x}, {y} — size {width} x {height}",
        &[
            ("x", &control.rect.x.to_string()),
            ("y", &control.rect.y.to_string()),
            ("width", &control.rect.width.to_string()),
            ("height", &control.rect.height.to_string()),
        ],
    ));

    let mut visible = properties.visible;
    if ui.checkbox(&mut visible, t("Visible")).changed() {
        apply(
            app,
            tab_id,
            object_kind,
            object,
            &control.name,
            ControlProperty::Visible(visible),
        );
    }
    let mut enabled = properties.enabled;
    if ui.checkbox(&mut enabled, t("Enabled")).changed() {
        apply(
            app,
            tab_id,
            object_kind,
            object,
            &control.name,
            ControlProperty::Enabled(enabled),
        );
    }
    let mut bold = properties.bold;
    if ui.checkbox(&mut bold, t("Bold")).changed() {
        apply(
            app,
            tab_id,
            object_kind,
            object,
            &control.name,
            ControlProperty::Bold(bold),
        );
    }
    let mut chosen = None;
    egui::ComboBox::from_label(t("Alignment"))
        .selected_text(align_label(properties.align))
        .show_ui(ui, |ui| {
            for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                if ui
                    .selectable_label(properties.align == align, align_label(align))
                    .clicked()
                {
                    chosen = Some(align);
                }
            }
        });
    if let Some(align) = chosen {
        apply(
            app,
            tab_id,
            object_kind,
            object,
            &control.name,
            ControlProperty::Align(align),
        );
    }
    ui.weak(t(
        "Name, visibility, enabled state, weight and alignment persist in the sidecar.",
    ));
}

/// Localized label of one alignment choice.
fn align_label(align: TextAlign) -> &'static str {
    match align {
        TextAlign::Left => t("Left"),
        TextAlign::Center => t("Center"),
        TextAlign::Right => t("Right"),
    }
}

/// Dispatches exactly one property command and follows a rename in the
/// designer selection (the canvas keeps the new control selected).
fn apply(
    app: &mut App,
    tab_id: TabId,
    object_kind: ObjectKind,
    object: &str,
    control: &str,
    property: ControlProperty,
) {
    let events = app.dispatch(Command::Design(DesignCommand::SetControlProperties(
        SetControlProperties {
            object: ObjectId::new(object_kind, object.to_owned()),
            control: control.to_owned(),
            property,
        },
    )));
    let Some(Event::ControlPropertiesSet { control, .. }) = events
        .iter()
        .find(|event| matches!(event, Event::ControlPropertiesSet { .. }))
    else {
        return;
    };
    match object_kind {
        Kind::Form => app.set_form_selection(tab_id, control.clone()),
        Kind::Report => app.set_report_selection(tab_id, control.clone()),
        _ => {}
    }
}
