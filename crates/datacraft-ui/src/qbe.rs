//! QBE grid (step 7.2, item H26).
//!
//! Renders the Access-style query design grid from the snapshot and
//! dispatches `SetQbeCell` on commit (lost focus / Enter). Text buffers
//! are per-frame clones of the cell value — the snapshot is the single
//! source of truth.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::qbe::{QbeRow, QbeValue, SortOrder};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;

/// Renders the QBE grid of one query tab.
pub fn qbe_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(query) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.queries.iter().find(|q| q.name == tab.object_name))
        .cloned()
    else {
        ui.label("Query no longer exists");
        return;
    };

    ui.heading(egui::RichText::new(format!("Query: {}", query.name)));
    ui.label(
        egui::RichText::new(format!(
            "Source table: {} — set a table, then field columns.",
            if query.table.is_empty() {
                "<none>"
            } else {
                &query.table
            }
        ))
        .small(),
    );
    ui.separator();
    let mut run = false;
    let mut clear = false;
    ui.horizontal(|ui| {
        if ui.button("Run").clicked() {
            run = true;
        }
        if ui.button("Clear criteria").clicked() {
            clear = true;
        }
    });

    let mut intent: Option<(QbeRow, usize, QbeValue)> = None;
    egui::Grid::new("datacraft.qbe.grid")
        .num_columns(query.fields.len() + 1)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            // Table row.
            ui.weak("Table:");
            let mut buf = query.table.clone();
            let response = ui.add_sized(
                [220.0, 22.0],
                egui::TextEdit::singleline(&mut buf).hint_text("source table"),
            );
            if committed(&response) && buf != query.table {
                intent = Some((QbeRow::Table, 0, QbeValue::Text(buf)));
            }
            ui.end_row();

            // Field row.
            ui.weak("Field:");
            for col in 0..=query.fields.len() {
                let current = query
                    .fields
                    .get(col)
                    .map(|f| f.name.clone())
                    .unwrap_or_default();
                let mut buf = current.clone();
                let response = ui.add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut buf));
                if committed(&response) && buf != current && !buf.is_empty() {
                    intent = Some((QbeRow::Field, col, QbeValue::Text(buf)));
                }
            }
            ui.end_row();

            // Sort row.
            ui.weak("Sort:");
            for col in 0..query.fields.len() {
                let current = query.fields[col].sort;
                egui::ComboBox::from_id_salt(format!("qbe.sort.{col}"))
                    .selected_text(sort_label(current))
                    .show_ui(ui, |ui| {
                        for option in [SortOrder::None, SortOrder::Ascending, SortOrder::Descending]
                        {
                            if ui
                                .selectable_label(current == option, sort_label(option))
                                .clicked()
                            {
                                intent =
                                    Some((QbeRow::Sort, col, QbeValue::Text(option.to_string())));
                            }
                        }
                    });
            }
            ui.end_row();

            // Show row.
            ui.weak("Show:");
            for col in 0..query.fields.len() {
                let mut shown = query.fields[col].shown;
                if ui.checkbox(&mut shown, "").changed() {
                    intent = Some((QbeRow::Show, col, QbeValue::Bool(shown)));
                }
            }
            ui.end_row();

            // Criteria rows.
            let or_rows = query
                .fields
                .iter()
                .map(|f| f.criteria.len())
                .max()
                .unwrap_or(0)
                + 1;
            for or_row in 0..or_rows {
                ui.weak(if or_row == 0 { "Criteria:" } else { "or:" });
                for col in 0..query.fields.len() {
                    let current = query
                        .fields
                        .get(col)
                        .and_then(|f| f.criteria.get(or_row))
                        .cloned()
                        .unwrap_or_default();
                    let mut buf = current.clone();
                    let response =
                        ui.add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut buf));
                    if committed(&response) && buf != current {
                        intent = Some((QbeRow::Criteria(or_row), col, QbeValue::Text(buf)));
                    }
                }
                ui.end_row();
            }
        });

    ui.separator();
    if clear {
        app.dispatch(Command::ClearQbeCriteria {
            query: query.name.clone(),
        });
        return;
    }
    if run {
        app.dispatch(Command::RunQuery {
            query: query.name.clone(),
        });
    }
    if let Some((row, col, value)) = intent {
        app.dispatch(Command::SetQbeCell {
            query: query.name.clone(),
            row,
            col,
            value,
        });
    }
}

fn committed(response: &egui::Response) -> bool {
    response.lost_focus() || response.ctx.input(|i| i.key_pressed(egui::Key::Enter))
}

fn sort_label(order: SortOrder) -> &'static str {
    match order {
        SortOrder::None => "(not sorted)",
        SortOrder::Ascending => "ascending",
        SortOrder::Descending => "descending",
    }
}
