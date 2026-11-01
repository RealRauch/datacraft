//! Read-only VBA source viewer (step 17.2, item L5).
//!
//! Module tree on the left, decoded source on the right, full-text search
//! over both. No execution path exists by design (ADR-2).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::snapshot::TabSnapshot;
use datacraft_core::vba::VbaModuleKind;
use datacraft_core::{ObjectId, ObjectKind};

use crate::app::App;

/// Renders the VBA module tab of the workspace.
pub fn vba_viewer_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.weak("No database open");
        return;
    };
    let Some(project) = db.vba.clone() else {
        match db.vba_error.as_deref() {
            Some(reason) => {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("VBA extraction failed: {reason}"),
                );
            }
            None => {
                ui.weak("The open database has no VBA project.");
            }
        }
        return;
    };
    let active = tab.object_name.clone();
    let mut open: Option<String> = None;
    egui::Panel::left("vba.module.list")
        .resizable(true)
        .default_size(260.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("VBA modules").strong());
            ui.add(
                egui::TextEdit::singleline(&mut app.vba_search)
                    .hint_text("Search modules and source"),
            );
            let needle = app.vba_search.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for module in &project.modules {
                    if !needle.is_empty() && !module.name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let label = format!(
                        "{}  ({}, {} lines)",
                        module.name,
                        kind_label(module.kind),
                        module.lines
                    );
                    if ui
                        .selectable_label(module.name.eq_ignore_ascii_case(&active), label)
                        .clicked()
                    {
                        open = Some(module.name.clone());
                    }
                }
            });
        });
    if let Some(name) = open {
        app.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::Module, name)));
        return;
    }

    let Some(module) = app.vba_module(&active) else {
        ui.weak("The selected module is no longer part of the project.");
        return;
    };
    let source = module.source.clone();
    let kind = module.kind;
    let lines = module.lines();
    ui.horizontal(|ui| {
        ui.heading(&active);
        ui.label(format!("{} — {lines} lines", kind_label(kind)));
        ui.weak("read-only; VBA is never executed");
    });
    ui.separator();
    let needle = app.vba_search.to_lowercase();
    egui::ScrollArea::both().show(ui, |ui| {
        if needle.is_empty() {
            ui.monospace(&source);
            return;
        }
        let mut shown = 0usize;
        for (index, line) in source.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                ui.horizontal(|ui| {
                    ui.weak(format!("{:>4}", index + 1));
                    ui.monospace(line);
                });
                shown += 1;
            }
        }
        if shown == 0 {
            ui.weak("No source line matches the search.");
        }
    });
}

fn kind_label(kind: VbaModuleKind) -> &'static str {
    match kind {
        VbaModuleKind::Standard => "standard",
        VbaModuleKind::Class => "class",
        VbaModuleKind::Form => "form",
        VbaModuleKind::Report => "report",
    }
}
