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
use crate::i18n::{t, tr};

/// Renders the VBA module tab of the workspace.
pub fn vba_viewer_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.weak(t("No database open"));
        return;
    };
    let Some(project) = db.vba.clone() else {
        match db.vba_error.as_deref() {
            Some(reason) => {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    tr("VBA extraction failed: {reason}", &[("reason", reason)]),
                );
            }
            None => {
                ui.weak(t("The open database has no VBA project."));
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
            ui.label(egui::RichText::new(t("VBA modules")).strong());
            ui.add(
                egui::TextEdit::singleline(&mut app.vba_search)
                    .hint_text(t("Search modules and source")),
            );
            let needle = app.vba_search.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for module in &project.modules {
                    if !needle.is_empty() && !module.name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let label = tr(
                        "{name}  ({kind}, {lines} lines)",
                        &[
                            ("name", &module.name),
                            ("kind", kind_label(module.kind)),
                            ("lines", &module.lines.to_string()),
                        ],
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
        ui.weak(t("The selected module is no longer part of the project."));
        return;
    };
    let source = module.source.clone();
    let kind = module.kind;
    let lines = module.lines();
    ui.horizontal(|ui| {
        ui.heading(&active);
        ui.label(tr(
            "{kind} — {lines} lines",
            &[("kind", kind_label(kind)), ("lines", &lines.to_string())],
        ));
        ui.weak(t("read-only; VBA is never executed"));
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
            ui.weak(t("No source line matches the search."));
        }
    });
}

fn kind_label(kind: VbaModuleKind) -> &'static str {
    match kind {
        VbaModuleKind::Standard => t("standard"),
        VbaModuleKind::Class => t("class"),
        VbaModuleKind::Form => t("form"),
        VbaModuleKind::Report => t("report"),
    }
}
