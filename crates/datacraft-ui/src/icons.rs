//! In-code vector iconography (step 20.3, item U3; nav glyphs step 38.1,
//! item M40).
//!
//! One name-keyed painter dispatcher: `paint` draws an icon into a rect from
//! monochrome ink strokes plus a teal accent, and returns whether the name
//! is known. Keys are the existing stable ids — ribbon action ids,
//! `ObjectKind::slug()` values and the `NAV_GLYPHS` backstage/chrome set —
//! never invented labels. No image assets.
//!
//! Section painters live beside this file: `file` (open/close/save/load and
//! pane actions), `data` (tables, queries, SQL, relationships), `objects`
//! (forms, reports, macros, modules) and `nav` (backstage/chrome glyphs,
//! adapted from the sibling Crafting Apps — GridCraft/WordCraft, MIT OR
//! Apache-2.0; credited in `THIRD-PARTY-NOTICES.md`).

use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Stroke, StrokeKind};

use crate::theme::tokens;

mod data;
mod file;
mod nav;
mod objects;

#[cfg(test)]
mod tests;

/// Icon box in points (nav pane rows, document tabs, ribbon buttons).
pub const ICON_SIZE: f32 = 16.0;

/// The backstage/chrome nav glyph names (step 38.1, item M40), in nav order.
/// Backstage navigation and the shell render exactly these keys; the painters
/// adapt the sibling Crafting Apps' 16×16 grammar (see `nav`).
pub const NAV_GLYPHS: &[&str] = &[
    "back", "home", "new", "open", "info", "save", "save_as", "print", "export", "close",
    "account", "options", "database",
];

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
        "undo" => file::undo(painter, rect, ink, accent),
        "redo" => file::redo(painter, rect, ink, accent),
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
        "back" => nav::back(painter, rect, ink, accent),
        "home" => nav::home(painter, rect, ink, accent),
        "new" => nav::new_file(painter, rect, ink, accent),
        "open" => nav::open_folder(painter, rect, ink, accent),
        "info" => nav::info(painter, rect, ink, accent),
        "save" => nav::save(painter, rect, ink, accent),
        "save_as" => nav::save_as(painter, rect, ink, accent),
        "print" => nav::print(painter, rect, ink, accent),
        "export" => nav::export(painter, rect, ink, accent),
        "close" => nav::close(painter, rect, ink, accent),
        "account" => nav::account(painter, rect, ink, accent),
        "options" => nav::options(painter, rect, ink, accent),
        "database" => nav::database(painter, rect, ink, accent),
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

/// Strokes a sub-rectangle given in normalized coordinates and size fractions.
fn rect_stroke_at(
    painter: &egui::Painter,
    rect: Rect,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    stroke: Stroke,
) {
    let min = pt(rect, x, y);
    painter.rect_stroke(
        Rect::from_min_size(min, egui::vec2(rect.width() * w, rect.height() * h)),
        1,
        stroke,
        StrokeKind::Inside,
    );
}

/// Strokes a circle; `center` is normalized, `radius` scales with the rect side.
fn circle_stroke(
    painter: &egui::Painter,
    rect: Rect,
    center: (f32, f32),
    radius: f32,
    stroke: Stroke,
) {
    let side = rect.width().min(rect.height());
    painter.circle_stroke(pt(rect, center.0, center.1), radius * side, stroke);
}

/// Fills a circle; `center` is normalized, `radius` scales with the rect side.
fn circle_filled(
    painter: &egui::Painter,
    rect: Rect,
    center: (f32, f32),
    radius: f32,
    color: Color32,
) {
    let side = rect.width().min(rect.height());
    painter.circle_filled(pt(rect, center.0, center.1), radius * side, color);
}

/// Fills a convex polygon from normalized points.
fn poly_filled(painter: &egui::Painter, rect: Rect, points: &[(f32, f32)], fill: Color32) {
    let points: Vec<Pos2> = points.iter().map(|(x, y)| pt(rect, *x, *y)).collect();
    painter.add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));
}
