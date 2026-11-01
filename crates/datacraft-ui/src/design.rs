//! Table design view (step 6.3, item H24) — read-only.
//!
//! Renders the design grid (Field Name / Data Type / Caption) and the
//! selected field's property sheet from the snapshot. Design-time schema
//! editing needs the write path (BACKLOG H9) and is intentionally absent;
//! the grid communicates that instead of pretending.

use eframe::egui;

use datacraft_core::schema::FieldProperties;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;

/// Renders the read-only design grid of one table tab.
pub fn design_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(fields) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.tables.iter().find(|t| t.ui_test_id == tab.ui_test_id))
        .map(|t| t.fields.clone())
    else {
        ui.label("Table no longer exists");
        return;
    };

    let selected = app.design_selection(tab.id);
    let mut clicked: Option<usize> = None;

    // Header.
    ui.horizontal(|ui| {
        ui.add_sized(
            [220.0, 24.0],
            egui::Label::new(egui::RichText::new("Field Name").strong()),
        );
        ui.add_sized(
            [200.0, 24.0],
            egui::Label::new(egui::RichText::new("Data Type").strong()),
        );
        ui.add_sized(
            [160.0, 24.0],
            egui::Label::new(egui::RichText::new("Caption").strong()),
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
                            ui.add_sized([200.0, 22.0], egui::Label::new(&field.kind_display));
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
        ui.heading(egui::RichText::new(format!("Field Properties — {}", field.name)).heading());
        property_grid(ui, &field.properties);
    }
    ui.separator();
    ui.label(
        egui::RichText::new(
            "Read-only view — design-time schema editing lands with the write path (BACKLOG H9).",
        )
        .small(),
    );
}

/// Two-column property sheet (name / value).
fn property_grid(ui: &mut egui::Ui, props: &FieldProperties) {
    egui::Grid::new("datacraft.design.properties")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            prop_row(ui, "Caption", props.caption.as_deref().unwrap_or(""));
            prop_row(
                ui,
                "Default Value",
                props.default_value.as_deref().unwrap_or(""),
            );
            prop_row(
                ui,
                "Validation Rule",
                props.validation_rule.as_deref().unwrap_or(""),
            );
            prop_row(
                ui,
                "Validation Text",
                props.validation_text.as_deref().unwrap_or(""),
            );
            prop_row(ui, "Input Mask", props.input_mask.as_deref().unwrap_or(""));
            prop_row(ui, "Format", props.format.as_deref().unwrap_or(""));
            prop_row(ui, "Required", if props.required { "Yes" } else { "No" });
            prop_row(
                ui,
                "Allow Zero Length",
                if props.allow_zero_length { "Yes" } else { "No" },
            );
            prop_row(ui, "Indexed", index_label(props));
        });
}

fn prop_row(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.weak(name);
    ui.label(value);
    ui.end_row();
}

fn index_label(props: &FieldProperties) -> &'static str {
    match props.indexed {
        datacraft_core::schema::IndexMode::None => "No",
        datacraft_core::schema::IndexMode::DuplicatesOk => "Yes (Duplicates OK)",
        datacraft_core::schema::IndexMode::NoDuplicates => "Yes (No Duplicates)",
    }
}
