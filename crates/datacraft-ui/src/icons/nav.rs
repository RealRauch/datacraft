//! Backstage/chrome nav glyphs (step 38.1, item M40).
//!
//! The painters adapt the sibling Crafting Apps' 16×16 in-code icon grammar
//! ([GridCraft](https://github.com/storytold/gridcraft) /
//! [WordCraft](https://github.com/storytold/wordcraft), MIT OR Apache-2.0) to
//! this dispatcher's normalized coordinates: `g()` divides a reference grid
//! unit by 16, so ported coordinates stay comparable with the upstream arms.
//! The set is monochrome ink; only `save_as` carries an accent part, matching
//! the rest of the family (compare `file::open_database`).

use super::{circle_filled, circle_stroke, line, poly_filled, polyline, rect_stroke_at};
use eframe::egui;
use eframe::egui::{Color32, Rect, Stroke};

/// Maps a 16×16 reference-grid coordinate to this dispatcher's 0..1 space.
fn g(v: f32) -> f32 {
    v / 16.0
}

/// Back: arrow-left in a ring (leaves the backstage).
pub(super) fn back(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    circle_stroke(painter, rect, (g(8.0), g(8.0)), g(6.5), stroke);
    line(painter, rect, (g(11.5), g(8.0)), (g(4.8), g(8.0)), stroke);
    polyline(
        painter,
        rect,
        &[(g(7.5), g(5.3)), (g(4.8), g(8.0)), (g(7.5), g(10.7))],
        stroke,
        false,
    );
}

/// Home: roof over walls.
pub(super) fn home(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[(g(2.0), g(8.0)), (g(8.0), g(2.5)), (g(14.0), g(8.0))],
        stroke,
        false,
    );
    polyline(
        painter,
        rect,
        &[
            (g(4.0), g(6.5)),
            (g(4.0), g(13.5)),
            (g(12.0), g(13.5)),
            (g(12.0), g(6.5)),
        ],
        stroke,
        false,
    );
}

/// New: a blank page with a folded corner.
pub(super) fn new_file(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    polyline(
        painter,
        rect,
        &[
            (g(9.5), g(1.5)),
            (g(3.5), g(1.5)),
            (g(3.5), g(14.5)),
            (g(12.5), g(14.5)),
            (g(12.5), g(4.5)),
            (g(9.5), g(1.5)),
            (g(9.5), g(4.5)),
            (g(12.5), g(4.5)),
        ],
        Stroke::new(1.2, ink),
        false,
    );
}

/// Open: a folder seen from the front, flap lifted.
pub(super) fn open_folder(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (g(13.0), g(6.5)),
            (g(13.0), g(4.5)),
            (g(7.5), g(4.5)),
            (g(6.0), g(3.0)),
            (g(1.5), g(3.0)),
            (g(1.5), g(13.0)),
        ],
        stroke,
        false,
    );
    polyline(
        painter,
        rect,
        &[
            (g(1.5), g(13.0)),
            (g(4.0), g(7.0)),
            (g(15.0), g(7.0)),
            (g(12.5), g(13.0)),
        ],
        stroke,
        true,
    );
}

/// Info: a ring with an i.
pub(super) fn info(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    circle_stroke(
        painter,
        rect,
        (g(8.0), g(8.0)),
        g(6.5),
        Stroke::new(1.2, ink),
    );
    line(
        painter,
        rect,
        (g(8.0), g(7.0)),
        (g(8.0), g(11.8)),
        Stroke::new(1.3, ink),
    );
    circle_filled(painter, rect, (g(8.0), g(4.8)), g(0.8), ink);
}

/// Save: the classic floppy — body, shutter, label.
pub(super) fn save(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    rect_stroke_at(painter, rect, g(2.5), g(2.5), g(11.0), g(11.0), stroke);
    rect_stroke_at(painter, rect, g(5.0), g(2.5), g(6.0), g(4.0), stroke);
    rect_stroke_at(painter, rect, g(4.5), g(9.0), g(7.0), g(4.5), stroke);
}

/// Save as: a compact floppy with an accent pen across the corner.
pub(super) fn save_as(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    rect_stroke_at(painter, rect, g(1.5), g(1.5), g(10.5), g(10.5), stroke);
    rect_stroke_at(painter, rect, g(3.5), g(1.5), g(5.0), g(3.0), stroke);
    rect_stroke_at(painter, rect, g(3.0), g(7.0), g(6.0), g(4.0), stroke);
    poly_filled(
        painter,
        rect,
        &[
            (g(8.0), g(14.5)),
            (g(8.75), g(12.5)),
            (g(12.75), g(8.5)),
            (g(14.0), g(9.75)),
            (g(10.0), g(13.75)),
        ],
        accent,
    );
    line(painter, rect, (g(8.0), g(14.5)), (g(8.75), g(12.5)), stroke);
}

/// Print: sheet feeder over a printer body over the output tray.
pub(super) fn print(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (g(4.5), g(5.5)),
            (g(4.5), g(1.5)),
            (g(11.5), g(1.5)),
            (g(11.5), g(5.5)),
        ],
        stroke,
        false,
    );
    polyline(
        painter,
        rect,
        &[
            (g(4.0), g(11.5)),
            (g(1.5), g(11.5)),
            (g(1.5), g(5.5)),
            (g(14.5), g(5.5)),
            (g(14.5), g(11.5)),
            (g(12.0), g(11.5)),
        ],
        stroke,
        false,
    );
    polyline(
        painter,
        rect,
        &[
            (g(4.5), g(9.0)),
            (g(4.5), g(14.5)),
            (g(11.5), g(14.5)),
            (g(11.5), g(9.0)),
        ],
        stroke,
        true,
    );
    circle_filled(painter, rect, (g(12.3), g(7.6)), g(0.6), ink);
}

/// Export: an arrow out of a tray (the Crafting Apps' share glyph).
pub(super) fn export(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (g(4.0), g(7.0)),
            (g(4.0), g(13.0)),
            (g(12.0), g(13.0)),
            (g(12.0), g(7.0)),
        ],
        stroke,
        false,
    );
    line(painter, rect, (g(8.0), g(10.0)), (g(8.0), g(2.5)), stroke);
    polyline(
        painter,
        rect,
        &[(g(5.5), g(5.0)), (g(8.0), g(2.5)), (g(10.5), g(5.0))],
        stroke,
        false,
    );
}

/// Close: a plain cross.
pub(super) fn close(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    let stroke = Stroke::new(1.3, ink);
    line(painter, rect, (g(4.0), g(4.0)), (g(12.0), g(12.0)), stroke);
    line(painter, rect, (g(12.0), g(4.0)), (g(4.0), g(12.0)), stroke);
}

/// Account: head over shoulders.
pub(super) fn account(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    circle_stroke(
        painter,
        rect,
        (g(8.0), g(5.5)),
        g(2.5),
        Stroke::new(1.2, ink),
    );
    let dome: Vec<(f32, f32)> = (0..=8)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 8.0;
            (g(8.0 + 4.0 * a.cos()), g(13.5 - 4.0 * a.sin()))
        })
        .collect();
    polyline(painter, rect, &dome, Stroke::new(1.2, ink), false);
}

/// Options: a ring around a dot (the Crafting Apps' settings glyph).
pub(super) fn options(painter: &egui::Painter, rect: Rect, ink: Color32, _accent: Color32) {
    circle_stroke(
        painter,
        rect,
        (g(8.0), g(8.0)),
        g(5.0),
        Stroke::new(1.2, ink),
    );
    circle_filled(painter, rect, (g(8.0), g(8.0)), g(2.0), ink);
}
