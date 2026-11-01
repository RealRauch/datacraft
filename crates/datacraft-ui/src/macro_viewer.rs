//! Read-only macro viewer (step 17.1, item L4).
//!
//! Renders the snapshot's coarse macro inventory: the readable definition
//! arguments in storage order. No execution affordance exists by design
//! (ADR-2).

use eframe::egui;

use crate::app::App;

/// Renders the macro tab of the workspace.
pub fn macro_viewer_ui(
    app: &mut App,
    tab: &datacraft_core::snapshot::TabSnapshot,
    ui: &mut egui::Ui,
) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.weak("No database open");
        return;
    };
    let Some(macro_def) = db
        .macros
        .iter()
        .find(|macro_def| macro_def.name.eq_ignore_ascii_case(&tab.object_name))
    else {
        ui.weak("Macro is no longer available");
        return;
    };

    ui.horizontal(|ui| {
        ui.heading(&macro_def.name);
        ui.label(if macro_def.definition_decoded {
            "read-only inventory"
        } else {
            "name only — definition not decoded"
        });
    });
    ui.separator();
    if !macro_def.definition_decoded {
        ui.weak(
            "Jet 3 keeps macro definitions inside MSysAccessObjects; that compound-file \
             storage is not decoded yet, so only the catalog name is shown.",
        );
        return;
    }
    if macro_def.actions.is_empty() {
        ui.weak("The macro definition carries no readable argument records.");
        return;
    }
    ui.label("Definition arguments (coarse action list, storage order):");
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (index, action) in macro_def.actions.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.weak(format!("{}.", index + 1));
                ui.monospace(&action.argument);
            });
        }
    });
}
