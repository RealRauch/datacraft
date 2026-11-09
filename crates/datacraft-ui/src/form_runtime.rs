//! Form runtime view (step 8.3, item H30; phase 64, items M68/M70; ADR-8).
//!
//! Renders bound control values for the current record. Typing edits a
//! per-record buffer (Enter or focus loss stages the control value; the
//! record is written with exactly one `UpdateRow` when it is left). `Esc`
//! reverts the field first, the whole record on the second press.

use eframe::egui;

use datacraft_core::availability::DisabledReason;
use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind, FormFilter};
use datacraft_core::ids::TabId;
use datacraft_core::schema::{FieldDef, FieldValue};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::labels::value_label;

/// Renders the runtime view of one form tab.
pub fn form_runtime_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let snapshot = app.snapshot().clone();
    let Some(form) = snapshot
        .database
        .as_ref()
        .and_then(|db| db.forms.iter().find(|f| f.name == tab.object_name))
        .cloned()
    else {
        ui.label(t("Form no longer exists"));
        return;
    };
    if form.source.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak(t(
                "This form has no source table yet — assign one in the designer.",
            ))
        });
        return;
    }
    let fields = app
        .table_definition(&form.source)
        .map(|table| table.fields.clone())
        .unwrap_or_default();
    let index_of = |name: &str| {
        fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(name))
    };
    // The form's own filter (ADR-8): a chip with the criteria and a
    // one-click clear; the datasheet's filters stay untouched.
    if let Some(criteria) = app.form_filter(&form.name).map(str::to_owned) {
        ui.horizontal(|ui| {
            ui.label(tr("Filter: {criteria}", &[("criteria", &criteria)]));
            if ui.button(t("Clear filter")).clicked() {
                app.dispatch(Command::SetFormFilter(FormFilter {
                    form: form.name.clone(),
                    criteria: None,
                }));
            }
        });
    }
    if let Some(reason) = app.form_filter_error(tab.id) {
        ui.label(tr(
            "Filter could not be applied: {reason}",
            &[("reason", reason)],
        ));
    }
    let Some(rows) = app.rows_for(tab.id).cloned() else {
        ui.label(t("Loading rows…"));
        return;
    };
    if rows.is_empty() {
        ui.centered_and_justified(|ui| ui.weak(t("The source table has no rows.")));
        return;
    }

    let indices = app
        .form_row_indices(tab.id)
        .map(<[usize]>::to_vec)
        .unwrap_or_else(|| (0..rows.len()).collect());
    let row_count = indices.len();
    if row_count == 0 {
        ui.centered_and_justified(|ui| ui.weak(t("No records match the filter.")));
        return;
    }
    let record = app.form_cursor(tab.id, &indices, row_count);
    let physical = indices[record];
    let row = rows[physical].clone();
    // ADR-8: a read-only source keeps the inert rendering and names why.
    let restriction =
        crate::availability::restricted(&snapshot, datacraft_core::availability::ids::EDIT_RECORD)
            .copied();

    let mut esc_handled = false;
    egui::ScrollArea::vertical()
        // Reserve the navigation row: an unconstrained scroll area takes
        // the whole available height and pushes the record controls below
        // the viewport (found by the editing kittest).
        .max_height((ui.available_height() - 32.0).max(80.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // The designed canvas (step 46.1, M57): the runtime paints
            // the same 640x400 surface as the designer and places every
            // control at its stored rect — the designer's arrangement is
            // what users see at runtime (ADR-1 WYSIWYG), instead of the
            // old top-left stack.
            let (canvas, _) = ui.allocate_exact_size(
                egui::vec2(
                    crate::form_design::FORM_WIDTH,
                    crate::form_design::FORM_HEIGHT,
                ),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(canvas);
            painter.rect_filled(canvas, 0.0, ui.visuals().extreme_bg_color);
            for control in &form.controls {
                ui.push_id(
                    format!(
                        "runtime.{}.control.{}",
                        form.name.to_lowercase(),
                        control.name.to_lowercase()
                    ),
                    |ui| {
                        let rect = egui::Rect::from_min_size(
                            canvas.min + egui::vec2(control.rect.x, control.rect.y),
                            egui::vec2(control.rect.width, control.rect.height),
                        );
                        crate::layout_probe::record(
                            &format!("runtime.{}.{}", form.name, control.name),
                            rect,
                        );
                        let mut control_ui =
                            ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(format!(
                                "runtime.canvas.{}.{}",
                                form.name.to_lowercase(),
                                control.name.to_lowercase()
                            )));
                        render_control(
                            &mut control_ui,
                            app,
                            tab.id,
                            control,
                            &row,
                            &fields,
                            &index_of,
                            physical,
                            restriction,
                            &mut esc_handled,
                        );
                    },
                );
            }
        });

    // Esc outside a focused bound control reverts the record: Access
    // commits the control on focus loss, so the first Esc then applies to
    // the record (support: "pressing the Esc key will consequently undo
    // the record").
    if !esc_handled && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        app.revert_form_record(tab.id);
    }

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(t("< Prev")).clicked() && record > 0 && app.flush_form_edits(tab.id) {
            app.set_form_record(tab.id, record - 1);
        }
        ui.label(tr(
            "Record {current} of {total}",
            &[
                ("current", &(record + 1).to_string()),
                ("total", &row_count.to_string()),
            ],
        ));
        if ui.button(t("Next >")).clicked()
            && record + 1 < row_count
            && app.flush_form_edits(tab.id)
        {
            app.set_form_record(tab.id, record + 1);
        }
    });
}

/// One control at runtime (bound fields show the record's value).
#[allow(clippy::too_many_arguments)]
fn render_control(
    ui: &mut egui::Ui,
    app: &mut App,
    tab_id: TabId,
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
            ui.strong(text);
        }
        ControlKind::TextBox { bound_field } => {
            let col = bound_field.as_deref().and_then(index_of);
            let value = col
                .map(|i| value_label(app.form_pending_value(tab_id, i).unwrap_or(&row[i])))
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
            let response = if restriction.is_some() {
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
                .map(|i| value_label(app.form_pending_value(tab_id, i).unwrap_or(&row[i])))
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
        ControlKind::Button { caption } => {
            let _ = ui.button(caption);
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
fn text_box_ui(
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
fn combo_box_ui(
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
    let editable = restriction.is_none();
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
                ui.menu_button("v", |ui| {
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
                })
            })
            .inner;
        let arrow = menu
            .response
            .accessible_name(format!("{} values", control.name));
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

/// Parses and stages one typed value; a failure keeps the draft visible
/// and reports precisely.
fn commit_text(
    app: &mut App,
    tab_id: TabId,
    control: &ControlDef,
    field: &FieldDef,
    col: usize,
    record: usize,
    text: &str,
) {
    match datacraft_core::row_edit::parse_field_value(field, text) {
        Ok(value) => {
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
        }
        Err(reason) => {
            app.last_error = Some(tr(
                "Control {control}: {reason}",
                &[("control", &control.name), ("reason", &reason)],
            ));
            app.last_notice = None;
            // The draft stays: the typed text remains visible.
        }
    }
}
