//! Report design canvas (steps 10.4/11.6, item H31).
//!
//! Renders the report's page (print geometry) from the snapshot: the
//! paper surface, margin guides and the shared control canvas with the
//! proven snap/drag math, beside the designer palette. Pure paint; every
//! change is one dispatch.

use eframe::egui;

use datacraft_core::ObjectKind;
use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::report::ReportDef;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};

/// Renders the design view of one report tab: canvas plus palette.
pub fn report_design_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(report) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.reports.iter().find(|r| r.name == tab.object_name))
        .cloned()
    else {
        ui.label(t("Report no longer exists"));
        return;
    };

    ui.horizontal_top(|ui| {
        // Cap the canvas column at the page width: egui separators span
        // the full available width, and an uncapped column would push
        // the palette column past the viewport (found in step 41.4).
        ui.vertical(|ui| {
            ui.set_max_width(report.page.width);
            canvas_ui(app, tab, &report, ui);
        });
        ui.separator();
        ui.vertical(|ui| {
            crate::designer_palette::designer_palette_ui(app, ObjectKind::Report, &report.name, ui);
            ui.separator();
            grouping_picker(app, &report, ui);
        });
    });
}

/// The grouping picker (step 41.4, item M48): the record source's fields
/// plus a "no grouping" entry, dispatching exactly one
/// `SetReportGrouping` per change. The preview re-renders the bands from
/// the refreshed snapshot.
fn grouping_picker(app: &mut App, report: &ReportDef, ui: &mut egui::Ui) {
    let fields: Vec<String> = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&report.source))
                .map(|table| table.fields.iter().map(|f| f.name.clone()).collect())
        })
        .unwrap_or_default();
    let selected = report
        .group_by
        .clone()
        .unwrap_or_else(|| t("no grouping").to_owned());
    let mut chosen: Option<Option<String>> = None;
    egui::ComboBox::from_label(t("Grouping"))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(report.group_by.is_none(), t("no grouping"))
                .clicked()
            {
                chosen = Some(None);
            }
            for field in &fields {
                let is_current = report
                    .group_by
                    .as_ref()
                    .is_some_and(|current| current.eq_ignore_ascii_case(field));
                if ui.selectable_label(is_current, field).clicked() {
                    chosen = Some(Some(field.clone()));
                }
            }
        });
    if let Some(group_by) = chosen {
        app.dispatch(Command::SetReportGrouping {
            report: report.name.clone(),
            group_by,
        });
    }
}

/// The report's design canvas itself (paper, grid, controls, footer).
fn canvas_ui(app: &mut App, tab: &TabSnapshot, report: &ReportDef, ui: &mut egui::Ui) {
    ui.heading(egui::RichText::new(tr(
        "Report: {name}",
        &[("name", &report.name)],
    )));
    let mut geometry = tr(
        "Page {width} x {height} pt — margins {margin} pt — snap grid {grid} px",
        &[
            ("width", &report.page.width.to_string()),
            ("height", &report.page.height.to_string()),
            ("margin", &report.page.margin_left.to_string()),
            ("grid", &report.grid.to_string()),
        ],
    );
    if !report.source.is_empty() {
        geometry.push_str(&tr(" — source: {source}", &[("source", &report.source)]));
    }
    ui.label(egui::RichText::new(geometry).small());
    ui.separator();

    let page = egui::vec2(report.page.width, report.page.height);
    let (canvas_rect, _) = ui.allocate_exact_size(page, egui::Sense::hover());
    let painter = ui.painter_at(canvas_rect);

    // Paper surface with margin guides (print-oriented design view).
    painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(
        canvas_rect,
        0.0,
        ui.visuals().widgets.noninteractive.fg_stroke,
        egui::StrokeKind::Inside,
    );
    let printable = canvas_rect.shrink2(egui::vec2(
        report.page.margin_left + report.page.margin_right,
        report.page.margin_top + report.page.margin_bottom,
    ));
    painter.rect_stroke(
        printable,
        0.0,
        egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
        egui::StrokeKind::Outside,
    );

    // Snap grid dots inside the printable area.
    let grid = report.grid;
    let mut x = grid;
    while x < report.page.width {
        let mut y = grid;
        while y < report.page.height {
            painter.circle_filled(
                canvas_rect.min + egui::vec2(x, y),
                1.0,
                ui.visuals().weak_text_color(),
            );
            y += grid;
        }
        x += grid;
    }

    // Controls paint in design order (later = on top); drag accumulates
    // the delta and dispatches one snapped move per gesture.
    let mut selected: Option<String> = None;
    let mut moved: Option<(String, Rect)> = None;
    for control in &report.controls {
        let rect = egui::Rect::from_min_size(
            canvas_rect.min + egui::vec2(control.rect.x, control.rect.y),
            egui::vec2(control.rect.width, control.rect.height),
        );
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let is_selected = app.report_selection(tab.id).as_deref() == Some(control.name.as_str());
        ui.push_id(
            format!(
                "report.{}.control.{}",
                report.name.to_lowercase(),
                control.name.to_lowercase()
            ),
            |ui| {
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                crate::form_design::paint_control(ui, &painter, rect, control, stroke, is_selected);
                if response.clicked() {
                    selected = Some(control.name.clone());
                }
                if response.dragged() {
                    let delta = response.drag_delta();
                    app.accumulate_drag(tab.id, &control.name, delta.x, delta.y);
                }
                if response.drag_stopped()
                    && let Some((dx, dy)) = app.finish_drag(tab.id, &control.name)
                {
                    moved = Some((
                        control.name.clone(),
                        Rect {
                            x: control.rect.x + dx,
                            y: control.rect.y + dy,
                            width: control.rect.width,
                            height: control.rect.height,
                        },
                    ));
                }
            },
        );
    }
    if let Some(name) = selected {
        app.set_report_selection(tab.id, name);
    }
    if let Some((name, rect)) = moved {
        app.dispatch(Command::SetReportControlRect {
            report: report.name.clone(),
            control: name,
            rect,
        });
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(
                "{count} controls — printable {width} x {height} pt",
                &[
                    ("count", &report.controls.len().to_string()),
                    ("width", &report.page.printable_width().to_string()),
                    ("height", &report.page.printable_height().to_string()),
                ],
            ))
            .small(),
        );
        if let Some(name) = app.report_selection(tab.id)
            && ui
                .small_button(tr("Delete {name}", &[("name", &name)]))
                .clicked()
        {
            app.dispatch(Command::DeleteReportControl {
                report: report.name.clone(),
                control: name,
            });
        }
    });
}

/// Renders the print preview of one report tab (step 10.5): the paginated
/// pages from `Core::report_preview` with prev/next navigation. Pure paint.
pub fn report_preview_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let pages = app.preview_pages(tab.id);
    if pages.is_empty() {
        ui.label(t("Preview unavailable"));
        return;
    }
    let page_count = pages.len();
    let current = app.preview_page(tab.id).min(page_count - 1);
    let items = pages[current].items.clone();
    let band = match pages[current].band {
        datacraft_core::report_preview::Band::Detail => t("detail"),
        datacraft_core::report_preview::Band::GroupHeader => t("group header"),
        datacraft_core::report_preview::Band::GroupFooter => t("group footer"),
    };

    ui.horizontal(|ui| {
        if ui.small_button("\u{25c0}").clicked() && current > 0 {
            app.set_preview_page(tab.id, current - 1);
        }
        ui.label(egui::RichText::new(tr(
            "Page {current} of {total}",
            &[
                ("current", &(current + 1).to_string()),
                ("total", &page_count.to_string()),
            ],
        )));
        ui.weak(tr("({band} band)", &[("band", band)]));
        if ui.small_button("\u{25b6}").clicked() && current + 1 < page_count {
            app.set_preview_page(tab.id, current + 1);
        }
        ui.weak(t("(print preview)"));
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Preview scale: 0.9 keeps an A4 page inside typical window heights.
        let scale = 0.9;
        let size = egui::vec2(595.0 * scale, 842.0 * scale);
        let (canvas_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(
            canvas_rect,
            0.0,
            ui.visuals().widgets.noninteractive.fg_stroke,
            egui::StrokeKind::Inside,
        );
        let font = egui::TextStyle::Body.resolve(ui.style());
        for item in &items {
            let rect = egui::Rect::from_min_size(
                canvas_rect.min + egui::vec2(item.rect.x * scale, item.rect.y * scale),
                egui::vec2(item.rect.width * scale, item.rect.height * scale),
            );
            painter.text(
                rect.left_top() + egui::vec2(2.0, 2.0),
                egui::Align2::LEFT_TOP,
                &item.text,
                font.clone(),
                ui.visuals().text_color(),
            );
        }
    });
}
