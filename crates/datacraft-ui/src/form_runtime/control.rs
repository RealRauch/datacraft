//! Per-control runtime rendering (split from `form_runtime.rs` for the
//! file-size gate, phase 102): the kind-specific widgets plus the phase-102
//! presentation properties (hidden, disabled, bold, alignment).

use eframe::egui;

use datacraft_core::availability::DisabledReason;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::ids::TabId;
use datacraft_core::schema::{FieldDef, FieldValue};

use crate::app::App;
use crate::i18n::{t, tr};
use crate::labels::{value_label, value_label_for};

use super::commit_text;

/// A static label at runtime (phase 102): the alignment property places
/// the text inside the control rect, bold selects the SemiBold face and a
/// disabled label renders weak.
fn label_ui(ui: &mut egui::Ui, control: &ControlDef, text: &str) {
    let align = match control.properties.align {
        datacraft_core::form::TextAlign::Left => egui::Align::Min,
        datacraft_core::form::TextAlign::Center => egui::Align::Center,
        datacraft_core::form::TextAlign::Right => egui::Align::Max,
    };
    ui.with_layout(egui::Layout::top_down(align), |ui| {
        if !control.properties.enabled {
            ui.weak(text);
        } else if control.properties.bold {
            ui.strong(text);
        } else {
            ui.label(text);
        }
    });
}

/// One control at runtime (bound fields show the record's value).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_control(
    ui: &mut egui::Ui,
    app: &mut App,
    tab_id: TabId,
    form: &str,
    control: &ControlDef,
    row: &[FieldValue],
    fields: &[FieldDef],
    index_of: &dyn Fn(&str) -> Option<usize>,
    record: usize,
    restriction: Option<DisabledReason>,
    esc_handled: &mut bool,
) {
    match &control.kind {
        ControlKind::Label { text } => {
            label_ui(ui, control, text);
        }
        ControlKind::TextBox { bound_field } => {
            let col = bound_field.as_deref().and_then(index_of);
            let value = col
                .map(|i| {
                    value_label_for(
                        app.form_pending_value(tab_id, i).unwrap_or(&row[i]),
                        Some(&fields[i].kind),
                    )
                })
                .unwrap_or_default();
            if let (Some(col), Some(field)) = (col, col.map(|i| &fields[i])) {
                text_box_ui(
                    ui,
                    app,
                    tab_id,
                    control,
                    field,
                    &value,
                    col,
                    record,
                    restriction,
                    esc_handled,
                );
            } else {
                ui.add_sized(
                    [ui.max_rect().width(), ui.max_rect().height()],
                    egui::TextEdit::singleline(&mut value.clone()).interactive(false),
                );
            }
        }
        ControlKind::CheckBox { bound_field } => {
            let Some(col) = bound_field.as_deref().and_then(index_of) else {
                let mut shown = false;
                ui.add_enabled(false, egui::Checkbox::new(&mut shown, ""));
                return;
            };
            let value = app.form_pending_value(tab_id, col).unwrap_or(&row[col]);
            let checked = matches!(value, FieldValue::Boolean(true));
            let mut shown = checked;
            let response = if restriction.is_some() || !control.properties.enabled {
                ui.add_enabled(false, egui::Checkbox::new(&mut shown, ""))
            } else {
                ui.add(egui::Checkbox::new(&mut shown, ""))
            }
            .accessible_name(control.name.clone());
            if let Some(reason) = restriction {
                response.on_hover_text(crate::availability::reason_text(&reason));
            } else if response.clicked() {
                // Two-state (Access TripleState=False): Null displays as
                // No; a click writes Yes.
                app.stage_form_value(tab_id, record, col, FieldValue::Boolean(!checked));
            }
        }
        ControlKind::ComboBox { bound_field, .. } => {
            let col = bound_field.as_deref().and_then(index_of);
            let value = col
                .map(|i| {
                    value_label_for(
                        app.form_pending_value(tab_id, i).unwrap_or(&row[i]),
                        Some(&fields[i].kind),
                    )
                })
                .unwrap_or_default();
            if let (Some(col), Some(field)) = (col, col.map(|i| &fields[i])) {
                combo_box_ui(
                    ui,
                    app,
                    tab_id,
                    control,
                    field,
                    &value,
                    col,
                    record,
                    restriction,
                    esc_handled,
                );
            } else {
                ui.weak(value);
            }
        }
        ControlKind::Button { .. } => {
            crate::form_actions::button_ui(app, form, control, ui);
        }
        ControlKind::SubForm { form } => {
            ui.weak(tr("[subform: {form}]", &[("form", form)]));
        }
    }
}

/// The editable bound text box (M68): Enter/focus loss stages the value,
/// Esc reverts the field (second press: the record). Read-only sources
/// render inert with the availability reason on hover.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_box_ui(
    ui: &mut egui::Ui,
    app: &mut App,
    tab_id: TabId,
    control: &ControlDef,
    field: &FieldDef,
    base: &str,
    col: usize,
    record: usize,
    restriction: Option<DisabledReason>,
    esc_handled: &mut bool,
) {
    let size = [ui.max_rect().width(), ui.max_rect().height()];
    let mut text = app
        .form_draft(tab_id, col)
        .map(str::to_owned)
        .unwrap_or_else(|| base.to_owned());
    if let Some(reason) = restriction {
        let response = ui
            .add_sized(
                size,
                egui::TextEdit::singleline(&mut text).interactive(false),
            )
            .accessible_name(control.name.clone());
        response.on_hover_text(crate::availability::reason_text(&reason));
        return;
    }
    // A disabled control renders inert (phase 102, item M133).
    if !control.properties.enabled {
        ui.add_sized(
            size,
            egui::TextEdit::singleline(&mut text).interactive(false),
        )
        .accessible_name(control.name.clone());
        return;
    }
    let response = ui
        .add_sized(size, egui::TextEdit::singleline(&mut text))
        .accessible_name(control.name.clone());
    if response.changed() {
        app.set_form_draft(tab_id, col, text.clone());
    }
    if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        *esc_handled = true;
        if app.form_esc_armed(tab_id) {
            app.revert_form_record(tab_id);
        } else {
            app.revert_form_field(tab_id, col);
        }
        return;
    }
    let committed = response.lost_focus()
        || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
    if committed {
        commit_text(app, tab_id, control, field, col, record, &text);
    }
}

/// The editable combo (M70; ADR-8): a free-entry text field plus the row
/// source list. Access `LimitToList = No` — typed values are stored like
/// any field value; the list shows every source row in source order,
/// duplicates included. An empty row source leaves free entry working.
#[allow(clippy::too_many_arguments)]
pub(super) fn combo_box_ui(
    ui: &mut egui::Ui,
    app: &mut App,
    tab_id: TabId,
    control: &ControlDef,
    field: &FieldDef,
    base: &str,
    col: usize,
    record: usize,
    restriction: Option<DisabledReason>,
    esc_handled: &mut bool,
) {
    let height = ui.max_rect().height();
    let width = ui.max_rect().width();
    let text_width = (width - 26.0).max(40.0);
    let mut text = app
        .form_draft(tab_id, col)
        .map(str::to_owned)
        .unwrap_or_else(|| base.to_owned());
    let editable = restriction.is_none() && control.properties.enabled;
    let mut committed: Option<String> = None;
    let mut escape = false;
    let mut selection: Option<FieldValue> = None;
    ui.horizontal(|ui| {
        let response = ui
            .add_sized(
                [text_width, height],
                egui::TextEdit::singleline(&mut text).interactive(editable),
            )
            .accessible_name(control.name.clone());
        if let Some(reason) = restriction {
            response.on_hover_text(crate::availability::reason_text(&reason));
        } else {
            if response.changed() {
                app.set_form_draft(tab_id, col, text.clone());
            }
            if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                escape = true;
            } else if response.lost_focus()
                || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                committed = Some(text.clone());
            }
        }
        let values = if editable {
            app.combo_values(tab_id, control)
        } else {
            Vec::new()
        };
        let menu = ui
            .add_enabled_ui(editable, |ui| {
                let menu = ui.menu_button("", |ui| {
                    if values.is_empty() {
                        ui.weak(t("No entries"));
                        return;
                    }
                    for (index, value) in values.iter().enumerate() {
                        ui.push_id(index, |ui| {
                            if ui.button(value_label(value)).clicked() {
                                selection = Some(value.clone());
                                ui.close();
                            }
                        });
                    }
                });
                // A painted caret, never a text glyph (step 91.10, item
                // M121) — the same affordance language the designer's combo
                // paints.
                let tokens = crate::theme::tokens(ui.visuals());
                let ink = if editable {
                    tokens.text
                } else {
                    tokens.text_muted
                };
                let caret = menu.response.rect.right_center() - egui::vec2(8.0, 0.0);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        caret + egui::vec2(-4.0, -2.0),
                        caret + egui::vec2(4.0, -2.0),
                        caret + egui::vec2(0.0, 3.0),
                    ],
                    ink,
                    egui::Stroke::NONE,
                ));
                crate::layout_probe::record(
                    "form.combo.caret",
                    egui::Rect::from_center_size(caret, egui::vec2(8.0, 5.0)),
                );
                menu.response
            })
            .inner;
        let arrow = menu.accessible_name(format!("{} values", control.name));
        if let Some(reason) = restriction {
            arrow.on_hover_text(crate::availability::reason_text(&reason));
        }
    });
    if escape {
        *esc_handled = true;
        if app.form_esc_armed(tab_id) {
            app.revert_form_record(tab_id);
        } else {
            app.revert_form_field(tab_id, col);
        }
        return;
    }
    if let Some(value) = selection {
        let current = app.form_pending_value(tab_id, col).cloned().or_else(|| {
            app.rows_for(tab_id)
                .and_then(|rows| rows.get(record))
                .and_then(|values| values.get(col))
                .cloned()
        });
        if current.as_ref() != Some(&value) {
            app.stage_form_value(tab_id, record, col, value);
        }
        app.clear_form_draft(tab_id, col);
        return;
    }
    if let Some(text) = committed {
        commit_text(app, tab_id, control, field, col, record, &text);
    }
}
