//! Table design view (step 6.3, item H24) — read-only.
//!
//! Renders the design grid (Field Name / Data Type / Caption) and the
//! selected field's property sheet from the snapshot. Design-time schema
//! editing is a separate workstream (BACKLOG M9); the view says so instead
//! of pretending.

use eframe::egui;

use datacraft_core::schema::FieldProperties;
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
        .and_then(|db| db.tables.iter().find(|t| t.ui_test_id == tab.ui_test_id))
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
    ui.label(
        egui::RichText::new(t(
            "Read-only view — design-time schema editing is not implemented yet (BACKLOG M9).",
        ))
        .small(),
    );
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
