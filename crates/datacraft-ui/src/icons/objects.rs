use super::{line, pt, rect_stroke};
use eframe::egui;
use eframe::egui::{Color32, Rect, Stroke};

/// Form: a window with an accent header and two fields.
pub(super) fn form(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    let window = Rect::from_min_max(pt(rect, 0.08, 0.14), pt(rect, 0.92, 0.88));
    rect_stroke(painter, window, stroke);
    line(
        painter,
        rect,
        (0.08, 0.34),
        (0.92, 0.34),
        Stroke::new(1.8, accent),
    );
    line(painter, rect, (0.18, 0.52), (0.58, 0.52), stroke);
    line(painter, rect, (0.18, 0.70), (0.74, 0.70), stroke);
}

/// Report: a page with an accent rule and a small bar chart.
pub(super) fn report(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    let page = Rect::from_min_max(pt(rect, 0.14, 0.10), pt(rect, 0.86, 0.90));
    rect_stroke(painter, page, stroke);
    line(
        painter,
        rect,
        (0.22, 0.28),
        (0.78, 0.28),
        Stroke::new(1.8, accent),
    );
    line(painter, rect, (0.22, 0.42), (0.70, 0.42), stroke);
    for (x, top) in [(0.24, 0.78), (0.42, 0.68), (0.60, 0.74)] {
        painter.rect_filled(
            Rect::from_min_max(pt(rect, x, top), pt(rect, x + 0.10, 0.82)),
            0,
            accent,
        );
    }
}

/// Macro: a lightning bolt in accent ink.
pub(super) fn macro_object(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            pt(rect, 0.58, 0.06),
            pt(rect, 0.22, 0.56),
            pt(rect, 0.52, 0.56),
        ],
        accent,
        Stroke::new(1.0, ink),
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            pt(rect, 0.30, 0.94),
            pt(rect, 0.80, 0.40),
            pt(rect, 0.48, 0.40),
        ],
        accent,
        Stroke::new(1.0, ink),
    ));
}

/// Module: a code page with accent chevrons.
pub(super) fn module(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    rect_stroke(
        painter,
        Rect::from_min_max(pt(rect, 0.10, 0.10), pt(rect, 0.90, 0.90)),
        stroke,
    );
    let accent = Stroke::new(1.6, accent);
    line(painter, rect, (0.42, 0.34), (0.24, 0.50), accent);
    line(painter, rect, (0.24, 0.50), (0.42, 0.66), accent);
    line(painter, rect, (0.58, 0.34), (0.76, 0.50), accent);
    line(painter, rect, (0.76, 0.50), (0.58, 0.66), accent);
}
