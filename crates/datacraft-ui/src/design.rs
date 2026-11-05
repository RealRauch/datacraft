//! Table design view (step 6.3, item H24; editing in step 24.3, item M9).
//!
//! Renders the design grid (Field Name / Data Type / Caption), the selected
//! field's property sheet and the design-time edit controls: add a field,
//! rename/drop the selected field and toggle its required property. Every
//! edit is exactly one command through the bus.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::schema::{FieldDef, FieldKind, FieldProperties};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::labels::field_kind_label;

/// Renders the read-only design grid of one table tab.
pub fn design_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(fields) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        })
        .map(|t| t.fields.clone())
    else {
        ui.label(t("Table no longer exists"));
        return;
    };

    let selected = app.design_selection(tab.id);
    let mut clicked: Option<usize> = None;

    // Header.
    ui.horizontal(|ui| {
        ui.add_sized(
            [220.0, 24.0],
            egui::Label::new(egui::RichText::new(t("Field Name")).strong()),
        );
        ui.add_sized(
            [200.0, 24.0],
            egui::Label::new(egui::RichText::new(t("Data Type")).strong()),
        );
        ui.add_sized(
            [160.0, 24.0],
            egui::Label::new(egui::RichText::new(t("Caption")).strong()),
        );
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, field) in fields.iter().enumerate() {
                ui.push_id(
                    format!(
                        "design.{}.field.{}",
                        tab.object_name.to_lowercase(),
                        field.name
                    ),
                    |ui| {
                        let is_selected = i == selected;
                        let row = ui.horizontal(|ui| {
                            ui.add_sized([220.0, 22.0], egui::Label::new(&field.name));
                            ui.add_sized(
                                [200.0, 22.0],
                                egui::Label::new(field_kind_label(&field.kind)),
                            );
                            ui.add_sized(
                                [160.0, 22.0],
                                egui::Label::new(
                                    field.properties.caption.clone().unwrap_or_default(),
                                ),
                            );
                        });
                        if row.response.interact(egui::Sense::click()).clicked() {
                            clicked = Some(i);
                        }
                        if is_selected {
                            ui.painter().rect_stroke(
                                row.response.rect,
                                2.0,
                                egui::Stroke::new(1.0, ui.visuals().selection.bg_fill),
                                egui::StrokeKind::Inside,
                            );
                        }
                    },
                );
            }
        });

    if let Some(i) = clicked {
        app.set_design_selection(tab.id, i);
    }
    ui.separator();

    // Property sheet of the selected field.
    if let Some(field) = fields.get(selected) {
        ui.heading(egui::RichText::new(tr(
            "Field Properties — {field}",
            &[("field", &field.name)],
        )));
        property_grid(ui, &field.properties);
    }
    ui.separator();

    // Design-time edits (step 24.3, M9): one command per action. The
    // block gates on the alter_table capability (step 42.2, L18) — a
    // restricted source shows the reason instead of dispatching into a
    // Failed toast; the dispatcher gate stays the second line.
    let mut intent: Option<Command> = None;
    let schema_restriction = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::DESIGN_ADD_FIELD,
    )
    .copied();
    if let Some(reason) = schema_restriction {
        ui.weak(crate::availability::reason_text(&reason));
    }
    ui.add_enabled_ui(schema_restriction.is_none(), |ui| {
        ui.horizontal(|ui| {
            let label = ui.label(t("New field"));
            let mut new_name = app.design_new_field(tab.id);
            if ui
                .text_edit_singleline(&mut new_name)
                .labelled_by(label.id)
                .changed()
            {
                app.set_design_new_field(tab.id, new_name.clone());
            }
            if ui.button(t("Add Field")).clicked() && !new_name.trim().is_empty() {
                intent = Some(Command::AddField {
                    table: tab.object_name.clone(),
                    field: FieldDef::new(new_name.trim(), FieldKind::Text { max_len: 50 }),
                });
                app.set_design_new_field(tab.id, String::new());
            }
        });
        if let Some(field) = fields.get(selected) {
            ui.horizontal(|ui| {
                let label = ui.label(t("Rename field"));
                let mut rename = app.design_rename(tab.id);
                if ui
                    .text_edit_singleline(&mut rename)
                    .labelled_by(label.id)
                    .changed()
                {
                    app.set_design_rename(tab.id, rename.clone());
                }
                if ui.button(t("Rename")).clicked() && !rename.trim().is_empty() {
                    intent = Some(Command::RenameField {
                        table: tab.object_name.clone(),
                        field: field.name.clone(),
                        new_name: rename.trim().to_owned(),
                    });
                    app.set_design_rename(tab.id, String::new());
                }
                if ui.button(t("Delete")).clicked() {
                    intent = Some(Command::DropField {
                        table: tab.object_name.clone(),
                        field: field.name.clone(),
                    });
                }
                let mut required = field.properties.required;
                if ui.checkbox(&mut required, t("Required")).changed() {
                    intent = Some(Command::SetFieldProperties {
                        table: tab.object_name.clone(),
                        field: field.name.clone(),
                        properties: FieldProperties {
                            required,
                            ..field.properties.clone()
                        },
                    });
                }
            });
        }
    });
    if let Some(command) = intent {
        app.dispatch(command);
    }
}

/// Two-column property sheet (name / value).
fn property_grid(ui: &mut egui::Ui, props: &FieldProperties) {
    egui::Grid::new("datacraft.design.properties")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            prop_row(ui, t("Caption"), props.caption.as_deref().unwrap_or(""));
            prop_row(
                ui,
                t("Default Value"),
                props.default_value.as_deref().unwrap_or(""),
            );
            prop_row(
                ui,
                t("Validation Rule"),
                props.validation_rule.as_deref().unwrap_or(""),
            );
            prop_row(
                ui,
                t("Validation Text"),
                props.validation_text.as_deref().unwrap_or(""),
            );
            prop_row(
                ui,
                t("Input Mask"),
                props.input_mask.as_deref().unwrap_or(""),
            );
            prop_row(ui, t("Format"), props.format.as_deref().unwrap_or(""));
            prop_row(
                ui,
                t("Required"),
                if props.required { t("Yes") } else { t("No") },
            );
            prop_row(
                ui,
                t("Allow Zero Length"),
                if props.allow_zero_length {
                    t("Yes")
                } else {
                    t("No")
                },
            );
            prop_row(ui, t("Indexed"), index_label(props));
        });
}

fn prop_row(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.weak(name);
    ui.label(value);
    ui.end_row();
}

fn index_label(props: &FieldProperties) -> &'static str {
    match props.indexed {
        datacraft_core::schema::IndexMode::None => t("No"),
        datacraft_core::schema::IndexMode::DuplicatesOk => t("Yes (Duplicates OK)"),
        datacraft_core::schema::IndexMode::NoDuplicates => t("Yes (No Duplicates)"),
    }
}
