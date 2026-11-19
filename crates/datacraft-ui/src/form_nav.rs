//! The form runtime's record navigation row (step 91.12, item M124;
//! split from `form_runtime.rs` to stay under the file-size gate; L30).

use eframe::egui;

use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};

/// The navigation row: first/prev/next/last jump the ends through the
/// flush-then-navigate path, and the new-record action inserts through
/// the normal path — the form's own record source with empty cells
/// (engine-assigned AutoNumbers per the M114 contract), capability-gated
/// like the datasheet's row actions with the reason on hover. After the
/// insert the cursor's survival logic pins the new last row (the form
/// sits on the new record, Access-style).
pub(super) fn record_nav_ui(
    app: &mut App,
    tab: &TabSnapshot,
    source: &str,
    record: usize,
    row_count: usize,
    ui: &mut egui::Ui,
) {
    ui.separator();
    ui.horizontal(|ui| {
        // The fuller Access set (step 91.12, item M124): first/last jump
        // the ends; every jump flushes the per-record edit buffer exactly
        // like prev/next.
        if ui.button(t("First")).clicked() && record > 0 && app.flush_form_edits(tab.id) {
            app.set_form_record(tab.id, 0);
        }
        if ui.button(t("< Prev")).clicked() && record > 0 && app.flush_form_edits(tab.id) {
            app.set_form_record(tab.id, record - 1);
        }
        ui.label(tr(
            "Record {current} of {total}",
            &[
                ("current", &(record + 1).to_string()),
                ("total", &row_count.to_string()),
            ],
        ));
        if ui.button(t("Next >")).clicked()
            && record + 1 < row_count
            && app.flush_form_edits(tab.id)
        {
            app.set_form_record(tab.id, record + 1);
        }
        if ui.button(t("Last")).clicked() && record + 1 < row_count && app.flush_form_edits(tab.id)
        {
            app.set_form_record(tab.id, row_count - 1);
        }
        let new_record_restriction = crate::availability::restricted(
            app.snapshot(),
            datacraft_core::availability::ids::NEW_RECORD,
        )
        .copied();
        let new_record = match new_record_restriction {
            Some(reason) => ui
                .add_enabled(false, egui::Button::new(t("New record")))
                .on_hover_text(crate::availability::reason_text(&reason)),
            None => ui.button(t("New record")),
        };
        if new_record.clicked()
            && app.flush_form_edits(tab.id)
            && app.insert_new_row(tab.id, source)
        {
            // The cursor's survival logic follows the new last row (the
            // form sits on the new record, Access-style).
            app.follow_physical_row(tab.id, row_count);
            app.set_form_record(tab.id, row_count);
        }
    });
}
