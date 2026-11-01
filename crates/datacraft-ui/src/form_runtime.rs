//! Form runtime view (step 8.3, item H30).
//!
//! Renders bound control values for the current record; navigation is
//! per-tab UI state, values come from the per-tab row cache (the source
//! table's rows). Read-only — the write path lands with BACKLOG H9 and
//! the view says so instead of pretending.

use eframe::egui;

use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::present::display_value;
use datacraft_core::schema::FieldValue;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;

/// Renders the runtime view of one form tab.
pub fn form_runtime_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let snapshot = app.snapshot().clone();
    let Some(form) = snapshot
        .database
        .as_ref()
        .and_then(|db| db.forms.iter().find(|f| f.name == tab.object_name))
        .cloned()
    else {
        ui.label("Form no longer exists");
        return;
    };
    if form.source.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak("This form has no source table yet — assign one in the designer.")
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
        ui.label("Loading rows…");
        return;
    };
    if rows.is_empty() {
        ui.centered_and_justified(|ui| ui.weak("The source table has no rows."));
        return;
    }

    let row_count = rows.len();
    let record = app.form_record(tab.id).min(row_count - 1);
    let row = rows[record].clone();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for control in &form.controls {
                ui.push_id(
                    format!(
                        "runtime.{}.control.{}",
                        form.name.to_lowercase(),
                        control.name.to_lowercase()
                    ),
                    |ui| {
                        ui.horizontal(|ui| {
                            render_control(ui, control, &row, &index_of);
                        });
                    },
                );
            }
        });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("< Prev").clicked() && record > 0 {
            app.set_form_record(tab.id, record - 1);
        }
        ui.label(format!("Record {} of {}", record + 1, row_count));
        if ui.button("Next >").clicked() && record + 1 < row_count {
            app.set_form_record(tab.id, record + 1);
        }
    });
    ui.label(
        egui::RichText::new("Read-only view — edits land with the write path (BACKLOG H9).")
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
                .map(|i| display_value(&row[i]))
                .unwrap_or_default();
            ui.add_sized(
                [200.0, 22.0],
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
                .map(|i| display_value(&row[i]))
                .unwrap_or_default();
            ui.weak(value);
        }
        ControlKind::Button { caption } => {
            let _ = ui.button(caption); // actions land with the write path (H9)
        }
        ControlKind::SubForm { form } => {
            ui.weak(format!("[subform: {form}]"));
        }
    }
}
