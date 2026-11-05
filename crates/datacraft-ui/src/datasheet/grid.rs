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
    frozen_width, scrolled_column_x, visible_scrolled_range,
};
use datacraft_core::qbe::SortOrder;

use super::header::{HeaderPass, header_ui};
use super::nav::{nav_target, read_delete_key, read_nav_intent};
use crate::i18n::t;
use crate::theme::tokens;

/// Width of the row-selector column (Access's record selector).
pub(super) const SELECTOR_WIDTH: f32 = 48.0;

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller. `id_salt` names
/// the grid's focus anchor (keyboard navigation, step 40.1, item M38) and
/// must be unique per rendered grid; `frozen` is the effective count of
/// frozen leading columns (step 40.2, item M39); `delete_reason` restricts
/// the record-deletion affordance for the open source (step 40.3, M44);
/// `edit` is the armed in-place cell editor (step 40.4, L23) and
/// `edit_restricted` keeps the cells inert on sources that cannot write.
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
    pub edit: Option<CellEditSlot<'a>>,
    pub edit_restricted: bool,
}

/// The armed in-place cell editor (step 40.4, item L23): the cell being
/// edited and its buffer, owned by the tab's presentation state.
pub(crate) struct CellEditSlot<'a> {
    pub row: usize,
    pub col: usize,
    pub buffer: &'a mut String,
}

/// One cell-editing intent reported by [`grid_ui`] to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditIntent {
    /// The clicked cell of the selected record arms the editor.
    Start { row: usize, col: usize },
    /// Enter or a click-away commits the buffer for the cell.
    Commit { row: usize, col: usize },
    /// Escape discards the buffer.
    Cancel,
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
/// In-place cell editing (step 40.4, item L23): a click into a cell of
/// the selected record arms an in-cell editor ([`EditIntent::Start`]),
/// Enter or a click-away commits ([`EditIntent::Commit`]), Escape
/// discards ([`EditIntent::Cancel`]); restricted sources keep the cells
/// inert.
/// The grid's paint callbacks plus the header menu's filter drafts
/// (step 44.5, M54 — bundled so `grid_ui` stays under the arity lint):
/// selection, QBE sort writes (query grids), freeze, record deletion,
/// in-place cell editing and the table column filters.
pub(crate) struct GridCallbacks<'a> {
    /// Row selection (display index).
    pub on_select: &'a mut dyn FnMut(usize),
    /// Header sort (query grids write the QBE sort cell).
    pub on_sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    /// Freeze/unfreeze through the header context menu.
    pub on_freeze: Option<&'a mut dyn FnMut(usize)>,
    /// Record deletion (after the caller's confirmation).
    pub on_delete: Option<&'a mut dyn FnMut(usize)>,
    /// In-place cell editing intents.
    pub on_edit: Option<&'a mut dyn FnMut(EditIntent)>,
    /// Table column filters from the header menu (`None` clears).
    pub on_filter: Option<&'a mut dyn FnMut(usize, Option<String>)>,
    /// The per-column criteria drafts the menu's editor edits.
    pub filter_drafts: &'a mut [String],
}

/// Bundles the callbacks for [`grid_ui`] (keeps the call sites tidy).
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_ui_callbacks<'a>(
    on_select: &'a mut dyn FnMut(usize),
    on_sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    on_freeze: Option<&'a mut dyn FnMut(usize)>,
    on_delete: Option<&'a mut dyn FnMut(usize)>,
    on_edit: Option<&'a mut dyn FnMut(EditIntent)>,
    on_filter: Option<&'a mut dyn FnMut(usize, Option<String>)>,
    filter_drafts: &'a mut [String],
) -> GridCallbacks<'a> {
    GridCallbacks {
        on_select,
        on_sort,
        on_freeze,
        on_delete,
        on_edit,
        on_filter,
        filter_drafts,
    }
}

pub(crate) fn grid_ui<'a>(ui: &mut egui::Ui, grid: Grid<'_>, callbacks: GridCallbacks<'a>) {
    let GridCallbacks {
        on_select,
        on_sort,
        on_freeze,
        mut on_delete,
        mut on_edit,
        on_filter,
        filter_drafts,
    } = callbacks;
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
        edit,
        edit_restricted,
    } = grid;
    let mut edit = edit;
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
        filter: on_filter,
        filter_drafts,
    };
    header_ui(ui, &metrics, columns, widths, sort_orders, &mut header);
    let total_width: f32 = widths.iter().sum();
    let frozen_total = frozen_width(widths, frozen);
    let body_viewport = (ui.available_width() - SELECTOR_WIDTH).max(0.0);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());
    let mut clicked_row = None;
    let mut delete_row: Option<usize> = delete_intent.then(|| cursor.min(body.len() - 1));
    let mut edit_intent: Option<EditIntent> = None;
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
                    ui.painter_at(cell).text(
                        cell.left_center() + egui::vec2(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        tokens.text,
                    );
                    let sense = if edit_restricted {
                        egui::Sense::hover()
                    } else {
                        egui::Sense::click()
                    };
                    let cell_id = ui.scope_id().with(("cell", row_idx, col));
                    let response = ui.interact(cell, cell_id, sense);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Label, true, text.to_owned())
                    });
                    ui.painter().line_segment(
                        [egui::pos2(cell.right(), cell.top()), cell.right_bottom()],
                        egui::Stroke::new(1.0, tokens.border),
                    );
                    if response.clicked() {
                        clicked_row = Some(row_idx);
                        if is_cursor
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
