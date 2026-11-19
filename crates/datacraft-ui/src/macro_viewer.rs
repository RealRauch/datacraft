//! Read-only macro viewer (step 17.1, item L4).
//!
//! Renders the snapshot's coarse macro inventory: the readable definition
//! arguments in storage order. No execution affordance exists by design
//! (ADR-2).

use eframe::egui;

use crate::app::App;
use crate::i18n::t;

/// Renders the macro tab of the workspace.
pub fn macro_viewer_ui(
    app: &mut App,
    tab: &datacraft_core::snapshot::TabSnapshot,
    ui: &mut egui::Ui,
) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.weak(t("No database open"));
        return;
    };
    let Some(macro_def) = db
        .macros
        .iter()
        .find(|macro_def| macro_def.name.eq_ignore_ascii_case(&tab.object_name))
    else {
        ui.weak(t("Macro is no longer available"));
        return;
    };

    ui.horizontal(|ui| {
        ui.heading(&macro_def.name);
        ui.label(if macro_def.definition_decoded {
            t("read-only inventory")
        } else {
            t("name only — definition not decoded")
        });
    });
    ui.separator();
    if !macro_def.definition_decoded {
        ui.weak(t(
            "Jet 3 keeps macro definitions inside MSysAccessObjects; that compound-file storage is not decoded yet, so only the catalog name is shown.",
        ));
        return;
    }
    if macro_def.actions.is_empty() {
        ui.weak(t(
            "The macro definition carries no readable argument records.",
        ));
        return;
    }
    ui.label(t(
        "Definition arguments (coarse action list, storage order):",
    ));
    // The number column width-pads to the widest index (step 91.9, item
    // L62): the monospace argument column keeps one left edge from ten
    // entries on; the argument rects ride the `macro.argument` probe.
    let number_width = macro_def.actions.len().to_string().len();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (index, action) in macro_def.actions.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(format!("{:>width$}.", index + 1, width = number_width));
                let argument = ui.monospace(&action.argument);
                crate::layout_probe::record("macro.argument", argument.rect);
            });
        }
    });
}
