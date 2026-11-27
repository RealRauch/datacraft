//! Migration-readiness window (phase 103, step 103.2; item M134).
//!
//! A read-only tool window following the SQL-window pattern: presence
//! comes from the snapshot, opening runs `AnalyzeApp` through the command
//! bus — the same command agents send — and every report section renders
//! as read-only evidence. No classification, resolution or execution
//! happens here.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::readiness::{AppReadiness, ObjectStatus, ReportObjectKind, UnsupportedArea};

use crate::app::App;
use crate::i18n::{t, tr};
use crate::theme::tokens;

/// Renders the floating readiness window (when open).
pub fn readiness_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().readiness_window_open {
        return;
    }
    // The analysis runs on first paint of an open window; Refresh re-runs.
    if app.readiness_report().is_none() && app.readiness_error().is_none() {
        app.run_readiness_analysis();
    }
    let Some(report) = app.readiness_report().cloned() else {
        let error = app.readiness_error().map(str::to_owned);
        window_frame(ui, |ui| {
            if let Some(reason) = error {
                ui.colored_label(tokens(ui.visuals()).danger, reason);
            }
        });
        return;
    };
    let mut refresh = false;
    let mut close = false;
    window_frame(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.small_button(t("Refresh")).clicked() {
                refresh = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(t("Close")).clicked() {
                    close = true;
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            section(ui, t("Effort summary"), |ui| effort_section(ui, &report));
            section(ui, t("Unsupported features"), |ui| {
                unsupported_section(ui, &report)
            });
            section(ui, t("VBA estimate"), |ui| vba_section(ui, &report));
            section(ui, t("Linked tables"), |ui| links_section(ui, &report));
            section(ui, t("Objects"), |ui| objects_section(ui, &report));
        });
    });
    if refresh {
        app.run_readiness_analysis();
    }
    if close {
        app.dispatch(Command::ToggleReadinessWindow);
    }
}

/// The shared window chrome: centered default position, fixed width.
fn window_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Window::new(t("Migration readiness"))
        .id(egui::Id::unique("datacraft.readiness".to_owned()))
        .default_width(560.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(560.0, 480.0),
        ))
        .show(ui, add);
}

/// One collapsed-by-default report section.
fn section(
    ui: &mut egui::Ui,
    heading: impl Into<egui::WidgetText>,
    add: impl FnOnce(&mut egui::Ui),
) {
    egui::CollapsingHeader::new(heading)
        .default_open(true)
        .show(ui, add);
}

/// The effort summary: the counts behind the per-object evidence.
fn effort_section(ui: &mut egui::Ui, report: &AppReadiness) {
    let effort = &report.effort;
    ui.label(tr(
        "{running} of {total} objects run",
        &[
            ("running", &effort.running.to_string()),
            ("total", &effort.objects_total.to_string()),
        ],
    ));
    ui.label(tr(
        "not covered: {n}",
        &[("n", &effort.not_covered.to_string())],
    ));
    summary_line(ui, t("Native designs to migrate"), effort.native_designs);
    summary_line(ui, t("Macros"), effort.macros);
    summary_line(ui, t("Linked tables"), effort.linked_tables);
    summary_line(ui, t("VBA modules"), effort.vba_modules);
    summary_line(ui, t("Native queries"), effort.native_queries);
    ui.label(tr(
        "{n} of {total} native queries are outside the executable subset",
        &[
            ("n", &effort.native_queries_not_executable.to_string()),
            ("total", &effort.native_queries.to_string()),
        ],
    ));
}

/// One label-value summary line.
fn summary_line(ui: &mut egui::Ui, label: &'static str, value: usize) {
    ui.label(format!("{label}: {value}"));
}

/// The cross-cutting unsupported features with their area labels.
fn unsupported_section(ui: &mut egui::Ui, report: &AppReadiness) {
    if report.unsupported.is_empty() {
        ui.weak(t("no unsupported features"));
        return;
    }
    for feature in &report.unsupported {
        ui.label(format!("{}: {}", area_label(feature.area), feature.reason));
    }
}

/// The VBA estimate: the candidate share and the per-module numbers.
fn vba_section(ui: &mut egui::Ui, report: &AppReadiness) {
    let Some(percent) = report.vba.transplicable_percent else {
        ui.weak(t("no VBA project"));
        return;
    };
    ui.label(tr(
        "{percent}% of {code_lines} code lines are candidates",
        &[
            ("percent", &percent.to_string()),
            ("code_lines", &report.vba.code_lines.to_string()),
        ],
    ));
    for module in &report.vba.modules {
        ui.label(tr(
            "{name}: {candidates} of {lines} candidate lines",
            &[
                ("name", &module.name),
                ("candidates", &module.candidate_lines.to_string()),
                ("lines", &module.code_lines.to_string()),
            ],
        ));
        let details: Vec<String> = [
            Some(format!("{}: {}", t("SQL literals"), module.sql_literals)),
            (!module.markers.is_empty()).then(|| {
                let markers = module
                    .markers
                    .iter()
                    .map(|hit| format!("{} ×{}", hit.marker, hit.count))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {markers}", t("markers"))
            }),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !details.is_empty() {
            ui.weak(details.join(" · "));
        }
    }
}

/// The linked-table inventory, verbatim from the report.
fn links_section(ui: &mut egui::Ui, report: &AppReadiness) {
    if report.links.is_empty() {
        ui.weak(t("no linked tables"));
        return;
    }
    for link in &report.links {
        let target = link
            .source
            .clone()
            .or_else(|| link.connect.clone())
            .unwrap_or_else(|| t("target unknown").to_owned());
        ui.label(format!("{} → {target}", link.name));
    }
}

/// The per-object list: name, kind, ring and status with the gap reason.
fn objects_section(ui: &mut egui::Ui, report: &AppReadiness) {
    for object in &report.objects {
        ui.horizontal(|ui| {
            ui.label(&object.name);
            ui.weak(kind_label(object.kind));
            ui.weak(format!("Ring {}", object.ring.number()));
            match &object.status {
                ObjectStatus::Runs => {
                    ui.weak(t("runs"));
                }
                ObjectStatus::NotCovered { .. } => {
                    ui.colored_label(tokens(ui.visuals()).danger, t("not covered"));
                }
            }
        });
        if let ObjectStatus::NotCovered { reason } = &object.status {
            ui.weak(reason);
        }
    }
}

/// Localized kind label.
fn kind_label(kind: ReportObjectKind) -> &'static str {
    match kind {
        ReportObjectKind::Table => t("table"),
        ReportObjectKind::LinkedTable => t("linked table"),
        ReportObjectKind::Query => t("query"),
        ReportObjectKind::NativeQuery => t("native query"),
        ReportObjectKind::View => t("view"),
        ReportObjectKind::NativeView => t("native view"),
        ReportObjectKind::Form => t("form"),
        ReportObjectKind::NativeForm => t("native form"),
        ReportObjectKind::Report => t("report"),
        ReportObjectKind::NativeReport => t("native report"),
        ReportObjectKind::Macro => t("macro"),
    }
}

/// Localized unsupported-area label.
fn area_label(area: UnsupportedArea) -> &'static str {
    match area {
        UnsupportedArea::MacroExecution => t("Macro execution"),
        UnsupportedArea::VbaExecution => t("VBA execution"),
        UnsupportedArea::LinkedTables => t("Linked tables"),
        UnsupportedArea::NativeDefinitions => t("Native definitions"),
        UnsupportedArea::DecodeError => t("Decode error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The localized kind labels cover every report kind.
    #[test]
    fn every_object_kind_has_a_label() {
        for kind in [
            ReportObjectKind::Table,
            ReportObjectKind::LinkedTable,
            ReportObjectKind::Query,
            ReportObjectKind::NativeQuery,
            ReportObjectKind::View,
            ReportObjectKind::NativeView,
            ReportObjectKind::Form,
            ReportObjectKind::NativeForm,
            ReportObjectKind::Report,
            ReportObjectKind::NativeReport,
            ReportObjectKind::Macro,
        ] {
            assert!(!kind_label(kind).is_empty(), "{kind:?} has no label");
        }
    }
}
