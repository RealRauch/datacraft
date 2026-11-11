//! The Import File window (step 58.1, item M50): the source path (native
//! picker through the dialog seam, step 38.3), the parsed preview (field
//! names, inferred kinds, the leading rows), a target table name and one
//! `ImportFile` dispatch — the ordinary write path does the rest. The
//! window closes over the bus when the import completes.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::import::ImportPreview;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::labels::field_kind_label;

/// Renders the Import File window, when open (the shell calls this beside
/// the other windows).
pub fn import_window_ui(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().import_window_open {
        return;
    }
    let mut open = true;
    egui::Window::new(t("Import File"))
        .id(egui::Id::unique("datacraft.import_window".to_owned()))
        .open(&mut open)
        .default_width(440.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(440.0, 360.0),
        ))
        .show(ui, |ui| import_window_body(app, ui));
    if !open {
        // The window's ✕ is a toggle like every other close path.
        app.dispatch(Command::ToggleImportWindow);
    }
}

fn import_window_body(app: &mut App, ui: &mut egui::Ui) {
    // The source row: path buffer plus the native picker through the
    // dialog seam (step 38.3, item M43) — picking dispatches the preview.
    let mut source = app.import_source().to_owned();
    ui.horizontal(|ui| {
        let label = ui.label(t("Source file"));
        let field = ui.add(egui::TextEdit::singleline(&mut source).desired_width(280.0));
        if field.labelled_by(label.id).changed() {
            app.set_import_source(source.clone());
        }
        if ui.button(t("Browse…")).clicked()
            && let Some(picked) = app.dialogs.pick_open_file()
        {
            app.set_import_source(picked.to_string_lossy().into_owned());
            app.dispatch(Command::PreviewImportFile { source: picked });
        }
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !source.trim().is_empty(),
                egui::Button::new(t("Load Preview")),
            )
            .clicked()
        {
            app.dispatch(Command::PreviewImportFile {
                source: app.import_source().into(),
            });
        }
    });

    // The preview: fields with the inferred kinds plus the leading rows.
    let preview = app.snapshot().import_preview.clone();
    let Some(preview) = preview else {
        ui.weak(t("No preview loaded."));
        return;
    };
    preview_ui(app, &preview, ui);
}

/// The parsed source: field kinds, the leading rows, the target table
/// name and the Import button (one `ImportFile` dispatch).
fn preview_ui(app: &mut App, preview: &ImportPreview, ui: &mut egui::Ui) {
    ui.separator();
    ui.label(tr(
        "{total} rows detected — showing the first {shown}",
        &[
            ("total", &preview.total_rows.to_string()),
            ("shown", &preview.rows.len().to_string()),
        ],
    ));
    egui::Grid::new("import.preview.fields")
        .striped(true)
        .show(ui, |ui| {
            for field in &preview.fields {
                ui.label(&field.name);
                ui.weak(field_kind_label(&field.kind));
                ui.end_row();
            }
        });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("import.preview.rows")
        .max_height(140.0)
        .show(ui, |ui| {
            egui::Grid::new("import.preview.rows.grid")
                .striped(true)
                .show(ui, |ui| {
                    for row in &preview.rows {
                        for cell in row {
                            ui.label(cell);
                        }
                        ui.end_row();
                    }
                });
        });
    ui.separator();

    // The target table name: the file stem is the hint and the fallback.
    let mut table = app.import_table().to_owned();
    ui.horizontal(|ui| {
        let label = ui.label(t("Target table"));
        let field = ui.add(egui::TextEdit::singleline(&mut table).hint_text(&preview.table));
        if field.labelled_by(label.id).changed() {
            app.set_import_table(table.clone());
        }
        if ui.button(t("Import")).clicked() {
            let name = if table.trim().is_empty() {
                preview.table.clone()
            } else {
                table.clone()
            };
            let events = app.dispatch(Command::ImportFile {
                source: app.import_source().into(),
                table: name,
            });
            if !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. }))
            {
                // The core closed the window; the buffers start empty for
                // the next import.
                app.set_import_source("");
                app.set_import_table("");
            }
        }
    });
    ui.weak(t("The source file is not modified."));
}
