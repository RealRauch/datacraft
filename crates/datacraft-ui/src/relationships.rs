//! Relationship mapper pane (step 7.1, item H25).
//!
//! A floating window listing the snapshot's relationships and painting
//! the table boxes with connection lines. Pure paint: the data comes from
//! the snapshot, toggling dispatches the command.

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;

/// Box geometry for the canvas.
const BOX_SIZE: egui::Vec2 = egui::Vec2::new(150.0, 64.0);
const BOX_GAP: f32 = 28.0;
const PER_ROW: usize = 3;

/// Renders the floating relationship mapper (when open).
pub fn mapper_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().relationship_mapper_open {
        return;
    }
    let snapshot = app.snapshot().clone();
    let Some(db) = snapshot.database.clone() else {
        return;
    };
    let mut close = false;
    egui::Window::new("Relationships")
        .id(egui::Id::unique("datacraft.relationships"))
        .default_width(640.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(format!("{} relationships", db.relationships.len()));
                if ui.small_button("Close").clicked() {
                    close = true;
                }
            });
            ui.separator();

            // Textual list.
            for rel in &db.relationships {
                ui.label(format!(
                    "{}.{}  →  {}.{}",
                    rel.table, rel.column, rel.related_table, rel.related_column
                ));
            }
            if db.relationships.is_empty() {
                ui.weak("This database has no relationships.");
            }
            ui.separator();

            // Canvas: deterministic box layout, lines between related tables.
            let tables: Vec<&String> = {
                let mut names: Vec<&String> = db
                    .relationships
                    .iter()
                    .flat_map(|r| [&r.table, &r.related_table])
                    .collect();
                names.sort_by_key(|a| a.to_lowercase());
                names.dedup();
                names
            };
            let cols = PER_ROW;
            let rows = tables.len().div_ceil(cols).max(1);
            let (canvas_rect, response) = ui.allocate_exact_size(
                egui::vec2(
                    cols as f32 * (BOX_SIZE.x + BOX_GAP),
                    rows as f32 * (BOX_SIZE.y + BOX_GAP),
                ),
                egui::Sense::hover(),
            );
            let box_rect = |i: usize| -> egui::Rect {
                let col = i % cols;
                let row = i / cols;
                egui::Rect::from_min_size(
                    canvas_rect.min
                        + egui::vec2(
                            col as f32 * (BOX_SIZE.x + BOX_GAP),
                            row as f32 * (BOX_SIZE.y + BOX_GAP),
                        ),
                    BOX_SIZE,
                )
            };
            let index_of = |name: &String| tables.iter().position(|t| t.eq_ignore_ascii_case(name));
            let stroke = ui.visuals().widgets.noninteractive.fg_stroke;

            // Lines first (under the boxes).
            for rel in &db.relationships {
                let (Some(a), Some(b)) = (index_of(&rel.table), index_of(&rel.related_table))
                else {
                    continue;
                };
                let (ra, rb) = (box_rect(a), box_rect(b));
                ui.painter()
                    .line_segment([ra.center(), rb.center()], stroke);
            }
            for (i, name) in tables.iter().enumerate() {
                let rect = box_rect(i);
                ui.painter()
                    .rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                ui.painter()
                    .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
                ui.painter().text(
                    rect.left_top() + egui::vec2(6.0, 6.0),
                    egui::Align2::LEFT_TOP,
                    name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    stroke.color,
                );
            }
            let _ = response;
        });
    if close {
        app.dispatch(Command::ToggleRelationshipMapper);
    }
}
