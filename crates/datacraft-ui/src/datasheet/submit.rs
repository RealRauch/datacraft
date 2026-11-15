//! The submission preview window (step 85.2, item H61).
//!
//! Ctrl+S previews the tab's edit-buffer batch: the window renders the
//! journal's own changes (`Event::WritePreviewed` diff — no second SQL
//! generator). Apply submits the token as one atomic batch; Discard —
//! like the window's close box — drops the journal so the grid unlocks
//! and keeps every staged edit for further editing.

use eframe::egui;

use datacraft_core::journal::ChangeKind;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::t;
use crate::labels::value_label;

/// Renders the open submission of this tab, if any.
pub(super) fn submission_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    ui: &mut egui::Ui,
) {
    let Some(submission) = app.submission() else {
        return;
    };
    if submission.tab != tab.id {
        return;
    }
    let changes = submission.changes.clone();
    let mut apply = false;
    let mut discard = false;
    let mut open = true;
    egui::Window::new(t("Pending Changes"))
        .open(&mut open)
        .id(egui::Id::unique(format!(
            "datacraft.submit.{}",
            tab.ui_test_id
        )))
        .default_width(420.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(420.0, 320.0),
        ))
        .show(ui, |ui| {
            ui.label(t(
                "The batch applies as one atomic write when you confirm it.",
            ));
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for change in &changes {
                        ui.label(change_label(columns, change));
                    }
                });
            ui.horizontal(|ui| {
                if ui.button(t("Apply Changes")).clicked() {
                    apply = true;
                }
                if ui.button(t("Discard")).clicked() {
                    discard = true;
                }
                ui.weak(t("Discard keeps every staged edit; you can submit again."));
            });
        });
    if apply {
        app.apply_submission();
    }
    // A closed window (close box or the X) drops the journal — a pending
    // journal without its window would keep the grid locked invisibly.
    if discard || !open {
        app.discard_submission();
    }
}

/// One rendered journal change: the kind, the row and the differing
/// columns with their before → after values.
fn change_label(columns: &[String], change: &datacraft_core::journal::RowChange) -> String {
    let name = |index: usize| {
        columns
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("col {}", index + 1))
    };
    let kind = match change.kind {
        ChangeKind::Insert => t("Insert"),
        ChangeKind::Update => t("Update"),
        ChangeKind::Delete => t("Delete"),
    };
    let row_note = change
        .row
        .map(|row| format!(" #{}", row + 1))
        .unwrap_or_default();
    let diff = match (&change.before, &change.after) {
        (Some(before), Some(after)) => before
            .iter()
            .zip(after.iter())
            .enumerate()
            .filter(|(_, (old, new))| old != new)
            .map(|(index, (old, new))| {
                format!(
                    "{}: {} → {}",
                    name(index),
                    value_label(old),
                    value_label(new)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        (None, Some(after)) => after
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{}: {}", name(index), value_label(value)))
            .collect::<Vec<_>>()
            .join(", "),
        (Some(before), None) => before
            .iter()
            .map(value_label)
            .collect::<Vec<_>>()
            .join(", "),
        (None, None) => String::new(),
    };
    if diff.is_empty() {
        format!("{kind}{row_note}")
    } else {
        format!("{kind}{row_note} — {diff}")
    }
}
