//! App-level command flows (steps 11.3/11.4/11.6): table creation, object
//! deletion and the designer palettes compose existing bus commands; no
//! logic lives in paint code.

use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::ControlDef;
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::i18n::tr;

impl App {
    /// Creates a new table with the Access default shape (step 11.3) and
    /// opens it — the ribbon's New Table flow.
    pub fn create_table_flow(&mut self) {
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
            self.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
                datacraft_core::ObjectKind::Table,
                name,
            )));
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
            // Macros are a read-only inventory (step 17.1) — no delete.
            datacraft_core::ObjectKind::Macro | ObjectKind::Module => return,
        };
        let message = crate::i18n::tr("Delete {name}? This cannot be undone.", &[("name", name)]);
        if !self
            .dialogs
            .confirm(crate::i18n::t("Delete Object"), &message)
        {
            return;
        }
        self.dispatch(command);
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
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro | ObjectKind::Module => {
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
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro | ObjectKind::Module => {
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
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro | ObjectKind::Module => {
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
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro | ObjectKind::Module => None,
        }
    }

    /// Inserts the buffered new-record row of a datasheet tab (step 11.7).
    /// Cell parsing errors land in the status bar; a successful insert
    /// clears the buffer so the next record starts empty.
    pub fn insert_new_row(&mut self, tab_id: TabId, table: &str) -> bool {
        let parsed = self.table_definition(table).map(|definition| {
            let cells = self.new_row_cells(tab_id, definition.fields.len());
            (
                definition.name.clone(),
                datacraft_core::row_edit::row_values(&definition.fields, &cells),
            )
        });
        let Some((canonical, parsed)) = parsed else {
            self.last_error = Some(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return false;
        };
        let values = match parsed {
            Ok(values) => values,
            Err(reason) => {
                self.last_error = Some(reason);
                return false;
            }
        };
        let events = self.dispatch(Command::InsertRow {
            table: canonical,
            values,
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::RowInserted { .. }))
        {
            self.clear_new_row(tab_id);
            true
        } else {
            false
        }
    }

    /// Arms the datasheet's in-place cell editor on one cell of a table
    /// tab with the cell's current display text (step 40.4, item L23).
    /// A no-op for restricted sources — the grid never arms there.
    pub fn start_cell_edit(&mut self, tab_id: TabId, row: usize, col: usize) {
        if crate::availability::restricted(
            self.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_some()
        {
            return;
        }
        let buffer = self
            .rows_for(tab_id)
            .and_then(|rows| rows.get(row))
            .and_then(|values| values.get(col))
            .map(crate::labels::value_label)
            .unwrap_or_default();
        self.arm_cell_edit(tab_id, row, col, buffer);
    }

    /// Commits the armed in-place editor (step 40.4, item L23): the
    /// buffer is parsed against the field's kind, then exactly one
    /// `UpdateRow` dispatch writes the full positional value list. A
    /// parse failure keeps the editor armed with the buffer intact and
    /// reports the precise reason; a dispatch failure reports through
    /// the status bar and closes the editor.
    pub fn commit_cell_edit(&mut self, tab_id: TabId, table: &str, row: usize, col: usize) {
        let Some((_, _, buffer)) = self.cell_edit_parts(tab_id) else {
            return;
        };
        let text = buffer.clone();
        let parsed = self.table_definition(table).map(|definition| {
            (
                definition.name.clone(),
                definition.fields.get(col).map(|field| {
                    datacraft_core::row_edit::parse_value(&field.kind, &text)
                        .map_err(|reason| format!("field {:?}: {reason}", field.name))
                }),
                definition.fields.len(),
            )
        });
        let Some((canonical, parsed, arity)) = parsed else {
            self.last_error = Some(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return;
        };
        let value = match parsed {
            Some(Ok(value)) => value,
            Some(Err(reason)) => {
                self.last_error = Some(reason);
                return;
            }
            None => {
                self.last_error = Some(format!(
                    "field index {col} is out of range ({arity} fields)"
                ));
                return;
            }
        };
        let values = match self.rows_for(tab_id).and_then(|rows| rows.get(row)) {
            Some(row_values) => {
                let mut values = row_values.clone();
                if col >= values.len() {
                    self.last_error = Some(format!(
                        "field index {col} is out of range ({arity} fields)"
                    ));
                    return;
                }
                values[col] = value;
                values
            }
            None => {
                self.last_error = Some(format!("row {row} no longer exists in {canonical:?}"));
                return;
            }
        };
        let events = self.dispatch(Command::UpdateRow {
            table: canonical,
            row,
            values,
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::RowUpdated { .. }))
        {
            self.close_cell_edit(tab_id);
        }
    }

    /// Closes the armed in-place editor without committing (Esc / the
    /// grid's cancel intent).
    pub fn cancel_cell_edit(&mut self, tab_id: TabId) {
        self.close_cell_edit(tab_id);
    }

    /// Reason of the last failed command (status bar).
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Success notice of the last command (status bar), if any.
    pub fn last_notice(&self) -> Option<&str> {
        self.last_notice.as_deref()
    }
}

/// First free `prefixN` name (case-insensitive).
fn next_free_name(used: Vec<String>, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !used.contains(&name.to_lowercase()))
        .unwrap_or_else(|| prefix.to_owned())
}

/// Status-bar notice for successful design-document and database-copy
/// operations.
pub(crate) fn notice_for(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::DatabaseSavedAs { path, encrypted } => Some(if *encrypted {
            tr(
                "Copy saved to {path} (password-protected)",
                &[("path", &path.display().to_string())],
            )
        } else {
            tr(
                "Copy saved to {path}",
                &[("path", &path.display().to_string())],
            )
        }),
        Event::PasswordChanged { encrypted } => Some(if *encrypted {
            tr("Database password changed", &[])
        } else {
            tr("Database password removed", &[])
        }),
        Event::DesignDocumentSaved { path } => Some(tr(
            "Design saved to {path}",
            &[("path", &path.display().to_string())],
        )),
        Event::DesignDocumentLoaded {
            path,
            forms,
            queries,
            reports,
        } => Some(tr(
            "Design loaded from {path} ({forms} forms, {queries} queries, {reports} reports)",
            &[
                ("path", &path.display().to_string()),
                ("forms", &forms.to_string()),
                ("queries", &queries.to_string()),
                ("reports", &reports.to_string()),
            ],
        )),
        _ => None,
    })
}
