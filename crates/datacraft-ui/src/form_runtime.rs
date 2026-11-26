//! Form runtime view (step 8.3, item H30; phase 64, items M68/M70; ADR-8).
//!
//! Renders bound control values for the current record. Typing edits a
//! per-record buffer (Enter or focus loss stages the control value; the
//! record is written with exactly one `UpdateRow` when it is left). `Esc`
//! reverts the field first, the whole record on the second press.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, FormFilter};
use datacraft_core::ids::TabId;
use datacraft_core::schema::FieldDef;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};
mod control;

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
    // Both axes scroll (step 91.10, item M122): the fixed 640-wide
    // canvas stays reachable at narrow widths.
    let runtime_scroll = egui::ScrollArea::both()
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
            crate::layout_probe::record("form.runtime.canvas", canvas);
            let painter = ui.painter_at(canvas);
            painter.rect_filled(canvas, 0.0, ui.visuals().extreme_bg_color);
            // A hidden control renders nothing (phase 102, item M133).
            for control in form
                .controls
                .iter()
                .filter(|control| control.properties.visible)
            {
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
                        control::render_control(
                            &mut control_ui,
                            app,
                            tab.id,
                            &form.name,
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
    // The viewport rect rides the layout probe (step 91.10, item M122).
    crate::layout_probe::record("form.runtime.viewport", runtime_scroll.inner_rect);

    // Esc outside a focused bound control reverts the record: Access
    // commits the control on focus loss, so the first Esc then applies to
    // the record (support: "pressing the Esc key will consequently undo
    // the record").
    if !esc_handled && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        app.revert_form_record(tab.id);
    }

    crate::form_nav::record_nav_ui(app, tab, &form.source, record, row_count, ui);
}

/// Parses and stages one typed value; a failure keeps the draft visible
/// and reports precisely.
pub(super) fn commit_text(
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
            app.record_error(tr(
                "Control {control}: {reason}",
                &[("control", &control.name), ("reason", &reason)],
            ));
            // The draft stays: the typed text remains visible.
        }
    }
}
