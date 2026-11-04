//! The shared Access grid: teal header band, virtualized body, row
//! selector (step 6.1/6.2, items H22/H23; Access fidelity 12.5).
//!
//! Paint-only glue over the core grid math: virtual scrolling, resize
//! handles, sort affordances, keyboard record navigation (step 40.1,
//! item M38) and freeze columns (step 40.2, item M39) consume
//! `datacraft_core::grid`; the datasheet, the query results and the SQL
//! window render through [`grid_ui`], so all three behave identically.
//! Frozen leading columns and the row selector ride the scroll
//! compensation inside the scrolled content — no separate panes, one
//! scroll area, and every placement is a pure core function.

use eframe::egui;

use datacraft_core::availability::DisabledReason;
use datacraft_core::grid::{
    DEFAULT_COLUMN_WIDTH, GridMetrics, clamp_scroll_x, ensure_row_visible, frozen_column_x,
    frozen_width, resize_column, scrolled_column_x, visible_scrolled_range,
};
use datacraft_core::qbe::SortOrder;

use super::nav::{nav_target, read_delete_key, read_nav_intent};
use crate::i18n::t;
use crate::theme::tokens;

/// Width of the row-selector column (Access's record selector).
pub(super) const SELECTOR_WIDTH: f32 = 48.0;
/// Width of a column-separator drag handle.
const HANDLE_WIDTH: f32 = 5.0;

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller. `id_salt` names
/// the grid's focus anchor (keyboard navigation, step 40.1, item M38) and
/// must be unique per rendered grid; `frozen` is the effective count of
/// frozen leading columns (step 40.2, item M39); `delete_reason` restricts
/// the record-deletion affordance for the open source (step 40.3, M44).
pub(crate) struct Grid<'a> {
    pub metrics: GridMetrics,
    pub columns: &'a [String],
    pub body: &'a [Vec<String>],
    pub cursor: usize,
    pub widths: &'a mut [f32],
    pub sort_orders: &'a [SortOrder],
    pub id_salt: String,
    pub frozen: usize,
    pub delete_reason: Option<DisabledReason>,
}

/// The header's scroll mirror and its callbacks (bundled to keep the
/// paint function's argument count at the lint limit): `scroll_x` is the
/// body's horizontal offset mirrored one frame later, `frozen` the
/// effective frozen-column count, `band_width` the viewport-bounded band
/// width (the band must never widen its container — that would defeat the
/// body's horizontal scroll); `sort` writes the QBE sort cell, `freeze`
/// sets the frozen count (0 = unfreeze all).
struct HeaderPass<'sort, 'freeze> {
    scroll_x: f32,
    frozen: usize,
    band_width: f32,
    sort: Option<&'sort mut dyn FnMut(usize, SortOrder)>,
    freeze: Option<&'freeze mut dyn FnMut(usize)>,
}

/// The shared Access grid: teal header band with resize handles and sort
/// affordances, alternating row shading, a clickable row selector (reports
/// the clicked row to `on_select`), an accent marker on the cursor row,
/// keyboard record navigation (step 40.1, item M38): arrow keys walk one
/// record, Home/End jump to the ends, PageUp/PageDown by the viewport, and
/// the virtualized window follows the cursor — plus freeze columns (step
/// 40.2, item M39): leading columns stay pinned in header and body while
/// the rest scrolls horizontally; the header context menu freezes and
/// unfreezes through `on_freeze` (the count, 0 = unfreeze). Record
/// deletion (step 40.3, item M44): the row selector's context menu offers
/// "Delete Record" — or a capability-disabled reason — and the Del key
/// deletes the selected record; both report through `on_delete` (the
/// zero-based live-row index) after the caller's own confirmation.
pub(crate) fn grid_ui(
    ui: &mut egui::Ui,
    grid: Grid<'_>,
    on_select: &mut dyn FnMut(usize),
    on_sort: Option<&mut dyn FnMut(usize, SortOrder)>,
    on_freeze: Option<&mut dyn FnMut(usize)>,
    mut on_delete: Option<&mut dyn FnMut(usize)>,
) {
    let Grid {
        metrics,
        columns,
        body,
        cursor,
        widths,
        sort_orders,
        id_salt,
        frozen,
        delete_reason,
    } = grid;
    let frozen = frozen.min(widths.len());
    // Navigation gate: the grid reacts when its focus anchor holds focus
    // (a row-selector click arms it) or when the pointer hovers it —
    // unless another widget, e.g. a text field, holds focus. Focus is read
    // live from memory so a just-clicked grid navigates in the very next
    // frame; hover uses the previous frame's anchor rect, which is stable.
    let anchor_id = ui.scope_id().with(&id_salt).with("grid-nav");
    let focused_elsewhere = ui
        .ctx()
        .memory(|memory| memory.focused().is_some_and(|id| id != anchor_id));
    let hover_armed = ui
        .ctx()
        .read_response(anchor_id)
        .is_some_and(|response| response.hovered());
    let wants_nav = ui.ctx().memory(|memory| memory.has_focus(anchor_id))
        || (hover_armed && !focused_elsewhere);
    // egui would otherwise move focus to the next focusable widget on the
    // first arrow press; the lock keeps the anchor focused so the whole
    // walk stays on the grid. Tab still leaves it (accessible), Esc still
    // reaches the global cancel-edit path.
    if ui.ctx().memory(|memory| memory.has_focus(anchor_id)) {
        ui.ctx().memory_mut(|memory| {
            memory.set_focus_lock_filter(
                anchor_id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..egui::EventFilter::default()
                },
            );
        });
    }
    let intent = if wants_nav { read_nav_intent(ui) } else { None };
    let viewport_height = (ui.available_height() - metrics.header_height).max(metrics.row_height);
    let follow_target =
        intent.map(|intent| nav_target(intent, cursor, body.len(), viewport_height, &metrics));
    // Del on the selected record is a deletion intent, not a cursor move —
    // it only fires for capable sources (step 40.3, item M44).
    let delete_intent =
        wants_nav && delete_reason.is_none() && read_delete_key(ui) && !body.is_empty();

    // The header trails the body's live horizontal offset by one frame:
    // the body scroll area is the single scroll source, the header only
    // mirrors it (the key is shared by the store after the body pass).
    let h_key = anchor_id.with("h-offset");
    let header_scroll_x = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(h_key))
        .unwrap_or(0.0);
    let mut header = HeaderPass {
        scroll_x: header_scroll_x,
        frozen,
        band_width: ui.available_width(),
        sort: on_sort,
        freeze: on_freeze,
    };
    header_ui(ui, &metrics, columns, widths, sort_orders, &mut header);
    let total_width: f32 = widths.iter().sum();
    let frozen_total = frozen_width(widths, frozen);
    let body_viewport = (ui.available_width() - SELECTOR_WIDTH).max(0.0);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());
    let mut clicked_row = None;
    let mut delete_row: Option<usize> = delete_intent.then(|| cursor.min(body.len() - 1));
    // Deterministic id of the body scroll area (`id_salt` below) — the row
    // painting reads the live horizontal offset from its state. The area
    // hashes its `IdSalt` through `make_persistent_id`, so the same
    // double-hash must be reproduced here.
    let body_id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));

    let mut scroll_area = egui::ScrollArea::both()
        .auto_shrink([false, false])
        .id_salt(&id_salt);
    if let Some(target) = follow_target {
        let offset = ui
            .ctx()
            .data(|data| data.get_temp::<f32>(anchor_id.with("offset")))
            .unwrap_or(0.0);
        scroll_area = scroll_area.vertical_scroll_offset(ensure_row_visible(
            offset,
            viewport_height,
            target,
            body.len(),
            &metrics,
        ));
    }
    let out = scroll_area.show_rows(ui, metrics.row_height, body.len(), |ui, range| {
        // Live horizontal offset of this frame's pass (clamped, as egui
        // applies it): frozen elements ride it so they stay pinned.
        let scroll_x = egui::scroll_area::State::load(ui.ctx(), body_id)
            .map(|state| clamp_scroll_x(state.clamped_offset().x, total_width, body_viewport))
            .unwrap_or(0.0);
        let widths_read: &[f32] = &*widths;
        for row_idx in range {
            let is_cursor = row_idx == cursor.min(body.len().saturating_sub(1));
            if row_idx % 2 == 1 {
                let band = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(SELECTOR_WIDTH + total_width, metrics.row_height),
                );
                ui.painter().rect_filled(band, 0.0, tokens.chrome);
            }
            ui.push_id(format!("row{row_idx}"), |ui| {
                // One full-width row slot: everything inside is placed at
                // computed positions, so the content size stays stable.
                let (row_rect, _) = ui.allocate_exact_size(
                    egui::vec2(SELECTOR_WIDTH + total_width, metrics.row_height),
                    egui::Sense::hover(),
                );
                let origin = row_rect.left_top();
                let row_h = metrics.row_height;
                // The selector rides the scroll compensation: pinned at
                // the viewport's left edge whatever the offset is.
                let sel_rect = egui::Rect::from_min_size(
                    origin + egui::vec2(scroll_x, 0.0),
                    egui::vec2(SELECTOR_WIDTH, row_h),
                );
                // The selector is an interact+paint pair, not a laid-out
                // Button: ui.put would clamp the rect against the
                // scrolled content ui's max_rect and break the pinned
                // placement. AccessKit still reports it as a Button with
                // the row number.
                let selector_label = format!("{:>6}", row_idx + 1);
                let selector_id = ui.scope_id().with(("selector", row_idx));
                let selector = ui.interact(sel_rect, selector_id, egui::Sense::click());
                selector.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, selector_label.clone())
                });
                if is_cursor {
                    ui.painter().rect_filled(sel_rect, 2.0, tokens.accent_faint);
                }
                ui.painter().text(
                    sel_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    selector_label,
                    font.clone(),
                    if is_cursor {
                        tokens.accent_text
                    } else {
                        tokens.text
                    },
                );
                if selector.clicked() {
                    clicked_row = Some(row_idx);
                }
                // The record menu (step 40.3, item M44): the deletion
                // affordance reports the row through `on_delete`; for
                // restricted sources it renders disabled with the reason.
                selector.context_menu(|ui| match delete_reason {
                    Some(reason) => {
                        ui.add_enabled(false, egui::Button::new(t("Delete Record")))
                            .on_hover_text(crate::availability::reason_text(&reason));
                    }
                    None => {
                        if ui.button(t("Delete Record")).clicked() {
                            delete_row = Some(row_idx);
                            ui.close();
                        }
                    }
                });
                // Painted cell: AccessKit label for the headless suite
                // (step 23.2, T5) plus the ink hairline separator.
                let paint_cell = |ui: &mut egui::Ui, x: f32, col: usize, text: &str| {
                    let cell = egui::Rect::from_min_size(
                        origin + egui::vec2(x, 0.0),
                        egui::vec2(
                            widths_read
                                .get(col)
                                .copied()
                                .unwrap_or(DEFAULT_COLUMN_WIDTH),
                            row_h,
                        ),
                    );
                    ui.painter_at(cell).text(
                        cell.left_center() + egui::vec2(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        tokens.text,
                    );
                    let cell_id = ui.scope_id().with(("cell", row_idx, col));
                    ui.interact(cell, cell_id, egui::Sense::hover())
                        .widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Label, true, text.to_owned())
                        });
                    ui.painter().line_segment(
                        [egui::pos2(cell.right(), cell.top()), cell.right_bottom()],
                        egui::Stroke::new(1.0, tokens.border),
                    );
                };
                let row = &body[row_idx];
                for col in 0..frozen {
                    let Some(text) = row.get(col) else { continue };
                    paint_cell(
                        ui,
                        SELECTOR_WIDTH + frozen_column_x(col, widths_read, scroll_x),
                        col,
                        text,
                    );
                }
                for col in visible_scrolled_range(scroll_x, body_viewport, widths_read, frozen) {
                    let Some(text) = row.get(col) else { continue };
                    paint_cell(
                        ui,
                        SELECTOR_WIDTH
                            + frozen_total
                            + scrolled_column_x(col, widths_read, frozen, scroll_x),
                        col,
                        text,
                    );
                }
                if is_cursor {
                    ui.painter().rect_filled(row_rect, 0.0, tokens.accent_faint);
                    let marker = egui::Rect::from_min_size(
                        row_rect.left_top(),
                        egui::vec2(3.0, row_rect.height()),
                    );
                    ui.painter().rect_filled(marker, 0.0, tokens.accent);
                }
            });
        }
    });
    ui.ctx().data_mut(|data| {
        let offset = out.state.clamped_offset();
        data.insert_temp(anchor_id.with("offset"), offset.y);
        data.insert_temp(h_key, offset.x);
    });
    // The focus anchor covers the whole grid: focusable but not
    // interactive, so it never steals clicks from the rows it overlays.
    // A row-selector click arms keyboard navigation for the next frame.
    let anchor = ui.interact(
        out.inner_rect,
        anchor_id,
        egui::Sense::focusable_noninteractive(),
    );
    if clicked_row.is_some() {
        anchor.request_focus();
    }
    if let Some(row) = clicked_row.or(follow_target) {
        on_select(row);
    }
    if let Some(row) = delete_row
        && let Some(callback) = on_delete.as_mut()
    {
        callback(row);
    }
}

/// Header band: accent fill, column names with sort indicators, resize
/// handles on every right edge, the sort affordances and the freeze
/// actions (header context menu). Painted over one full-width band; the
/// cells sit at computed positions — frozen ones pinned, scrolled ones
/// offset against `pass.scroll_x` (the body's offset, mirrored one frame
/// later).
fn header_ui(
    ui: &mut egui::Ui,
    metrics: &GridMetrics,
    columns: &[String],
    widths: &mut [f32],
    sort_orders: &[SortOrder],
    pass: &mut HeaderPass<'_, '_>,
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
}
