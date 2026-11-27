//! The cell value viewer (step 86.4, item M102; Access's Zoom).
//!
//! A read-only zoom window on one cell: JSON indents only when it
//! parses, binary and GUID values show a bounded hex view that names the
//! truncation, everything else renders its display label. For an
//! editable long-text field the viewer becomes a multiline editor that
//! writes through the normal edit path — the H61 batch on an
//! atomic-batch driver, one `UpdateRow` otherwise. Opening never
//! modifies the database.

use eframe::egui;

use datacraft_core::schema::{FieldKind, FieldValue};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::labels::value_label_for;

/// Hex-view bound: at most this many bytes render; the window names the
/// truncation instead of silently clipping.
const MAX_HEX_BYTES: usize = 512;

/// Renders the tab's open value viewer, if any.
#[allow(clippy::too_many_arguments)]
pub(super) fn value_viewer_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    slots: &[usize],
    ui: &mut egui::Ui,
) {
    let Some((row, col)) = app.value_viewer(tab.id) else {
        return;
    };
    let Some(field_name) = columns.get(col).cloned() else {
        app.close_value_viewer(tab.id);
        return;
    };
    let Some(slot) = slots.get(row).copied() else {
        app.close_value_viewer(tab.id);
        return;
    };
    let Some(value) = app.cell_value(tab.id, slot, col) else {
        app.close_value_viewer(tab.id);
        return;
    };
    let value = value.clone();
    // The multiline editor exists only for editable long-text fields on a
    // writable source; a submission in flight keeps the viewer read-only.
    let locked = app.snapshot().pending_write.is_some();
    let editable = !locked
        && crate::availability::restricted(
            app.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_none()
        && matches!(value, FieldValue::Text(_))
        && app
            .table_definition(&tab.object_name)
            .and_then(|definition| definition.fields.get(col))
            .is_some_and(|field| matches!(field.kind, FieldKind::Text { .. } | FieldKind::Memo));
    let title = tr("Value — {field}", &[("field", &field_name)]);
    // The column's kind feeds the rendering: a decimal shows its declared
    // scale (phase 104, step 104.1; M135).
    let kind = app
        .table_definition(&tab.object_name)
        .and_then(|definition| definition.fields.get(col))
        .map(|field| field.kind);
    // `Window::open` expects the flag to read `true` while the window is
    // visible; its close box flips it (the notices-dialog pattern).
    let mut open = true;
    let mut close_now = false;
    let mut apply = false;
    let mut draft = app
        .value_editor_draft(tab.id)
        .unwrap_or_else(|| value_label_for(&value, kind.as_ref()));
    let window_id = format!("datacraft.zoom.{}", tab.ui_test_id);
    egui::Window::new(title)
        .open(&mut open)
        .id(egui::Id::unique(window_id.as_str()))
        .default_width(420.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(420.0, 320.0),
        ))
        .show(ui, |ui| {
            match &value {
                FieldValue::Binary(bytes) => {
                    ui.monospace(hex_view(bytes));
                    if bytes.len() > MAX_HEX_BYTES {
                        ui.weak(tr(
                            "Showing the first {shown} of {total} bytes",
                            &[
                                ("shown", &MAX_HEX_BYTES.to_string()),
                                ("total", &bytes.len().to_string()),
                            ],
                        ));
                    }
                }
                FieldValue::Guid(bytes) => {
                    ui.monospace(hex_view(bytes));
                }
                _ if editable => {
                    let label = ui.label(t("Cell value"));
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut draft)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(6)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label.id);
                        });
                    ui.horizontal(|ui| {
                        if ui.button(t("Apply")).clicked() {
                            apply = true;
                        }
                        if ui.button(t("Close")).clicked() {
                            close_now = true;
                        }
                    });
                }
                _ => {
                    let text = display_text(&value, kind.as_ref());
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            // JSON indents only when it parses; invalid
                            // JSON stays raw (M102).
                            if is_json(&text) {
                                let pretty = serde_json::from_str::<serde_json::Value>(&text)
                                    .map(|parsed| {
                                        serde_json::to_string_pretty(&parsed)
                                            .expect("a parsed value serializes")
                                    })
                                    .unwrap_or_else(|_| text.clone());
                                ui.monospace(pretty);
                            } else {
                                ui.monospace(&text);
                            }
                        });
                }
            }
        });
    if apply {
        let text = draft.clone();
        app.set_value_editor_draft(tab.id, text);
        app.commit_value_editor(tab.id, &tab.object_name, row, slot, col);
        app.close_value_viewer(tab.id);
    }
    // Esc closes without committing (the editor applies only through
    // Apply — a click-away commit would fight the grid's own Esc path).
    let esc = ui.input(|input| input.key_pressed(egui::Key::Escape));
    if !open || close_now || esc {
        app.close_value_viewer(tab.id);
    }
}

/// The read-only display text: JSON pretty-printed when it parses,
/// otherwise the display label verbatim.
fn display_text(value: &FieldValue, kind: Option<&FieldKind>) -> String {
    let text = value_label_for(value, kind);
    if is_json(&text) {
        serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|parsed| serde_json::to_string_pretty(&parsed).ok())
            .unwrap_or(text)
    } else {
        text
    }
}

/// Whether the text looks like a JSON document and parses.
fn is_json(text: &str) -> bool {
    let trimmed = text.trim_start();
    (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(text).is_ok()
}

/// The bounded hex view: `DE AD BE EF …` byte pairs, four pairs per line.
fn hex_view(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().take(MAX_HEX_BYTES).enumerate() {
        if index > 0 {
            if index % 16 == 0 {
                out.push('\n');
            } else {
                out.push(' ');
            }
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON detection accepts objects and arrays and rejects lookalikes.
    #[test]
    fn json_detection_needs_a_parse() {
        assert!(is_json("{\"a\": 1}"));
        assert!(is_json("  [1, 2]"));
        assert!(!is_json("{not json"));
        assert!(!is_json("123"));
        assert!(!is_json("just text"));
    }

    /// The hex view bounds its output and formats byte pairs.
    #[test]
    fn the_hex_view_bounds_and_formats() {
        assert_eq!(hex_view(&[0xDE, 0xAD]), "DE AD");
        let big: Vec<u8> = vec![0x41; 600];
        let view = hex_view(&big);
        let lines = view.lines().count();
        assert_eq!(lines, 32, "512 bytes = 32 lines of 16");
        assert!(!view.contains("0x"));
    }

    /// The truncation note names both numbers.
    #[test]
    fn the_truncation_note_names_both_numbers() {
        let note = tr(
            "Showing the first {shown} of {total} bytes",
            &[("shown", "512"), ("total", "600")],
        );
        assert!(note.contains("512") && note.contains("600"));
    }
}
