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
use crate::i18n::{plural, t, tr};

/// Renders the QBE grid of one query tab (native saved queries render
/// read-only — step 15.3, M3).
pub fn qbe_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    if app.native_query(&tab.object_name).is_some() {
        native_query_ui(app, tab, ui);
        return;
    }
    let Some(query) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.queries.iter().find(|q| q.name == tab.object_name))
        .cloned()
    else {
        ui.label(t("Query no longer exists"));
        return;
    };

    ui.heading(egui::RichText::new(tr(
        "Query: {name}",
        &[("name", &query.name)],
    )));
    let source = if query.table.is_empty() {
        t("<none>")
    } else {
        query.table.as_str()
    };
    ui.label(
        egui::RichText::new(tr(
            "Source table: {table} — set a table, then field columns.",
            &[("table", source)],
        ))
        .small(),
    );
    ui.separator();
    let mut run = false;
    let mut clear = false;
    ui.horizontal(|ui| {
        if ui.button(t("Run")).clicked() {
            run = true;
        }
        if ui.button(t("Clear criteria")).clicked() {
            clear = true;
        }
    });

    let mut intent: Option<(QbeRow, usize, QbeValue)> = None;
    egui::Grid::new("datacraft.qbe.grid")
        .num_columns(query.fields.len() + 1)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            // Table row.
            ui.weak(t("Table:"));
            let mut buf = query.table.clone();
            let response = ui.add_sized(
                [220.0, 22.0],
                egui::TextEdit::singleline(&mut buf).hint_text(t("source table")),
            );
            if committed(&response) && buf != query.table {
                intent = Some((QbeRow::Table, 0, QbeValue::Text(buf)));
            }
            ui.end_row();

            // Field row.
            ui.weak(t("Field:"));
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
            ui.weak(t("Sort:"));
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
            ui.weak(t("Show:"));
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
                ui.weak(if or_row == 0 {
                    t("Criteria:")
                } else {
                    t("or:")
                });
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
        SortOrder::None => t("(not sorted)"),
        SortOrder::Ascending => t("ascending"),
        SortOrder::Descending => t("descending"),
    }
}

/// Read-only view of a native saved query (M3): flavor, tables, columns
/// and the decoded expressions; `Run` executes the mappable subset.
fn native_query_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(native) = app.native_query(&tab.object_name) else {
        ui.label(t("Query no longer exists"));
        return;
    };
    let name = native.name.clone();
    let kind = format!("{:?}", native.kind);
    let tables = native.tables.join(", ");
    let columns: Vec<String> = native
        .columns
        .iter()
        .map(
            |column| match (&column.table, &column.field, &column.expression) {
                (_, _, Some(expression)) => expression.clone(),
                (Some(table), Some(field), None) => format!("{table}.{field}"),
                (None, Some(field), None) => field.clone(),
                _ => t("<column>").to_owned(),
            },
        )
        .collect();
    let criteria = native.criteria.join(" AND ");
    let joins = native
        .joins
        .iter()
        .map(|join| join.expression.clone())
        .collect::<Vec<_>>()
        .join("; ");
    let order_by = native
        .order_by
        .iter()
        .map(|order| {
            if order.descending {
                format!("{} DESC", order.field)
            } else {
                order.field.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    ui.heading(egui::RichText::new(tr(
        "Native query: {name}",
        &[("name", &name)],
    )));
    ui.label(
        egui::RichText::new(tr("{kind} — read-only (MSysQueries)", &[("kind", &kind)]))
            .small()
            .weak(),
    );
    ui.separator();
    let tables_text = if tables.is_empty() {
        t("<none>").to_owned()
    } else {
        tables.clone()
    };
    ui.weak(tr("Tables: {tables}", &[("tables", &tables_text)]));
    for column in &columns {
        ui.label(column.clone());
    }
    if !criteria.is_empty() {
        ui.weak(tr("Criteria: {criteria}", &[("criteria", &criteria)]));
    }
    if !joins.is_empty() {
        ui.weak(tr("Joins: {joins}", &[("joins", &joins)]));
    }
    if !order_by.is_empty() {
        ui.weak(tr("Order by: {order_by}", &[("order_by", &order_by)]));
    }
    ui.separator();
    if ui.button(t("Run")).clicked() {
        let events = app.dispatch(Command::RunQuery {
            query: name.clone(),
        });
        let ran = events.iter().find_map(|event| match event {
            datacraft_core::event::Event::QueryRan { rows, .. } => Some(*rows),
            _ => None,
        });
        ui.weak(match ran {
            Some(rows) => tr(
                "Ran — {rows}",
                &[("rows", &plural(rows, "{count} row", "{count} rows"))],
            ),
            None => t("Run failed — see the status line").to_owned(),
        });
    }
}
