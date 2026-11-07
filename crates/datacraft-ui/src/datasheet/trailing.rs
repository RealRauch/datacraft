//! The datasheet's trailing panels below the grid: the row-count label
//! and the new-record row (step 48.2, item M61).
//!
//! The virtualized grid fills its viewport; these panels render after it.
//! The grid's row pass therefore reserves [`reserve_height`] first —
//! without that budget the panels land behind the status bar and the
//! Add-record affordance is unreachable (the user could not add a record
//! at all, report 2026-11-07).

use eframe::egui;

use datacraft_core::grid::DEFAULT_COLUMN_WIDTH;
use datacraft_core::snapshot::TabSnapshot;

use super::grid::SELECTOR_WIDTH;
use crate::app::App;
use crate::i18n::{plural, t};

/// The vertical budget the trailing panels need below the grid: one item
/// slot for the row-count label, the new-record separator and edit row,
/// the Add-record button row, each with the live item spacing. Computed
/// from the theme metrics so a spacing or font change moves the budget
/// with it; the M61 rect pins arbitrate the exact numbers.
pub(super) fn reserve_height(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing().item_spacing.y;
    let button = ui.spacing().interact_size.y;
    let label = ui.text_style_height(&egui::TextStyle::Small);
    spacing + label
        // the new-record row's separator
        + spacing + 6.0
        // the edit row (fixed height in `new_row_ui`)
        + spacing + 22.0
        // the Add-record button row
        + spacing + button
}

/// The row-count label and the new-record row, rendered after the grid —
/// the caller reserved their height before laying the grid out.
pub(super) fn trailing_ui(
    app: &mut App,
    tab: &TabSnapshot,
    row_count: usize,
    columns: &[String],
    widths: &[f32],
    ui: &mut egui::Ui,
) {
    let count =
        ui.label(egui::RichText::new(plural(row_count, "{count} row", "{count} rows")).small());
    crate::layout_probe::record("datasheet.row_count", count.rect);
    new_row_ui(app, tab, columns, widths, ui);
}

/// The datasheet's new-record row (step 11.7): one text cell per column,
/// marked with the Access asterisk. "Add record" parses the cells and
/// inserts through the bus; failures land in the status bar.
fn new_row_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    widths: &[f32],
    ui: &mut egui::Ui,
) {
    ui.separator();
    // Capability-disabled for the open source (ADR-7): the read-only
    // new-record row renders muted with the reason instead of editable
    // cells.
    let new_record_reason = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::NEW_RECORD,
    );
    let mut cells = app.new_row_cells(tab.id, columns.len());
    let mut add = false;
    egui::ScrollArea::horizontal()
        .id_salt(format!("datasheet.newrow.{}", tab.ui_test_id))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(SELECTOR_WIDTH);
                ui.label(egui::RichText::new("*").strong());
                for (index, cell) in cells.iter_mut().enumerate() {
                    let edit = egui::TextEdit::singleline(cell).hint_text(&columns[index]);
                    if new_record_reason.is_some() {
                        ui.add_enabled(
                            false,
                            edit.desired_width(
                                widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                            ),
                        );
                    } else {
                        ui.add_sized(
                            [
                                widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH),
                                22.0,
                            ],
                            edit,
                        );
                    }
                }
            });
            crate::layout_probe::record("datasheet.new_row_cells", ui.min_rect());
        });
    ui.horizontal(|ui| {
        ui.add_space(SELECTOR_WIDTH);
        let add_response = match new_record_reason {
            Some(reason) => ui
                .add_enabled(false, egui::Button::new(t("Add record")))
                .on_hover_text(crate::availability::reason_text(reason)),
            None => ui.button(t("Add record")),
        };
        if add_response.clicked() {
            add = true;
        }
        crate::layout_probe::record("datasheet.new_row_add", add_response.rect);
        ui.weak(t("Empty cells are Null; AutoNumber assigns itself."));
    });
    app.set_new_row_cells(tab.id, cells);
    if add {
        app.insert_new_row(tab.id, &tab.object_name);
    }
}
