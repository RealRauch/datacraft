//! Original vector iconography (step 20.3, item U3).
//!
//! One name-keyed painter dispatcher: `paint` draws an icon into a rect from
//! monochrome ink strokes plus a teal accent, and returns whether the name
//! is known. Keys are the existing stable ids — ribbon action ids and
//! `ObjectKind::slug()` values — never invented labels. No image assets.
//!
//! Section painters live beside this file: `file` (open/close/save/load and
//! pane actions), `data` (tables, queries, SQL, relationships) and
//! `objects` (forms, reports, macros, modules).

use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Stroke, StrokeKind};

use crate::theme::tokens;

mod data;
mod file;
mod objects;

#[cfg(test)]
mod tests;

/// Icon box in points (nav pane rows, document tabs, ribbon buttons).
pub const ICON_SIZE: f32 = 16.0;

/// Paints the icon named `name` into `rect`. Returns whether the name is
/// known; unknown names paint nothing.
pub fn paint(
    painter: &egui::Painter,
    rect: Rect,
    name: &str,
    ink: Color32,
    accent: Color32,
) -> bool {
    match name {
        "open_database" => file::open_database(painter, rect, ink, accent),
        "close_database" => file::close_database(painter, rect, ink, accent),
        "save_design" => file::save_design(painter, rect, ink, accent),
        "load_design" => file::load_design(painter, rect, ink, accent),
        "toggle_nav_pane" => file::nav_pane(painter, rect, ink, accent),
        "import_file" => file::import_file(painter, rect, ink, accent),
        "table" => data::table(painter, rect, ink, accent),
        "table_design" => data::table_design(painter, rect, ink, accent),
        "query_design" | "query" => data::query(painter, rect, ink, accent),
        "query_sql" => data::sql(painter, rect, ink, accent),
        "view_mode" => data::view_mode(painter, rect, ink, accent),
        "relationships" => data::relationships(painter, rect, ink, accent),
        "form_design" | "form" => objects::form(painter, rect, ink, accent),
        "report_design" | "report" => objects::report(painter, rect, ink, accent),
        "macro" => objects::macro_object(painter, rect, ink, accent),
        "module" => objects::module(painter, rect, ink, accent),
        _ => return false,
    }
    true
}

/// Paints a 16 px icon inside a UI row; `active` swaps ink to accent ink.
pub fn icon(ui: &mut egui::Ui, name: &str, active: bool) {
    let tokens = tokens(ui.visuals());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ICON_SIZE, ICON_SIZE), egui::Sense::hover());
    let ink = if active {
        tokens.accent_text
    } else {
        tokens.text_muted
    };
    paint(ui.painter(), rect, name, ink, tokens.accent);
}

fn pt(rect: Rect, x: f32, y: f32) -> Pos2 {
    rect.min + egui::vec2(rect.width() * x, rect.height() * y)
}

fn line(painter: &egui::Painter, rect: Rect, a: (f32, f32), b: (f32, f32), stroke: Stroke) {
    painter.line_segment([pt(rect, a.0, a.1), pt(rect, b.0, b.1)], stroke);
}

fn polyline(
    painter: &egui::Painter,
    rect: Rect,
    points: &[(f32, f32)],
    stroke: Stroke,
    closed: bool,
) {
    let mut points: Vec<Pos2> = points.iter().map(|(x, y)| pt(rect, *x, *y)).collect();
    if closed {
        points.push(points[0]);
    }
    painter.add(egui::Shape::line(points, stroke));
}

fn rect_stroke(painter: &egui::Painter, rect: Rect, stroke: Stroke) {
    painter.rect_stroke(rect, 1, stroke, StrokeKind::Inside);
}
