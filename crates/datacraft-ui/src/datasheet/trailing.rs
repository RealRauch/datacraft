//! The datasheet's trailing panels below the grid: the row-count label
//! and the new-record row (step 48.2, item M61).
//!
//! The virtualized grid fills its viewport; these panels render after it.
//! The grid's row pass therefore reserves [`reserve_height`] first —
//! without that budget the panels land behind the status bar and the
//! Add-record affordance is unreachable (the user could not add a record
//! at all, report 2026-11-07).

use eframe::egui;

use datacraft_core::export::ExportFormat;
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
/// the caller reserved their height before laying the grid out. The
/// foreign-key navigation's Back/Forward buttons ride the same row
/// (step 87.2, item M103): disabled while the tab's stack is empty.
pub(super) fn trailing_ui(
    app: &mut App,
    tab: &TabSnapshot,
    row_count: usize,
    columns: &[String],
    widths: &[f32],
    ui: &mut egui::Ui,
) {
    ui.horizontal(|ui| {
        let back = ui
            .add_enabled(app.fk_back_len(tab.id) > 0, egui::Button::new(t("Back")))
            .on_hover_text(t("Back to the previous table and filter"));
        let forward = ui
            .add_enabled(
                app.fk_forward_len(tab.id) > 0,
                egui::Button::new(t("Forward")),
            )
            .on_hover_text(t("Forward to the next table and filter"));
        let count =
            ui.label(egui::RichText::new(plural(row_count, "{count} row", "{count} rows")).small());
        crate::layout_probe::record("datasheet.row_count", count.rect);
        if back.clicked() {
            app.fk_back(tab.id);
        }
        if forward.clicked() {
            app.fk_forward(tab.id);
        }
        export_buttons(app, tab, ui);
    });
    new_row_ui(app, tab, columns, widths, ui);
}

/// The table export entries (phase 99, step 99.2; item M130): one button
/// per format, each parking the save dialog with the table's suggested
/// file name. The command runs on the answer, so the flow never blocks
/// inside the frame that asked.
fn export_buttons(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let csv = ui
        .button(t("Export CSV…"))
        .on_hover_text(t("Export the table to a CSV file."));
    if csv.clicked() {
        request_table_export(app, tab, ExportFormat::Csv);
    }
    let json = ui
        .button(t("Export JSON…"))
        .on_hover_text(t("Export the table to a JSON document (schema and rows)."));
    if json.clicked() {
        request_table_export(app, tab, ExportFormat::Json);
    }
}

/// Parks the save dialog for one table export.
fn request_table_export(app: &mut App, tab: &TabSnapshot, format: ExportFormat) {
    app.request_dialog(
        crate::dialogs::DialogRequest::SaveFile {
            suggested: format!("{}.{}", tab.object_name, format.extension()),
        },
        crate::app::dialog_flow::DialogIntent::ExportTable {
            table: tab.object_name.clone(),
            format,
        },
    );
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
    // cells. A submission in flight disables it too (step 85.2, H61).
    let submission_open = app.snapshot().pending_write.is_some();
    let new_record_reason = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::NEW_RECORD,
    );
    let mut cells = app.new_row_cells(tab.id, columns.len());
    // The AutoNumber gate (step 91.6, item M114): engine-assigned columns
    // render disabled before input — the contract never surfaces as a
    // runtime rejection after typing.
    let autonumber: Vec<bool> = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&tab.object_name))
        })
        .map(|table| {
            columns
                .iter()
                .map(|name| {
                    table
                        .fields
                        .iter()
                        .find(|field| field.name.eq_ignore_ascii_case(name))
                        .is_some_and(|field| {
                            matches!(field.kind, datacraft_core::schema::FieldKind::AutoNumber(_))
                        })
                })
                .collect()
        })
        .unwrap_or_else(|| vec![false; columns.len()]);
    let mut add = false;
    egui::ScrollArea::horizontal()
        .id_salt(format!("datasheet.newrow.{}", tab.ui_test_id))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(SELECTOR_WIDTH);
                ui.label(egui::RichText::new("*").strong());
                for (index, cell) in cells.iter_mut().enumerate() {
                    let edit = egui::TextEdit::singleline(cell).hint_text(&columns[index]);
                    let width = widths.get(index).copied().unwrap_or(DEFAULT_COLUMN_WIDTH);
                    if new_record_reason.is_some() || submission_open {
                        ui.add_enabled(false, edit.desired_width(width));
                    } else if autonumber[index] {
                        let response = ui
                            .add_enabled(false, edit.desired_width(width))
                            .on_hover_text(t("Input is rejected — the engine assigns the value."));
                        crate::layout_probe::record(
                            "datasheet.newrow.autonumber_gate",
                            response.rect,
                        );
                    } else {
                        ui.add_sized([width, 22.0], edit);
                    }
                }
            });
            crate::layout_probe::record("datasheet.new_row_cells", ui.min_rect());
        });
    ui.horizontal(|ui| {
        ui.add_space(SELECTOR_WIDTH);
        let add_response = match (&new_record_reason, submission_open) {
            (Some(reason), _) => ui
                .add_enabled(false, egui::Button::new(t("Add record")))
                .on_hover_text(crate::availability::reason_text(reason)),
            (None, true) => ui
                .add_enabled(false, egui::Button::new(t("Add record")))
                .on_hover_text(t("A submission is in flight — apply or discard it first.")),
            (None, false) => ui.button(t("Add record")),
        };
        if add_response.clicked() {
            add = true;
        }
        crate::layout_probe::record("datasheet.new_row_add", add_response.rect);
        ui.weak(t(
            "Empty cells are Null; AutoNumber cells reject input — the engine assigns the value.",
        ));
    });
    app.set_new_row_cells(tab.id, cells);
    if add {
        app.insert_new_row(tab.id, &tab.object_name);
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::{Lang, lang_lock, set_lang, t};

    /// The AutoNumber hint names the contract (step 91.6, item M114):
    /// the English key resolves in both catalogs. (The trailing row sits
    /// below the a11y fold in headless harnesses, so the string pin rides
    /// here and the gate itself rides the layout probe.)
    #[test]
    fn the_autonumber_hint_names_the_contract_in_both_languages() {
        let _guard = lang_lock();
        const KEY: &str =
            "Empty cells are Null; AutoNumber cells reject input — the engine assigns the value.";
        set_lang(Lang::English);
        assert!(t(KEY).contains("reject input"), "{}", t(KEY));
        set_lang(Lang::German);
        assert!(t(KEY).contains("lehnen Eingaben ab"), "{}", t(KEY));
        set_lang(Lang::English);
    }
}
