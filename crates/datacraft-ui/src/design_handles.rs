//! Resize handles of the selected designer control (phase 102, step
//! 102.2; item M133).
//!
//! Paint and gesture glue only: the eight handle positions and the resize
//! math live in core `canvas`; a finished gesture becomes exactly one
//! `SetFormControlRect`/`SetReportControlRect` dispatch at the designer's
//! call site. The handle squares are painted above the control and take
//! the pointer before the control's own move gesture.

use eframe::egui;

use datacraft_core::canvas::{self, HANDLE_SIZE, Rect, ResizeHandle};
use datacraft_core::form::ControlDef;
use datacraft_core::ids::TabId;

use crate::app::App;

/// Paints the handles of the selected control and returns the new design
/// rect once a handle drag finishes. The parameter list mirrors the
/// designer's canvas state (control, canvas, grid, bounds, paint and
/// gesture seams); grouping it into a struct would gain nothing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resize_handles_ui(
    app: &mut App,
    tab_id: TabId,
    control: &ControlDef,
    canvas: egui::Rect,
    grid: f32,
    bounds: (f32, f32),
    painter: &egui::Painter,
    ui: &mut egui::Ui,
) -> Option<Rect> {
    let accent = crate::theme::tokens(ui.visuals()).accent;
    let square_size = egui::vec2(HANDLE_SIZE, HANDLE_SIZE);
    let mut finished = None;
    for (handle, (x, y)) in canvas::handle_centers(control.rect) {
        let square = egui::Rect::from_center_size(canvas.min + egui::vec2(x, y), square_size);
        painter.rect_filled(square, 1.0, accent);
        painter.rect_stroke(
            square,
            1.0,
            egui::Stroke::new(1.0, ui.visuals().extreme_bg_color),
            egui::StrokeKind::Inside,
        );
        crate::layout_probe::record(
            &format!("design.handle.{}.{}", control.name, handle.id()),
            square,
        );
        let key = format!("{}#{}", control.name, handle.id());
        let response = ui
            .allocate_rect(square.expand(1.0), egui::Sense::drag())
            .on_hover_cursor(cursor_for(handle));
        if response.dragged() {
            let delta = response.drag_delta();
            app.accumulate_drag(tab_id, &key, delta.x, delta.y);
        }
        if response.drag_stopped()
            && let Some((dx, dy)) = app.finish_drag(tab_id, &key)
        {
            finished = Some(canvas::resize_rect(
                control.rect,
                handle,
                dx,
                dy,
                grid,
                bounds,
            ));
        }
    }
    finished
}

/// The resize cursor of one handle direction.
fn cursor_for(handle: ResizeHandle) -> egui::CursorIcon {
    match handle {
        ResizeHandle::NorthWest | ResizeHandle::SouthEast => egui::CursorIcon::ResizeNwSe,
        ResizeHandle::NorthEast | ResizeHandle::SouthWest => egui::CursorIcon::ResizeNeSw,
        ResizeHandle::North | ResizeHandle::South => egui::CursorIcon::ResizeVertical,
        ResizeHandle::West | ResizeHandle::East => egui::CursorIcon::ResizeHorizontal,
    }
}
