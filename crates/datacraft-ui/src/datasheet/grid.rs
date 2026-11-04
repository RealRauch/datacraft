//! The shared Access grid: teal header band, virtualized body, row
//! selector (step 6.1/6.2, items H22/H23; Access fidelity 12.5).
//!
//! Paint-only glue over the core grid math: virtual scrolling, resize
//! handles and sort affordances consume `datacraft_core::grid`; the
//! datasheet, the query results and the SQL window render through
//! [`grid_ui`], so all three behave identically.

use eframe::egui;

use datacraft_core::grid::{DEFAULT_COLUMN_WIDTH, GridMetrics, ensure_row_visible, resize_column};
use datacraft_core::qbe::SortOrder;

use super::nav::{nav_target, read_nav_intent};
use crate::i18n::t;
use crate::theme::tokens;

/// Width of the row-selector column (Access's record selector).
pub(super) const SELECTOR_WIDTH: f32 = 48.0;
/// Width of a column-separator drag handle.
const HANDLE_WIDTH: f32 = 5.0;

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller. `id_salt` names
/// the grid's focus anchor (keyboard navigation, step 40.1, item M38) and
/// must be unique per rendered grid.
pub(crate) struct Grid<'a> {
    pub metrics: GridMetrics,
    pub columns: &'a [String],
    pub body: &'a [Vec<String>],
    pub cursor: usize,
    pub widths: &'a mut [f32],
    pub sort_orders: &'a [SortOrder],
    pub id_salt: String,
}

/// The shared Access grid: teal header band with resize handles and sort
/// affordances, alternating row shading, a clickable row selector (reports
/// the clicked row to `on_select`), an accent marker on the cursor row and
/// keyboard record navigation (step 40.1, item M38): arrow keys walk one
/// record, Home/End jump to the ends, PageUp/PageDown by the viewport, and
/// the virtualized window follows the cursor.
pub(crate) fn grid_ui(
    ui: &mut egui::Ui,
    grid: Grid<'_>,
    on_select: &mut dyn FnMut(usize),
    on_sort: Option<&mut dyn FnMut(usize, SortOrder)>,
) {
    let Grid {
        metrics,
        columns,
        body,
        cursor,
        widths,
        sort_orders,
        id_salt,
    } = grid;
    header_ui(ui, &metrics, columns, widths, sort_orders, on_sort);
    let offsets: Vec<f32> = {
        let mut offsets = Vec::with_capacity(widths.len());
        let mut acc = 0.0;
        for width in widths.iter() {
            offsets.push(acc);
            acc += width;
        }
        offsets
    };
    let total_width = SELECTOR_WIDTH + widths.iter().sum::<f32>();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let tokens = tokens(ui.visuals());
    let mut clicked_row = None;

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

    let mut scroll_area = egui::ScrollArea::vertical()
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
        for row_idx in range {
            let is_cursor = row_idx == cursor.min(body.len().saturating_sub(1));
            if row_idx % 2 == 1 {
                let band = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(total_width, metrics.row_height),
                );
                ui.painter().rect_filled(band, 0.0, tokens.chrome);
            }
            ui.push_id(format!("row{row_idx}"), |ui| {
                let inner = ui
                    .horizontal(|ui| {
                        let selector = ui.add_sized(
                            [SELECTOR_WIDTH, metrics.row_height],
                            egui::Button::selectable(
                                is_cursor,
                                egui::RichText::new(format!("{:>6}", row_idx + 1)).color(
                                    if is_cursor {
                                        tokens.accent_text
                                    } else {
                                        tokens.text
                                    },
                                ),
                            ),
                        );
                        if selector.clicked() {
                            clicked_row = Some(row_idx);
                        }
                        for (col, text) in body[row_idx].iter().enumerate() {
                            let cell = egui::Rect::from_min_size(
                                selector.rect.right_top()
                                    + egui::vec2(offsets.get(col).copied().unwrap_or(0.0), 0.0),
                                egui::vec2(
                                    widths.get(col).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                                    metrics.row_height,
                                ),
                            );
                            ui.painter_at(cell).text(
                                cell.left_center() + egui::vec2(4.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                text,
                                font.clone(),
                                tokens.text,
                            );
                            // Cells are painted, so expose them as
                            // AccessKit labels for the headless suite
                            // (step 23.2, T5): label-based addressing
                            // instead of pixel coordinates.
                            let cell_id = ui.scope_id().with(("cell", row_idx, col));
                            ui.interact(cell, cell_id, egui::Sense::hover())
                                .widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::Role::Label, true, text.clone())
                                });
                            ui.painter().line_segment(
                                [egui::pos2(cell.right(), cell.top()), cell.right_bottom()],
                                egui::Stroke::new(1.0, tokens.border),
                            );
                        }
                    })
                    .response;
                if is_cursor {
                    ui.painter()
                        .rect_filled(inner.rect, 0.0, tokens.accent_faint);
                    let marker = egui::Rect::from_min_size(
                        inner.rect.left_top(),
                        egui::vec2(3.0, inner.rect.height()),
                    );
                    ui.painter().rect_filled(marker, 0.0, tokens.accent);
                }
            });
        }
    });
    ui.ctx()
        .data_mut(|data| data.insert_temp(anchor_id.with("offset"), out.state.clamped_offset().y));
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
}

/// Header band: accent fill, column names with sort indicators, resize
/// handles on every right edge, and the sort affordances.
fn header_ui(
    ui: &mut egui::Ui,
    metrics: &GridMetrics,
    columns: &[String],
    widths: &mut [f32],
    sort_orders: &[SortOrder],
    mut on_sort: Option<&mut dyn FnMut(usize, SortOrder)>,
) {
    let tokens = tokens(ui.visuals());
    let font = egui::TextStyle::Body.resolve(ui.style());
    let sortable = on_sort.is_some();
    let mut pending: Option<(usize, SortOrder)> = None;
    ui.horizontal(|ui| {
        let (selector, _) = ui.allocate_exact_size(
            egui::vec2(SELECTOR_WIDTH, metrics.header_height),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(selector, 0.0, tokens.accent_fill);
        for (col, name) in columns.iter().enumerate() {
            let width = widths.get(col).copied().unwrap_or(DEFAULT_COLUMN_WIDTH);
            let cell_width = (width - HANDLE_WIDTH).max(1.0);
            let sense = if sortable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(cell_width, metrics.header_height), sense);
            ui.painter().rect_filled(rect, 0.0, tokens.accent_fill);
            let order = sort_orders.get(col).copied().unwrap_or_default();
            ui.painter().text(
                rect.left_center() + egui::vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                name,
                font.clone(),
                tokens.on_accent,
            );
            // Sort caret: painted, not a font glyph (Inter has no U+25B2).
            if order != SortOrder::None {
                let cx = rect.right() - 9.0;
                let cy = rect.center().y;
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
            if sortable {
                if response.clicked() {
                    pending = Some((col, order.toggled()));
                }
                response.context_menu(|ui| {
                    for target in [SortOrder::Ascending, SortOrder::Descending, SortOrder::None] {
                        let text = match target {
                            SortOrder::Ascending => t("Sort Ascending"),
                            SortOrder::Descending => t("Sort Descending"),
                            SortOrder::None => t("Clear Sort"),
                        };
                        if ui.selectable_label(order == target, text).clicked() {
                            pending = Some((col, target));
                            ui.close();
                        }
                    }
                });
            }
            let (handle, drag) = ui.allocate_exact_size(
                egui::vec2(HANDLE_WIDTH, metrics.header_height),
                egui::Sense::drag(),
            );
            if drag.hovered() || drag.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if drag.dragged()
                && let Some(width) = widths.get_mut(col)
            {
                *width = resize_column(*width, drag.drag_delta().x);
            }
            ui.painter().line_segment(
                [handle.left_top(), handle.left_bottom()],
                egui::Stroke::new(1.0, tokens.border),
            );
        }
    });
    if let Some((col, order)) = pending
        && let Some(callback) = on_sort.as_mut()
    {
        callback(col, order);
    }
}
