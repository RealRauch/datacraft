//! App-level command flows (steps 11.3/11.4/11.6): table creation, object
//! deletion and the designer palettes compose existing bus commands; no
//! logic lives in paint code.

use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::ControlDef;
use datacraft_core::ids::TabId;

use crate::app::App;

impl App {
    /// Creates a new table with the Access default shape (step 11.3) and
    /// opens it — the ribbon's New Table flow.
    pub fn create_table_flow(&mut self) {
        if let Some(name) = self.create_default_table() {
            self.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
                datacraft_core::ObjectKind::Table,
                name,
            )));
        }
    }

    /// Creates a new table with the Access default shape and opens it in
    /// the design view (the ribbon's Table Design flow, step 41.5, item
    /// M49) — the same design flow the nav pane's Design menu dispatches.
    pub fn create_table_design_flow(&mut self) {
        if let Some(name) = self.create_default_table() {
            self.dispatch(Command::OpenObjectInMode {
                object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Table, name),
                mode: datacraft_core::state::TabMode::Design,
            });
        }
    }

    /// Creates a new form and opens its design view (step 69.3, item M80):
    /// the shell follows the create instead of leaving the object
    /// unreachable behind the ribbon.
    pub fn create_form_flow(&mut self) {
        let name = crate::ribbon::next_form_name(self);
        let events = self.dispatch(Command::CreateForm { name: name.clone() });
        if events
            .iter()
            .any(|event| matches!(event, Event::FormCreated { .. }))
        {
            self.dispatch(Command::OpenObjectInMode {
                object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Form, name),
                mode: datacraft_core::state::TabMode::Design,
            });
        }
    }

    /// Creates a new report and opens its design view (step 69.3, M80).
    pub fn create_report_flow(&mut self) {
        let name = crate::ribbon::next_report_name(self);
        let events = self.dispatch(Command::CreateReport { name: name.clone() });
        if events
            .iter()
            .any(|event| matches!(event, Event::ReportCreated { .. }))
        {
            self.dispatch(Command::OpenObjectInMode {
                object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Report, name),
                mode: datacraft_core::state::TabMode::Design,
            });
        }
    }

    /// Creates a new query and opens its design view (step 69.3, M80).
    pub fn create_query_flow(&mut self) {
        let name = crate::ribbon::next_query_name(self);
        let events = self.dispatch(Command::CreateQuery { name: name.clone() });
        if events
            .iter()
            .any(|event| matches!(event, Event::QueryCreated { .. }))
        {
            self.dispatch(Command::OpenObjectInMode {
                object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Query, name),
                mode: datacraft_core::state::TabMode::Design,
            });
        }
    }

    /// Creates one Access-default table with the next free name; the
    /// created name on success, `None` when the dispatch failed.
    fn create_default_table(&mut self) -> Option<String> {
        let name = next_free_name(
            self.snapshot()
                .database
                .as_ref()
                .map(|db| db.tables.iter().map(|t| t.name.to_lowercase()).collect())
                .unwrap_or_default(),
            "Table",
        );
        let events = self.dispatch(Command::CreateTable {
            definition: datacraft_core::schema::TableDef::access_default(name.clone()),
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::TableCreated { .. }))
        {
            Some(name)
        } else {
            None
        }
    }

    /// Deletes a database object through the bus (step 11.4) after a native
    /// confirmation (step 38.3, item M43) — a declined dialog dispatches
    /// nothing.
    pub fn delete_object(&mut self, kind: datacraft_core::ObjectKind, name: &str) {
        let command = match kind {
            datacraft_core::ObjectKind::Table => Command::DropTable {
                table: name.to_owned(),
            },
            datacraft_core::ObjectKind::Query => Command::DeleteQuery {
                name: name.to_owned(),
            },
            datacraft_core::ObjectKind::Form => Command::DeleteForm {
                name: name.to_owned(),
            },
            datacraft_core::ObjectKind::Report => Command::DeleteReport {
                name: name.to_owned(),
            },
            datacraft_core::ObjectKind::View => {
                Command::View(datacraft_core::view::ViewCommand::Delete {
                    name: name.to_owned(),
                })
            }
            // Macros are a read-only inventory (step 17.1) — no delete.
            datacraft_core::ObjectKind::Macro | ObjectKind::Module => return,
        };
        let message = crate::i18n::tr("Delete {name}? This cannot be undone.", &[("name", name)]);
        // The confirmation parks and dispatches on a later frame (step
        // 81.2, item M97); a declined dialog deletes nothing.
        self.request_dialog(
            crate::dialogs::DialogRequest::Confirm {
                title: crate::i18n::t("Delete Object").to_owned(),
                message,
            },
            crate::app::dialog_flow::DialogIntent::DeleteObject { command },
        );
    }

    /// Assigns a form's or report's record source (step 11.6) — the
    /// designer palette's source selector.
    pub fn set_record_source(&mut self, kind: ObjectKind, object: &str, table: &str) {
        let command = match kind {
            ObjectKind::Form => Command::SetFormSource {
                form: object.to_owned(),
                table: table.to_owned(),
            },
            ObjectKind::Report => Command::SetReportSource {
                report: object.to_owned(),
                table: table.to_owned(),
            },
            ObjectKind::Table
            | ObjectKind::Query
            | ObjectKind::View
            | ObjectKind::Macro
            | ObjectKind::Module => {
                return;
            }
        };
        self.dispatch(command);
    }

    /// Adds a record-source field from the designer's field list (step
    /// 11.6) as a bound control at the next free slot.
    pub fn add_designer_field(&mut self, kind: ObjectKind, object: &str, field: &str) {
        let command = match kind {
            ObjectKind::Form => Command::AddFormField {
                form: object.to_owned(),
                field: field.to_owned(),
            },
            ObjectKind::Report => Command::AddReportField {
                report: object.to_owned(),
                field: field.to_owned(),
            },
            ObjectKind::Table
            | ObjectKind::Query
            | ObjectKind::View
            | ObjectKind::Macro
            | ObjectKind::Module => {
                return;
            }
        };
        self.dispatch(command);
    }

    /// Adds a toolbox control at the next free slot (step 11.6). Name,
    /// default kind and placement come from the core palette helpers.
    pub fn add_toolbox_control(&mut self, kind: ObjectKind, object: &str, entry_id: &str) {
        let Some((grid, bounds, controls)) = self.design_surface(kind, object) else {
            return;
        };
        let Some(control) =
            datacraft_core::palette::place_toolbox_control(entry_id, grid, bounds, &controls)
        else {
            return;
        };
        let command = match kind {
            ObjectKind::Form => Command::AddFormControl {
                form: object.to_owned(),
                control,
            },
            ObjectKind::Report => Command::AddReportControl {
                report: object.to_owned(),
                control,
            },
            ObjectKind::Table
            | ObjectKind::Query
            | ObjectKind::View
            | ObjectKind::Macro
            | ObjectKind::Module => {
                return;
            }
        };
        self.dispatch(command);
    }

    /// Grid, bounds and placed controls of a design surface.
    fn design_surface(
        &self,
        kind: ObjectKind,
        object: &str,
    ) -> Option<(f32, (f32, f32), Vec<ControlDef>)> {
        let db = self.snapshot().database.as_ref()?;
        match kind {
            ObjectKind::Form => {
                let def = db
                    .forms
                    .iter()
                    .find(|form| form.name.eq_ignore_ascii_case(object))?;
                Some((
                    def.grid,
                    (def.size.width, def.size.height),
                    def.controls.clone(),
                ))
            }
            ObjectKind::Report => {
                let def = db
                    .reports
                    .iter()
                    .find(|report| report.name.eq_ignore_ascii_case(object))?;
                Some((
                    def.grid,
                    (def.page.width, def.page.height),
                    def.controls.clone(),
                ))
            }
            ObjectKind::Table
            | ObjectKind::Query
            | ObjectKind::View
            | ObjectKind::Macro
            | ObjectKind::Module => None,
        }
    }
}

/// First free `prefixN` name (case-insensitive).
fn next_free_name(used: Vec<String>, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !used.contains(&name.to_lowercase()))
        .unwrap_or_else(|| prefix.to_owned())
}

impl App {
    /// The content draft of one designer control (step 63.2, item H56).
    pub fn control_content_draft(&self, tab_id: TabId, control: &str) -> Option<&str> {
        self.control_content_drafts
            .get(&(tab_id, control.to_owned()))
            .map(String::as_str)
    }

    /// Replaces the content draft of one designer control.
    pub fn set_control_content_draft(&mut self, tab_id: TabId, control: &str, text: String) {
        self.control_content_drafts
            .insert((tab_id, control.to_owned()), text);
    }

    /// Clears every content draft of one tab (selection and after apply).
    pub fn clear_control_content_draft(&mut self, tab_id: TabId) {
        self.control_content_drafts
            .retain(|(tab, _), _| *tab != tab_id);
    }

    /// The expression window's picked row context (step 60.1, item L27;
    /// moved from `app.rs` for the size gate): the table plus the
    /// zero-based row as typed text.
    pub fn expression_context(&self) -> Option<&(String, String)> {
        self.expression_context.as_ref()
    }

    /// Replaces the expression window's picked row context.
    pub fn set_expression_context(&mut self, context: Option<(String, String)>) {
        self.expression_context = context;
    }

    /// Runs the editor's expression through the bus and caches the
    /// outcome (step 41.2, item M46; moved from `app.rs` for the size
    /// gate). The picked context resolves the row at run time (step 60.1,
    /// item L27); an unparsable row number answers the localized failure
    /// instead of silently dropping the context.
    pub fn run_expression(&mut self) {
        let context = self
            .expression_context
            .as_ref()
            .and_then(|(table, row)| row.parse::<usize>().ok().map(|row| (table.clone(), row)));
        if self.expression_context.is_some() && context.is_none() {
            self.expression_output = None;
            self.expression_error =
                Some(crate::i18n::t("the row number must be a whole number (0-based)").to_owned());
            return;
        }
        let events = self.dispatch(Command::RunExpression {
            context: context
                .map(|(table, row)| datacraft_core::command::ExpressionContext { table, row }),
            expression: self.expression_text.clone(),
        });
        self.expression_output = None;
        self.expression_error = None;
        for event in events {
            match event {
                Event::ExpressionEvaluated { result } => self.expression_output = Some(result),
                Event::Failed { code, reason } => {
                    self.expression_error = Some(crate::failure_text::failure_text(code, &reason))
                }
                _ => {}
            }
        }
    }

    /// The Import File window's source path buffer (step 58.1, item M50;
    /// moved from `app.rs` for the size gate).
    pub fn import_source(&self) -> &str {
        &self.import_source
    }

    /// Replaces the Import File window's source path buffer.
    pub fn set_import_source(&mut self, text: impl Into<String>) {
        self.import_source = text.into();
    }

    /// The Import File window's target table buffer.
    pub fn import_table(&self) -> &str {
        &self.import_table
    }

    /// Replaces the Import File window's target table buffer.
    pub fn set_import_table(&mut self, text: impl Into<String>) {
        self.import_table = text.into();
    }
}
