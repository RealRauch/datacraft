//! The grid's header band (step 6.2; freeze actions step 40.2, item M39).
//!
//! Painted over one viewport-bounded band: the cells sit at computed
//! positions — frozen ones pinned, scrolled ones offset against the
//! body's mirrored offset. The context menu carries the sort and freeze
//! affordances. Split from `grid.rs` for the file-size gate.

use eframe::egui;

use datacraft_core::grid::{
    DEFAULT_COLUMN_WIDTH, GridMetrics, frozen_column_x, frozen_width, resize_column,
    scrolled_column_x,
};
use datacraft_core::qbe::SortOrder;

use super::grid::SELECTOR_WIDTH;
use crate::i18n::t;
use crate::theme::tokens;

/// Width of a column-separator drag handle.
const HANDLE_WIDTH: f32 = 5.0;

/// The header's scroll mirror and its callbacks (bundled to keep the
/// paint function's argument count at the lint limit): `scroll_x` is the
/// body's horizontal offset mirrored one frame later, `frozen` the
/// effective frozen-column count, `band_width` the viewport-bounded band
/// width (the band must never widen its container — that would defeat the
/// body's horizontal scroll); `sort` writes the QBE sort cell, `freeze`
/// sets the frozen count (0 = unfreeze all).
pub(super) struct HeaderPass<'a> {
    pub scroll_x: f32,
    pub frozen: usize,
    pub band_width: f32,
    pub sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    pub freeze: Option<&'a mut dyn FnMut(usize)>,
    /// Column filter affordance (step 45.1, M55): opens the column's
    /// criteria dialog (the in-menu editor died on click — egui menus
    /// do not host text editing).
    pub filter_dialog: Option<&'a mut dyn FnMut(usize)>,
}

/// Header band: accent fill, column names with sort indicators, resize
/// handles on every right edge, the sort affordances and the freeze
/// actions (header context menu). Painted over one full-width band; the
/// cells sit at computed positions — frozen ones pinned, scrolled ones
/// offset against `pass.scroll_x` (the body's offset, mirrored one frame
/// later).
pub(super) fn header_ui(
    ui: &mut egui::Ui,
    metrics: &GridMetrics,
    columns: &[String],
    widths: &mut [f32],
    sort_orders: &[SortOrder],
    pass: &mut HeaderPass<'_>,
) {
    let tokens = tokens(ui.visuals());
    let font = egui::TextStyle::Body.resolve(ui.style());
    let scroll_x = pass.scroll_x;
    let frozen = pass.frozen;
    let sortable = pass.sort.is_some();
    let frozen_total = frozen_width(widths, frozen);
    let band_h = metrics.header_height;
    let mut pending_sort: Option<(usize, SortOrder)> = None;
    let mut pending_freeze: Option<usize> = None;
    let mut pending_filter_dialog: Option<usize> = None;
    // The band is viewport-bounded, never content-sized: a content-sized
    // band would widen the container ui and defeat the body's horizontal
    // scroll. Cells beyond the band are painted clipped.
    let band_width = pass.band_width.max(SELECTOR_WIDTH);
    let (band, _) = ui.allocate_exact_size(egui::vec2(band_width, band_h), egui::Sense::hover());
    let origin = band.left_top();
    let sel_rect = egui::Rect::from_min_size(
        origin + egui::vec2(scroll_x, 0.0),
        egui::vec2(SELECTOR_WIDTH, band_h),
    );
    ui.painter().rect_filled(sel_rect, 0.0, tokens.accent_fill);
    for (col, name) in columns.iter().enumerate() {
        let width = widths.get(col).copied().unwrap_or(DEFAULT_COLUMN_WIDTH);
        let x = if col < frozen {
            SELECTOR_WIDTH + frozen_column_x(col, widths, 0.0)
        } else {
            SELECTOR_WIDTH + frozen_total + scrolled_column_x(col, widths, frozen, scroll_x)
        };
        let cell_width = (width - HANDLE_WIDTH).max(1.0);
        let cell_rect =
            egui::Rect::from_min_size(origin + egui::vec2(x, 0.0), egui::vec2(cell_width, band_h));
        // Click sense so the context menu (freeze actions) opens on every
        // grid; on sortable grids the click also toggles the sort.
        let sense = egui::Sense::click();
        let cell_id = ui.scope_id().with(("header", col));
        let response = ui.interact(cell_rect, cell_id, sense);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ColumnHeader, true, name));
        ui.painter().rect_filled(cell_rect, 0.0, tokens.accent_fill);
        let order = sort_orders.get(col).copied().unwrap_or_default();
        ui.painter().text(
            cell_rect.left_center() + egui::vec2(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            tokens.on_accent,
        );
        // Sort caret: painted, not a font glyph (Inter has no U+25B2).
        if order != SortOrder::None {
            let cx = cell_rect.right() - 9.0;
            let cy = cell_rect.center().y;
            let points = if order == SortOrder::Ascending {
                vec![
                    egui::pos2(cx, cy - 3.5),
                    egui::pos2(cx - 4.0, cy + 2.5),
                    egui::pos2(cx + 4.0, cy + 2.5),
                ]
            } else {
                vec![
                    egui::pos2(cx, cy + 3.5),
                    egui::pos2(cx - 4.0, cy - 2.5),
                    egui::pos2(cx + 4.0, cy - 2.5),
                ]
            };
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                tokens.on_accent,
                egui::Stroke::NONE,
            ));
        }
        response.context_menu(|ui| {
            if sortable {
                for target in [SortOrder::Ascending, SortOrder::Descending, SortOrder::None] {
                    let text = match target {
                        SortOrder::Ascending => t("Sort Ascending"),
                        SortOrder::Descending => t("Sort Descending"),
                        SortOrder::None => t("Clear Sort"),
                    };
                    if ui.selectable_label(order == target, text).clicked() {
                        pending_sort = Some((col, target));
                        ui.close();
                    }
                }
            }
            if pass.filter_dialog.is_some() {
                ui.separator();
                if ui.selectable_label(false, t("Filter…")).clicked() {
                    pending_filter_dialog = Some(col);
                    ui.close();
                }
            }
            if col >= frozen && ui.selectable_label(false, t("Freeze Fields")).clicked() {
                pending_freeze = Some(col + 1);
                ui.close();
            }
            if frozen > 0
                && ui
                    .selectable_label(false, t("Unfreeze All Fields"))
                    .clicked()
            {
                pending_freeze = Some(0);
                ui.close();
            }
        });
        let handle_rect = egui::Rect::from_min_size(
            origin + egui::vec2(x + cell_width, 0.0),
            egui::vec2(HANDLE_WIDTH, band_h),
        );
        let handle_id = ui.scope_id().with(("handle", col));
        let drag = ui.interact(handle_rect, handle_id, egui::Sense::drag());
        if drag.hovered() || drag.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if drag.dragged()
            && let Some(width) = widths.get_mut(col)
        {
            *width = resize_column(*width, drag.drag_delta().x);
        }
        ui.painter().line_segment(
            [handle_rect.left_top(), handle_rect.left_bottom()],
            egui::Stroke::new(1.0, tokens.border),
        );
    }
    if let Some((col, order)) = pending_sort
        && let Some(callback) = pass.sort.as_mut()
    {
        callback(col, order);
    }
    if let Some(count) = pending_freeze
        && let Some(callback) = pass.freeze.as_mut()
    {
        callback(count);
    }
    if let Some(column) = pending_filter_dialog
        && let Some(callback) = pass.filter_dialog.as_mut()
    {
        callback(column);
    }
}
