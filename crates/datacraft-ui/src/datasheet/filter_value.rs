//! The column value list (phase 101, step 101.2; item M132).
//!
//! Opened from the column header's context menu, the window shows the
//! column's distinct values as the engine listed them (bounded, ordered),
//! the blank entry only where the column carries a `Null` cell, and one
//! action per pick. The picks dispatch the bus commands — the window never
//! filters on its own.

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;
use crate::i18n::{t, tr};

/// The open value list: which column it belongs to and its entries
/// (filled by the `ColumnValuesListed` event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueList {
    /// Table the list was requested for.
    pub table: String,
    /// Column the list was requested for.
    pub column: String,
    /// Distinct rendered texts, as the engine ordered them.
    pub values: Vec<String>,
    /// Whether the column carries a `Null` cell.
    pub blank: bool,
    /// Whether more values exist than the list shows.
    pub truncated: bool,
}

/// What the window collected this frame.
enum Outcome {
    /// A value was picked (`None` = the blank entry).
    Pick(Option<String>),
    /// The column's filter was cleared.
    Clear,
    /// The window was closed without a pick.
    Close,
}

/// Renders the open value list, when one is open. The picks dispatch
/// after the window closure, so the borrow ends before the bus runs.
pub(crate) fn value_list_ui(app: &mut App, ctx: &egui::Context) {
    let Some(list) = app.value_filter.clone() else {
        return;
    };
    let mut outcome = None;
    egui::Window::new(tr("Filter by Value: {column}", &[("column", &list.column)]))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_min_width(240.0);
            if list.blank && ui.selectable_label(false, t("(Blanks)")).clicked() {
                outcome = Some(Outcome::Pick(None));
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for value in &list.values {
                        if ui.selectable_label(false, value.as_str()).clicked() {
                            outcome = Some(Outcome::Pick(Some(value.clone())));
                        }
                    }
                });
            if list.truncated {
                ui.weak(t("More values exist — narrow the column with a criteria."));
            }
            ui.separator();
            if ui.button(t("Clear filter")).clicked() {
                outcome = Some(Outcome::Clear);
            }
            if ui.button(t("Close")).clicked() {
                outcome = Some(Outcome::Close);
            }
        });
    match outcome {
        Some(Outcome::Pick(value)) => {
            app.value_filter = None;
            app.dispatch(Command::FilterByValue {
                table: list.table,
                field: list.column,
                value,
            });
        }
        Some(Outcome::Clear) => {
            app.value_filter = None;
            app.dispatch(Command::SetTableFilter {
                table: list.table,
                field: list.column,
                criteria: None,
            });
        }
        Some(Outcome::Close) => app.value_filter = None,
        None => {}
    }
}

impl App {
    /// Stores the value list a `ColumnValuesListed` event carried (phase
    /// 101, step 101.2; item M132): the request and its answer travel on
    /// the bus, the window only renders what the event delivered.
    pub(crate) fn capture_value_list(&mut self, events: &[datacraft_core::event::Event]) {
        for event in events {
            if let datacraft_core::event::Event::ColumnValuesListed {
                table,
                field,
                values,
                blank,
                truncated,
            } = event
            {
                self.value_filter = Some(ValueList {
                    table: table.clone(),
                    column: field.clone(),
                    values: values.clone(),
                    blank: *blank,
                    truncated: *truncated,
                });
            }
        }
    }
}
