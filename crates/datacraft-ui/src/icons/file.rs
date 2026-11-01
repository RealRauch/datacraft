use super::{line, polyline, rect_stroke};
use eframe::egui;
use eframe::egui::{Color32, Rect, Stroke};

/// Open database: folder with an accent base.
pub(super) fn open_database(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (0.08, 0.80),
            (0.08, 0.26),
            (0.34, 0.26),
            (0.42, 0.36),
            (0.92, 0.36),
            (0.92, 0.80),
        ],
        stroke,
        true,
    );
    line(
        painter,
        rect,
        (0.12, 0.80),
        (0.88, 0.80),
        Stroke::new(2.0, accent),
    );
}

/// Close database: a cylinder with an accent cross.
pub(super) fn close_database(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (0.16, 0.30),
            (0.30, 0.22),
            (0.70, 0.22),
            (0.84, 0.30),
            (0.70, 0.38),
            (0.30, 0.38),
        ],
        stroke,
        true,
    );
    line(painter, rect, (0.16, 0.30), (0.16, 0.72), stroke);
    line(painter, rect, (0.84, 0.30), (0.84, 0.72), stroke);
    polyline(
        painter,
        rect,
        &[(0.16, 0.72), (0.30, 0.80), (0.70, 0.80), (0.84, 0.72)],
        stroke,
        false,
    );
    let accent = Stroke::new(1.8, accent);
    line(painter, rect, (0.60, 0.50), (0.78, 0.68), accent);
    line(painter, rect, (0.78, 0.50), (0.60, 0.68), accent);
}

/// Save design: a floppy disk with an accent label slot.
pub(super) fn save_design(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (0.12, 0.12),
            (0.74, 0.12),
            (0.88, 0.26),
            (0.88, 0.88),
            (0.12, 0.88),
        ],
        stroke,
        true,
    );
    rect_stroke(
        painter,
        Rect::from_min_max(super::pt(rect, 0.28, 0.12), super::pt(rect, 0.66, 0.36)),
        stroke,
    );
    painter.rect_filled(
        Rect::from_min_max(super::pt(rect, 0.30, 0.56), super::pt(rect, 0.70, 0.88)),
        1,
        accent,
    );
}

/// Load design: folder with a downward accent arrow.
pub(super) fn load_design(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[
            (0.06, 0.82),
            (0.06, 0.30),
            (0.32, 0.30),
            (0.40, 0.40),
            (0.94, 0.40),
            (0.94, 0.82),
        ],
        stroke,
        true,
    );
    let accent = Stroke::new(1.8, accent);
    line(painter, rect, (0.50, 0.48), (0.50, 0.78), accent);
    polyline(
        painter,
        rect,
        &[(0.38, 0.66), (0.50, 0.78), (0.62, 0.66)],
        accent,
        false,
    );
}

/// Navigation pane: a window with an accent sidebar.
pub(super) fn nav_pane(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let inner = Rect::from_min_max(super::pt(rect, 0.08, 0.16), super::pt(rect, 0.92, 0.84));
    painter.rect_filled(
        Rect::from_min_max(super::pt(rect, 0.08, 0.16), super::pt(rect, 0.38, 0.84)),
        1,
        accent,
    );
    rect_stroke(painter, inner, Stroke::new(1.2, ink));
    line(
        painter,
        rect,
        (0.38, 0.16),
        (0.38, 0.84),
        Stroke::new(1.2, ink),
    );
}

/// Import file: a tray with an accent download arrow.
pub(super) fn import_file(painter: &egui::Painter, rect: Rect, ink: Color32, accent: Color32) {
    let stroke = Stroke::new(1.2, ink);
    polyline(
        painter,
        rect,
        &[(0.12, 0.58), (0.12, 0.86), (0.88, 0.86), (0.88, 0.58)],
        stroke,
        false,
    );
    line(painter, rect, (0.50, 0.10), (0.50, 0.58), stroke);
    let accent = Stroke::new(1.8, accent);
    polyline(
        painter,
        rect,
        &[(0.36, 0.44), (0.50, 0.58), (0.64, 0.44)],
        accent,
        false,
    );
}
