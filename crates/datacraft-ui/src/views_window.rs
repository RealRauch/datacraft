//! View tabs and the view editor (step 71.2, item M93).
//!
//! A view tab renders its SELECT result read-only (the cache is filled in
//! `maintain_caches`); the editor creates and updates sidecar views through
//! the bus. The tab toolbar carries Edit and Delete.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::grid::GridMetrics;
use datacraft_core::ids::TabId;
use datacraft_core::snapshot::TabSnapshot;
use datacraft_core::sql_types::SqlResult;
use datacraft_core::view::ViewCommand;

use crate::app::App;
use crate::i18n::{plural, t, tr};
use crate::labels::value_label;
use crate::theme::tokens;

/// The open editor's draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewDraft {
    /// Current name text.
    pub name: String,
    /// Current SQL text.
    pub sql: String,
    /// The view being edited (`None` while creating).
    pub original: Option<String>,
}

impl App {
    /// Opens the editor, empty or on an existing view.
    pub fn open_view_editor(&mut self, name: Option<&str>) {
        let existing = name.and_then(|name| {
            self.snapshot()
                .database
                .as_ref()
                .and_then(|db| {
                    db.views
                        .iter()
                        .find(|view| view.name.eq_ignore_ascii_case(name))
                })
                .map(|view| ViewDraft {
                    name: view.name.clone(),
                    sql: view.sql.clone(),
                    original: Some(view.name.clone()),
                })
        });
        self.view_draft = Some(existing.unwrap_or(ViewDraft {
            name: String::new(),
            sql: "SELECT ".to_owned(),
            original: None,
        }));
    }

    /// The open editor draft.
    pub fn view_draft(&self) -> Option<&ViewDraft> {
        self.view_draft.as_ref()
    }

    /// Sets the draft name.
    pub fn set_view_draft_name(&mut self, name: &str) {
        if let Some(draft) = &mut self.view_draft {
            draft.name = name.to_owned();
        }
    }

    /// Sets the draft SQL.
    pub fn set_view_draft_sql(&mut self, sql: &str) {
        if let Some(draft) = &mut self.view_draft {
            draft.sql = sql.to_owned();
        }
    }

    /// Dispatches the draft: create or replace the SQL; a failure keeps the
    /// editor open with the buffer intact.
    pub fn save_view_draft(&mut self) {
        let Some(draft) = self.view_draft.clone() else {
            return;
        };
        let command = match &draft.original {
            Some(original) => Command::View(ViewCommand::SetSql {
                name: original.clone(),
                sql: draft.sql.clone(),
            }),
            None => Command::View(ViewCommand::Create {
                name: draft.name.clone(),
                sql: draft.sql.clone(),
            }),
        };
        let events = self.dispatch(command);
        if !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
        {
            self.view_draft = None;
        }
    }

    /// Closes the editor without saving.
    pub fn close_view_editor(&mut self) {
        self.view_draft = None;
    }

    /// The cached SELECT result of an open view tab.
    pub fn view_result(&self, tab_id: TabId) -> Option<&SqlResult> {
        self.view_results.get(&tab_id)
    }

    /// Deletes a view after the native confirmation.
    pub fn delete_view(&mut self, name: &str) {
        let message = tr("Delete {name}? This cannot be undone.", &[("name", name)]);
        if !self.dialogs.confirm(t("Delete Object"), &message) {
            return;
        }
        self.dispatch(Command::View(ViewCommand::Delete {
            name: name.to_owned(),
        }));
    }
}

/// The view tab: toolbar plus the read-only result grid.
pub fn view_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    // File-owned views (M90) are read-only: the design and delete
    // affordances belong to the sidecar authority only.
    let native = app.snapshot().database.as_ref().is_some_and(|db| {
        db.native_views
            .iter()
            .any(|view| view.name.eq_ignore_ascii_case(&tab.object_name))
    });
    if native {
        let hint = ui.weak(t("Read-only native view"));
        // Static text is culled from the semantic tree; the rect probe
        // pins its presence (step 44.1 seam, M90).
        crate::layout_probe::record("view.native_hint", hint.rect);
    } else {
        ui.horizontal(|ui| {
            if ui.small_button(t("Edit SQL")).clicked() {
                app.open_view_editor(Some(&tab.object_name));
            }
            if ui.small_button(t("Delete")).clicked() {
                app.delete_view(&tab.object_name);
            }
        });
    }
    if let Some(reason) = app.load_error(tab.id) {
        ui.colored_label(tokens.danger, reason);
        return;
    }
    let Some(result) = app.view_result(tab.id) else {
        ui.weak(t("No result yet."));
        return;
    };
    let body: Vec<Vec<String>> = result
        .rows
        .iter()
        .map(|row| row.iter().map(value_label).collect())
        .collect();
    let mut widths: Vec<f32> = Vec::new();
    crate::datasheet::grid_ui(
        ui,
        crate::datasheet::Grid {
            metrics: GridMetrics::default(),
            columns: &result.columns,
            body: &body,
            cursor: 0,
            widths: &mut widths,
            sort_orders: &[],
            filters: &std::collections::BTreeMap::new(),
            id_salt: format!("view.grid.{}", tab.ui_test_id),
            frozen: 0,
            delete_reason: None,
            edit: None,
            edit_restricted: true,
        },
        crate::datasheet::grid_ui_callbacks(&mut |_row: usize| {}, None, None, None, None, None),
    );
    ui.label(
        egui::RichText::new(plural(
            result.rows.len(),
            "{count} result row",
            "{count} result rows",
        ))
        .small(),
    );
}

/// The view editor window.
pub fn view_editor(app: &mut App, ui: &mut egui::Ui) {
    let Some(draft) = app.view_draft().cloned() else {
        return;
    };
    let mut name = draft.name.clone();
    let mut sql = draft.sql.clone();
    let mut open = true;
    let mut save = false;
    let mut cancel = false;
    egui::Window::new(t("View"))
        .open(&mut open)
        .resizable(true)
        .default_width(460.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(460.0, 340.0),
        ))
        .show(ui.ctx(), |ui| {
            ui.label(t("View name"));
            ui.text_edit_singleline(&mut name);
            ui.label(t("SQL"));
            ui.add(
                egui::TextEdit::multiline(&mut sql)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
            ui.horizontal(|ui| {
                if ui.button(t("Save")).clicked() {
                    save = true;
                }
                if ui.button(t("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    if save {
        app.set_view_draft_name(&name);
        app.set_view_draft_sql(&sql);
        app.save_view_draft();
    } else if cancel || !open {
        app.close_view_editor();
    }
}
