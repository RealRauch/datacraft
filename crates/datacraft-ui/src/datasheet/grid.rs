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

use datacraft_core::grid::{
    DEFAULT_COLUMN_WIDTH, clamp_scroll_x, ensure_row_visible, frozen_column_x, frozen_width,
    scrolled_column_x, visible_scrolled_range,
};
use datacraft_core::selection::Selection;

use super::grid_keys::{GridKeys, grid_keys};
use super::grid_model::{EditIntent, Grid, GridCallbacks};
use super::header::{HeaderPass, header_ui};
use super::nav::nav_target;

use crate::theme::tokens;

/// Width of the row-selector column (Access's record selector).
pub(super) const SELECTOR_WIDTH: f32 = 48.0;

pub(crate) fn grid_ui<'a>(ui: &mut egui::Ui, grid: Grid<'a>, callbacks: GridCallbacks<'a>) {
    let GridCallbacks {
        on_select,
        mut on_selection,
        on_select_column,
        mut on_copy,
        mut on_paste,
        mut on_fill,
        mut on_clear,
        mut on_zoom,
        on_sort,
        on_freeze,
        mut on_delete,
        mut on_edit,
        on_filter_dialog,
        mut on_follow,
        mut on_follow_menu,
        filter_builder,
    } = callbacks;
    let Grid {
        metrics,
        columns,
        body,
        cursor,
        selection,
        widths,
        sort_orders,
        filters: grid_filters,
        follow_targets,
        follow_counts,
        id_salt,
        frozen,
        delete_reason,
        edit,
        edit_restricted,
        pending,
        locked,
    } = grid;
    let mut edit = edit;
    let frozen = frozen.min(widths.len());
    // A submission in flight locks the grid (step 85.2, item H61): no
    // editor arms, no deletion reports, the cells keep rendering their
    // staged image.
    let edit_restricted = edit_restricted || locked;
    // Navigation gate and the frame's key intents (step 87.1, item M103
    // moved the pass into `grid_keys` — the L30 split below the gate).
    let anchor_id = ui.scope_id().with(&id_salt).with("grid-nav");
    let GridKeys {
        nav,
        select_all,
        copy,
        paste,
        fill,
        zoom_key,
        follow_key,
        clear_intent,
        delete_intent,
    } = grid_keys(
        ui,
        anchor_id,
        body.len(),
        selection,
        edit_restricted,
        locked,
        delete_reason.is_none(),
    );
    let viewport_height = (ui.available_height() - metrics.header_height).max(metrics.row_height);
    let follow_target = nav
        .as_ref()
        .map(|(intent, _)| nav_target(*intent, cursor, body.len(), viewport_height, &metrics));

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
        filter_dialog: on_filter_dialog,
        filter_builder,
        select_column: on_select_column,
        filters: grid_filters,
        selection,
    };
    header_ui(ui, &metrics, columns, widths, sort_orders, &mut header);
    let total_width: f32 = widths.iter().sum();
    let frozen_total = frozen_width(widths, frozen);
    let body_viewport = (ui.available_width() - SELECTOR_WIDTH).max(0.0);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());
    // Loaded bounds for the selection's containment checks (step 86.1,
    // item M101) — copied so the cell closure shares them.
    let body_row_total = body.len();
    let col_total = columns.len();
    let mut clicked_row = None;
    let mut delete_row: Option<usize> = delete_intent.then(|| cursor.min(body.len() - 1));
    let mut edit_intent: Option<EditIntent> = None;
    // Deterministic id of the body scroll area (`id_salt` below) — the row
    // painting reads the live horizontal offset from its state. The area
    // hashes its `IdSalt` through `make_persistent_id`, so the same
    // double-hash must be reproduced here.
    let body_id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));

    // The moving corner's column for keyboard selection (step 86.1, item
    // M101); `selection_after_nav` applies the intent to it.
    let focus_col = selection.focus_cell().col;
    let extend_nav = nav.map(|(_, extend)| extend).unwrap_or(false);
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
                // Pending-change markers (step 85.2, item H61): staged
                // updates and inserts carry the amber bar on the row,
                // staged deletes wash the whole record — the database is
                // untouched until the batch applies.
                let pending_row = pending.get(row_idx);
                if let Some(pending_row) = pending_row {
                    match pending_row.state {
                        datacraft_core::edit_buffer::RowState::Deleted => {
                            ui.painter().rect_filled(
                                row_rect,
                                0.0,
                                tokens.danger.gamma_multiply(0.12),
                            );
                        }
                        datacraft_core::edit_buffer::RowState::Updated
                        | datacraft_core::edit_buffer::RowState::Inserted => {
                            let marker = egui::Rect::from_min_size(
                                row_rect.left_top(),
                                egui::vec2(3.0, row_rect.height()),
                            );
                            ui.painter().rect_filled(marker, 0.0, tokens.pending);
                        }
                        _ => {}
                    }
                }
                // The selector rides the scroll compensation: pinned at
                // the viewport's left edge whatever the offset is. The
                // paint lives in `selector.rs` (the size-gate split).
                super::selector::record_selector(
                    ui,
                    super::selector::selector_rect(origin, scroll_x, row_h),
                    row_idx,
                    is_cursor,
                    delete_reason.as_ref(),
                    locked,
                    tokens,
                    &font,
                    &mut clicked_row,
                    &mut delete_row,
                );
                // Painted cell: AccessKit label for the headless suite
                // (step 23.2, T5) plus the ink hairline separator. On an
                // editable source the cell also senses clicks: clicking a
                // cell selects its record, and clicking one of the
                // selected record arms the in-place editor (step 40.4,
                // L23). The edited cell renders a real TextEdit instead.
                let mut cell_ui = |ui: &mut egui::Ui, x: f32, col: usize, text: &str| {
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
                    let editing_here =
                        matches!(edit.as_ref(), Some(slot) if slot.row == row_idx && slot.col == col);
                    if editing_here {
                        let Some(slot) = edit.as_mut() else {
                            return;
                        };
                        let editor_id = ui.scope_id().with(("editor", row_idx, col));
                        let is_new = ui.ctx().read_response(editor_id).is_none();
                        let width = widths_read
                            .get(col)
                            .copied()
                            .unwrap_or(DEFAULT_COLUMN_WIDTH);
                        let response = ui.put(
                            cell,
                            egui::TextEdit::singleline(slot.buffer)
                                .id(editor_id)
                                .desired_width(width)
                                .font(egui::TextStyle::Body)
                                // The column name doubles as the editor's
                                // accessible name (the new-record row's
                                // pattern) — the suite asserts on it.
                                .hint_text(
                                    columns.get(col).map(String::as_str).unwrap_or_default(),
                                ),
                        );
                        if is_new {
                            response.request_focus();
                        }
                        let (enter, esc) = ui.input(|i| {
                            (
                                i.key_pressed(egui::Key::Enter),
                                i.key_pressed(egui::Key::Escape),
                            )
                        });
                        if response.has_focus() && (enter || esc) {
                            edit_intent = Some(if esc {
                                EditIntent::Cancel
                            } else {
                                EditIntent::Commit {
                                    row: row_idx,
                                    col,
                                }
                            });
                        } else if response.lost_focus() {
                            // Focus dropped without a key this frame
                            // (click-away): Escape drops focus in the
                            // same frame it is pressed, so `esc` still
                            // separates cancel from commit.
                            edit_intent = Some(if esc {
                                EditIntent::Cancel
                            } else {
                                EditIntent::Commit {
                                    row: row_idx,
                                    col,
                                }
                            });
                        }
                        return;
                    }
                    // Selected cells carry the faint accent wash under
                    // their text (step 86.1, item M101).
                    if selection.contains(row_idx, col, body_row_total, col_total) {
                        ui.painter()
                            .rect_filled(cell, 0.0, tokens.accent_faint);
                    }
                    ui.painter_at(cell).text(
                        cell.left_center() + egui::vec2(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        tokens.text,
                    );
                    // Staged cells carry the amber wash and outline (step
                    // 85.2, item H61) — a visual state only; the value is
                    // untouched until the batch applies.
                    if let Some(pending_row) = pending.get(row_idx)
                        && pending_row.changed.contains(&col)
                    {
                        ui.painter().rect_filled(
                            cell,
                            0.0,
                            tokens.pending.gamma_multiply(0.18),
                        );
                        ui.painter()
                            .rect_stroke(cell, 0.0, egui::Stroke::new(1.0, tokens.pending), egui::StrokeKind::Inside);
                    }
                    // Click sense on every grid: a read-only cell still
                    // offers zoom (step 86.4, item M102); the edit arm
                    // stays gated below.
                    let sense = egui::Sense::click();
                    let cell_id = ui.scope_id().with(("cell", row_idx, col));
                    let response = ui.interact(cell, cell_id, sense);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Label, true, text.to_owned())
                    });
                    ui.painter().line_segment(
                        [egui::pos2(cell.right(), cell.top()), cell.right_bottom()],
                        egui::Stroke::new(1.0, tokens.border),
                    );
                    // Zoom intents (step 86.4, item M102); see
                    // grid_cells — a zooming click never arms the editor.
                    let zoomed = super::grid_cells::cell_zoom_interactions(
                        ui,
                        &response,
                        row_idx,
                        col,
                        edit_restricted,
                        &mut on_zoom,
                        follow_targets
                            .get(col)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                        follow_counts,
                        &mut on_follow,
                        &mut on_follow_menu,
                    );
                    if zoomed {
                        return;
                    }
                    if response.clicked() {
                        // Selection intents (step 86.1, item M101): a plain
                        // click collapses onto the cell (the anchor),
                        // Shift+click extends; the editor arms only on a
                        // plain click into the cursor row.
                        let shift = ui.input(|i| i.modifiers.shift);
                        if let Some(callback) = on_selection.as_mut() {
                            let new_selection = if shift {
                                selection.extend_to_cell(row_idx, col)
                            } else {
                                Selection::collapse(row_idx, col)
                            };
                            callback(new_selection);
                        }
                        clicked_row = Some(row_idx);
                        if !shift
                            && is_cursor
                            && edit.is_none()
                            && !edit_restricted
                        {
                            edit_intent = Some(EditIntent::Start {
                                row: row_idx,
                                col,
                            });
                        }
                    }
                };
                let row = &body[row_idx];
                for col in 0..frozen {
                    let Some(text) = row.get(col) else { continue };
                    cell_ui(
                        ui,
                        SELECTOR_WIDTH + frozen_column_x(col, widths_read, scroll_x),
                        col,
                        text,
                    );
                }
                for col in visible_scrolled_range(scroll_x, body_viewport, widths_read, frozen) {
                    let Some(text) = row.get(col) else { continue };
                    cell_ui(
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
    // Selection intents (step 86.1, item M101): a plain navigation key
    // collapses the selection onto the new cursor row, a Shift-modified
    // one extends from the current anchor (`All` stays all), Ctrl+A
    // selects everything loaded. Empty grids select nothing.
    if !body.is_empty()
        && let Some(target) = follow_target
        && let Some(callback) = on_selection.as_mut()
    {
        callback(crate::datasheet::nav::selection_after_nav(
            selection, target, extend_nav, focus_col,
        ));
    }
    if select_all && let Some(callback) = on_selection.as_mut() {
        callback(Selection::All);
    }
    if copy && let Some(callback) = on_copy.as_mut() {
        callback();
    }
    if paste && let Some(callback) = on_paste.as_mut() {
        callback();
    }
    if fill && let Some(callback) = on_fill.as_mut() {
        callback();
    }
    if clear_intent && let Some(callback) = on_clear.as_mut() {
        callback();
    }
    if zoom_key && let Some(callback) = on_zoom.as_mut() {
        let (row, col) = super::grid_cells::zoom_focus_target(selection, body.len(), columns.len());
        callback(row, col);
    }
    if follow_key
        && let Some(callback) = on_follow.as_mut()
        && let Some((row, col)) =
            super::grid_cells::follow_key_target(selection, body.len(), follow_targets)
    {
        callback(row, col, 0);
    }
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
    if let Some(intent) = edit_intent
        && let Some(callback) = on_edit.as_mut()
    {
        callback(intent);
    }
}
