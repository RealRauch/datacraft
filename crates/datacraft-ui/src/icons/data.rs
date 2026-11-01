use super::{line, polyline, pt, rect_stroke};
use eframe::egui;
use eframe::egui::{Color32, Rect, Stroke};

/// Table: a grid with an accent header rule.
pub(super) fn table(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    let grid = Rect::from_min_max(pt(rect, 0.08, 0.18), pt(rect, 0.92, 0.86));
    rect_stroke(painter, grid, stroke);
    line(
        painter,
        rect,
        (0.08, 0.36),
        (0.92, 0.36),
        Stroke::new(1.6, accent),
    );
    for x in [0.38, 0.68] {
        line(painter, rect, (x, 0.36), (x, 0.86), stroke);
    }
}

/// Table design: the table grid plus an accent pencil.
pub(super) fn table_design(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    let grid = Rect::from_min_max(pt(rect, 0.06, 0.14), pt(rect, 0.68, 0.68));
    rect_stroke(painter, grid, stroke);
    line(painter, rect, (0.06, 0.30), (0.68, 0.30), stroke);
    line(painter, rect, (0.06, 0.46), (0.68, 0.46), stroke);
    line(painter, rect, (0.36, 0.30), (0.36, 0.68), stroke);
    let accent = Stroke::new(2.2, accent);
    line(painter, rect, (0.58, 0.94), (0.94, 0.58), accent);
    polyline(
        painter,
        rect,
        &[(0.55, 0.84), (0.66, 0.95), (0.68, 0.86)],
        Stroke::new(1.0, accent.color),
        true,
    );
}

/// Query: the QBE funnel with an accent neck.
pub(super) fn query(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    polyline(
        painter,
        rect,
        &[
            (0.10, 0.20),
            (0.90, 0.20),
            (0.58, 0.52),
            (0.58, 0.84),
            (0.42, 0.90),
            (0.42, 0.52),
        ],
        Stroke::new(1.2, ink),
        true,
    );
    line(
        painter,
        rect,
        (0.42, 0.68),
        (0.58, 0.68),
        Stroke::new(1.8, accent),
    );
}

/// SQL: the `&lt;/&gt;` chevrons with an accent slash.
pub(super) fn sql(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.4, ink);
    polyline(
        painter,
        rect,
        &[(0.30, 0.28), (0.10, 0.50), (0.30, 0.72)],
        stroke,
        false,
    );
    polyline(
        painter,
        rect,
        &[(0.70, 0.28), (0.90, 0.50), (0.70, 0.72)],
        stroke,
        false,
    );
    line(
        painter,
        rect,
        (0.58, 0.22),
        (0.42, 0.78),
        Stroke::new(1.6, accent),
    );
}

/// View switch: a datasheet grid beside a design frame, joined by an
/// accent rule (the Views split-button, step 21.1).
pub(super) fn view_mode(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    let sheet = Rect::from_min_max(pt(rect, 0.04, 0.20), pt(rect, 0.42, 0.80));
    rect_stroke(painter, sheet, stroke);
    line(painter, rect, (0.04, 0.38), (0.42, 0.38), stroke);
    line(painter, rect, (0.23, 0.38), (0.23, 0.80), stroke);
    let design = Rect::from_min_max(pt(rect, 0.58, 0.20), pt(rect, 0.96, 0.80));
    rect_stroke(painter, design, stroke);
    line(
        painter,
        rect,
        (0.58, 0.38),
        (0.96, 0.38),
        Stroke::new(1.8, accent),
    );
    line(
        painter,
        rect,
        (0.50, 0.50),
        (0.50, 0.50),
        Stroke::new(1.0, accent),
    );
    polyline(
        painter,
        rect,
        &[(0.46, 0.44), (0.52, 0.50), (0.46, 0.56)],
        Stroke::new(1.0, accent),
        false,
    );
}

/// Relationships: two linked tables.
pub(super) fn relationships(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    rect_stroke(
        painter,
        Rect::from_min_max(pt(rect, 0.04, 0.16), pt(rect, 0.42, 0.46)),
        stroke,
    );
    rect_stroke(
        painter,
        Rect::from_min_max(pt(rect, 0.58, 0.54), pt(rect, 0.96, 0.84)),
        stroke,
    );
    line(
        painter,
        rect,
        (0.42, 0.31),
        (0.58, 0.69),
        Stroke::new(1.8, accent),
    );
}
