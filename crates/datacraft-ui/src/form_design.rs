//! Form design canvas (step 8.1, item H28).
//!
//! Renders the form frame and the snap grid from the snapshot; controls
//! arrive with step 8.2 and drag with the same snap math. Pure paint.

use eframe::egui;

use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;

/// Form canvas width in points.
const FORM_WIDTH: f32 = 640.0;
/// Form canvas height in points.
const FORM_HEIGHT: f32 = 400.0;

/// Renders the design canvas of one form tab.
pub fn form_design_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(form) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.forms.iter().find(|f| f.name == tab.object_name))
        .cloned()
    else {
        ui.label("Form no longer exists");
        return;
    };

    ui.heading(egui::RichText::new(format!("Form: {}", form.name)));
    ui.label(
        egui::RichText::new(format!(
            "Snap grid: {} px — controls arrive in step 8.2.",
            form.grid
        ))
        .small(),
    );
    ui.separator();

    let (canvas_rect, _) =
        ui.allocate_exact_size(egui::vec2(FORM_WIDTH, FORM_HEIGHT), egui::Sense::hover());
    let painter = ui.painter_at(canvas_rect);

    // Form surface.
    painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(
        canvas_rect,
        0.0,
        ui.visuals().widgets.noninteractive.fg_stroke,
        egui::StrokeKind::Inside,
    );

    // Snap grid dots.
    let grid = form.grid;
    let mut x = grid;
    while x < FORM_WIDTH {
        let mut y = grid;
        while y < FORM_HEIGHT {
            painter.circle_filled(
                canvas_rect.min + egui::vec2(x, y),
                1.0,
                ui.visuals().weak_text_color(),
            );
            y += grid;
        }
        x += grid;
    }

    // Controls render here from step 8.2 on (tab order = paint order).
    ui.label(egui::RichText::new(format!("{} controls", form.controls.len())).small());
}
