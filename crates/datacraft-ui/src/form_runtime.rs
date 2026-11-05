//! Form runtime view (step 8.3, item H30).
//!
//! Renders bound control values for the current record; navigation is
//! per-tab UI state, values come from the per-tab row cache (the source
//! table's rows). Read-only — bound-field edits land with the Jet update
//! path (step 11.10, BACKLOG H40); the view says so instead of pretending.

use eframe::egui;

use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::schema::FieldValue;
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
    let field_names: Vec<String> = snapshot
        .database
        .as_ref()
        .and_then(|db| db.tables.iter().find(|t| t.name == form.source))
        .map(|t| t.fields.iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default();
    let index_of = |name: &str| {
        field_names
            .iter()
            .position(|f| f.eq_ignore_ascii_case(name))
    };
    let Some(rows) = app.rows_for(tab.id) else {
        ui.label(t("Loading rows…"));
        return;
    };
    if rows.is_empty() {
        ui.centered_and_justified(|ui| ui.weak(t("The source table has no rows.")));
        return;
    }

    let row_count = rows.len();
    let record = app.form_record(tab.id).min(row_count - 1);
    let row = rows[record].clone();

    egui::ScrollArea::vertical()
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
                        render_control(&mut control_ui, control, &row, &index_of);
                    },
                );
            }
        });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(t("< Prev")).clicked() && record > 0 {
            app.set_form_record(tab.id, record - 1);
        }
        ui.label(tr(
            "Record {current} of {total}",
            &[
                ("current", &(record + 1).to_string()),
                ("total", &row_count.to_string()),
            ],
        ));
        if ui.button(t("Next >")).clicked() && record + 1 < row_count {
            app.set_form_record(tab.id, record + 1);
        }
    });
    ui.label(
        egui::RichText::new(t(
            "Read-only view — bound-field edits land with the Jet update path (step 11.10, BACKLOG H40).",
        ))
        .small(),
    );
}

/// One control at runtime (bound fields show the record's value).
fn render_control(
    ui: &mut egui::Ui,
    control: &ControlDef,
    row: &[FieldValue],
    index_of: &dyn Fn(&str) -> Option<usize>,
) {
    match &control.kind {
        ControlKind::Label { text } => {
            ui.strong(text);
        }
        ControlKind::TextBox { bound_field } => {
            let value = bound_field
                .as_deref()
                .and_then(index_of)
                .map(|i| value_label(&row[i]))
                .unwrap_or_default();
            // The stored rect's size (step 46.1, M57) — the designed
            // box, not the old fixed 200x22.
            ui.add_sized(
                [ui.max_rect().width(), ui.max_rect().height()],
                egui::TextEdit::singleline(&mut value.clone()).interactive(false),
            );
        }
        ControlKind::CheckBox { bound_field } => {
            let checked = bound_field
                .as_deref()
                .and_then(index_of)
                .is_some_and(|i| matches!(&row[i], FieldValue::Boolean(true)));
            let mut shown = checked;
            ui.add(egui::Checkbox::new(&mut shown, ""));
        }
        ControlKind::ComboBox { bound_field } => {
            let value = bound_field
                .as_deref()
                .and_then(index_of)
                .map(|i| value_label(&row[i]))
                .unwrap_or_default();
            ui.weak(value);
        }
        ControlKind::Button { caption } => {
            let _ = ui.button(caption); // actions land with the write path (H9)
        }
        ControlKind::SubForm { form } => {
            ui.weak(tr("[subform: {form}]", &[("form", form)]));
        }
    }
}
