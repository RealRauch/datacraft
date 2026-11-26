//! Report print preview (split from `report_design.rs` for the
//! file-size gate, phase 102): the paginated pages, the zoom control, the
//! scaled text and its unit tests.

use eframe::egui;

use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};

pub fn report_preview_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
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
        // Zoom (phase 102, step 102.4): a per-surface setting, persisted in
        // ui.json; the choice repaints this frame from the settings value.
        let zoom = app.preview_zoom();
        let mut chosen: Option<f32> = None;
        egui::ComboBox::from_label(t("Zoom"))
            .selected_text(zoom_label(zoom))
            .show_ui(ui, |ui| {
                for step in PREVIEW_ZOOM_STEPS {
                    let current = (zoom - step).abs() < 0.001;
                    if ui.selectable_label(current, zoom_label(step)).clicked() {
                        chosen = Some(step);
                    }
                }
            });
        if let Some(zoom) = chosen {
            app.set_preview_zoom(zoom);
        }
        ui.weak(t("(print preview)"));
        // PDF export (phase 97, step 97.3; item M128): the build flag gates
        // the affordance; the picker supplies the new file path.
        let available = app.snapshot().pdf_export;
        if !available {
            ui.weak(t("This build does not include the PDF writer."));
        }
        let export = ui.add_enabled(available, egui::Button::new(t("Export PDF…")));
        if export
            .on_hover_text(t("Export the report's print preview to a PDF file."))
            .clicked()
        {
            app.request_dialog(
                crate::dialogs::DialogRequest::SaveFile {
                    suggested: format!("{}.pdf", report.name),
                },
                crate::app::dialog_flow::DialogIntent::ExportReportPdf {
                    report: report.name.clone(),
                },
            );
        }
    });
    ui.separator();

    egui::ScrollArea::both().show(ui, |ui| {
        // The zoom comes from the persisted per-surface setting (phase
        // 102, step 102.4); both axes scroll so a magnified page stays
        // reachable.
        let scale = app.preview_zoom();
        // The page geometry follows the report's own PageSetup (step
        // 91.11, item M123) — a non-A4 setup previews on its own paper.
        let size = preview_page_size(report.page, scale);
        let (canvas_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        crate::layout_probe::record("report.preview.page", canvas_rect);
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(
            canvas_rect,
            0.0,
            ui.visuals().widgets.noninteractive.fg_stroke,
            egui::StrokeKind::Inside,
        );
        // The text scales with the page (step 91.11, item L63): glyphs
        // stay inside their scaled rects. Bold and alignment follow the
        // control's phase-102 properties, which core carried into the
        // item.
        let body = egui::TextStyle::Body.resolve(ui.style());
        for item in &items {
            let rect = egui::Rect::from_min_size(
                canvas_rect.min + egui::vec2(item.rect.x * scale, item.rect.y * scale),
                egui::vec2(item.rect.width * scale, item.rect.height * scale),
            );
            let font = preview_font(&body, scale, item.bold);
            let (pos, anchor) = preview_anchor(rect, item.align);
            painter.text(pos, anchor, &item.text, font, ui.visuals().text_color());
        }
    });
}

/// The scaled preview page size (step 91.11, item M123): the report's
/// own PageSetup at the preview scale — never a hardcoded paper.
fn preview_page_size(page: datacraft_core::report::PageSetup, scale: f32) -> egui::Vec2 {
    egui::vec2(page.width * scale, page.height * scale)
}

/// Zoom choices of the preview toolbar (phase 102, step 102.4).
pub(crate) const PREVIEW_ZOOM_STEPS: [f32; 7] = [0.5, 0.75, 0.9, 1.0, 1.25, 1.5, 2.0];

/// A zoom choice as its percent label (numbers only — no catalog key).
fn zoom_label(zoom: f32) -> String {
    format!("{} %", (zoom * 100.0).round() as i32)
}

/// The preview text font: the body rung scaled with the page (step 91.11,
/// item L63) in the requested weight (phase 102, M133).
fn preview_font(body: &egui::FontId, scale: f32, bold: bool) -> egui::FontId {
    let family = if bold {
        egui::FontFamily::Name(crate::theme::fonts::FAMILY_SEMIBOLD.into())
    } else {
        body.family.clone()
    };
    egui::FontId::new(body.size * scale, family)
}

/// The horizontal text anchor of one preview item (phase 102).
fn preview_anchor(
    rect: egui::Rect,
    align: datacraft_core::form::TextAlign,
) -> (egui::Pos2, egui::Align2) {
    use datacraft_core::form::TextAlign;
    let pad = egui::vec2(2.0, 2.0);
    match align {
        TextAlign::Left => (rect.left_top() + pad, egui::Align2::LEFT_TOP),
        TextAlign::Center => (
            rect.center_top() + egui::vec2(0.0, pad.y),
            egui::Align2::CENTER_TOP,
        ),
        TextAlign::Right => (
            rect.right_top() + egui::vec2(-pad.x, pad.y),
            egui::Align2::RIGHT_TOP,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-A4 PageSetup previews on its own paper (step 91.11, item
    /// M123): the size follows the setup, not a hardcoded A4.
    #[test]
    fn the_preview_size_follows_the_page_setup() {
        let letter = datacraft_core::report::PageSetup {
            width: 612.0,
            height: 792.0,
            margin_left: 36.0,
            margin_top: 36.0,
            margin_right: 36.0,
            margin_bottom: 36.0,
        };
        let size = preview_page_size(letter, 0.9);
        assert!((size.x - 612.0 * 0.9).abs() < 0.01, "{size:?}");
        assert!((size.y - 792.0 * 0.9).abs() < 0.01, "{size:?}");
    }

    /// The preview text scales with the page (step 91.11, item L63):
    /// the font is the body rung at the page scale.
    #[test]
    fn the_preview_font_scales_with_the_page() {
        let body = egui::FontId::new(14.0, egui::FontFamily::Proportional);
        let scaled = preview_font(&body, 0.9, false);
        assert!((scaled.size - 12.6).abs() < 0.01, "{scaled:?}");
        assert_eq!(scaled.family, body.family);
    }

    /// Bold preview text uses the SemiBold family at the same scaled size
    /// (phase 102).
    #[test]
    fn bold_preview_text_uses_the_semibold_family() {
        let body = egui::FontId::new(14.0, egui::FontFamily::Proportional);
        let bold = preview_font(&body, 0.9, true);
        assert!((bold.size - 12.6).abs() < 0.01, "{bold:?}");
        assert_eq!(
            bold.family,
            egui::FontFamily::Name(crate::theme::fonts::FAMILY_SEMIBOLD.into())
        );
    }
}
