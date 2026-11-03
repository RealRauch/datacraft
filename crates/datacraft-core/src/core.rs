//! Command dispatch core (step 4.1, item H13; ARCHITECTURE §5).
//!
//! `Core::dispatch` is a total reducer: every command produces a
//! deterministic event list, failures become `Event::Failed` with precise
//! reasons. Storage I/O happens only behind the [`StoragePort`](crate::port)
//! — the reducer itself is pure over [`AppState`].

use crate::command::Command;
use crate::event::Event;
use crate::port::{PortError, StoragePort};
use crate::snapshot::UiStateSnapshot;
use crate::state::AppState;

mod relationships;
mod tabs;

/// The headless application core: reducer + state + storage port.
pub struct Core {
    pub(crate) state: AppState,
    pub(crate) port: Box<dyn StoragePort>,
}

impl Core {
    /// Creates a core over the given storage backend.
    pub fn new(port: Box<dyn StoragePort>) -> Self {
        Self {
            state: AppState::new(),
            port,
        }
    }

    /// Read access to the application state (the UI re-renders from it).
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Serializable snapshot of everything the UI shows — the headless
    /// test seam (ARCHITECTURE §5).
    pub fn inspect(&self) -> UiStateSnapshot {
        UiStateSnapshot::of(&self.state)
    }

    /// Executes a query (in-memory predicates over port rows, decision
    /// default 7); errors are precise strings for headless rendering.
    pub fn run_query(&self, name: &str) -> Result<crate::query_engine::QueryResult, String> {
        let Some(db) = &self.state.database else {
            return Err("no database is open".to_owned());
        };
        let Some(def) = db
            .queries
            .iter()
            .find(|q| q.name.eq_ignore_ascii_case(name))
            .cloned()
        else {
            let native = db
                .native_queries
                .iter()
                .find(|query| query.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("query {name:?} not found"))?;
            // Native queries execute through the qualified dialect
            // (step 27.1, M12): the decoded expression dialect runs on the
            // phase-15 expression service; anything outside it answers a
            // precise `Unsupported`.
            let mut sources = Vec::with_capacity(native.tables.len());
            for table in &native.tables {
                let Some(source) = db
                    .tables
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
                else {
                    return Err(format!(
                        "native query {:?}: table {table:?} not found",
                        native.name
                    ));
                };
                let rows = self
                    .port
                    .read_rows(
                        &source.name,
                        crate::port::RowRange {
                            start: 0,
                            len: usize::MAX,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                sources.push((source, rows));
            }
            return crate::native_exec::run(native, &sources).map_err(|e| e.to_string());
        };
        let Some(source) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&def.table))
        else {
            return Err(format!("source table {:?} not found", def.table));
        };
        let rows = self
            .port
            .read_rows(
                &source.name,
                crate::port::RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .map_err(|e| e.to_string())?;
        crate::query_engine::run_query(&def, source, &rows).map_err(|e| e.to_string())
    }

    /// A native saved query by name (step 15.3, M3) — the read-only
    /// consumer seam (bus/MCP/UI) for queries the sidecar does not own.
    pub fn native_query(&self, name: &str) -> Option<&crate::port::NativeQuery> {
        self.state
            .database
            .as_ref()?
            .native_queries
            .iter()
            .find(|query| query.name.eq_ignore_ascii_case(name))
    }

    /// Runs one gated SQL statement over the loaded tables in a throwaway
    /// in-memory database (step 13.3, H37) — the data seam of the SQL
    /// window. Sources are read through the port and never written.
    #[cfg(feature = "sql-engine")]
    pub fn run_sql(&self, sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        let Some(db) = &self.state.database else {
            return Err("no database is open".to_owned());
        };
        let mut sources = Vec::with_capacity(db.tables.len());
        for table in &db.tables {
            let rows = self
                .port
                .read_rows(
                    &table.name,
                    crate::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .map_err(|e| format!("materializing {:?} failed: {e}", table.name))?;
            sources.push((table.clone(), rows));
        }
        crate::sql_engine::run_sql(sql, &sources)
    }

    /// Without the `sql-engine` feature the engine is not compiled in; the
    /// command still dispatches and answers a precise failure (step 31.2).
    #[cfg(not(feature = "sql-engine"))]
    pub fn run_sql(&self, _sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        Err(
            "the SQL engine is not part of this build (enable the datacraft-core `sql-engine` feature)"
                .to_owned(),
        )
    }

    /// Result of the last successful `RunSql` (the UI's data source).
    pub fn sql_result(&self) -> Option<&crate::sql_types::SqlResult> {
        self.state.sql_result.as_ref()
    }

    fn run_sql_command(&mut self, sql: &str) -> Vec<Event> {
        match self.run_sql(sql) {
            Ok(result) => {
                let rows = result.rows.len();
                self.state.sql_result = Some(result);
                vec![Event::SqlRan { rows }]
            }
            Err(reason) => {
                self.state.sql_result = None;
                failed(reason)
            }
        }
    }

    /// Reads a window of rows through the port (the datasheet's data
    /// source; the port owns the backend mapping).
    pub fn read_rows(
        &self,
        table: &str,
        range: crate::port::RowRange,
    ) -> Result<Vec<Vec<crate::schema::FieldValue>>, PortError> {
        self.port.read_rows(table, range)
    }

    /// The open database's table definition (the datasheet's new-record
    /// row and the designers need domain kinds, not the projection).
    pub fn table_definition(&self, table: &str) -> Option<&crate::schema::TableDef> {
        self.state
            .database
            .as_ref()?
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
    }

    /// Applies one command; returns the observable outcomes in order.
    pub fn dispatch(&mut self, command: Command) -> Vec<Event> {
        let clears_history = clears_history(&command);
        let events = match command {
            Command::ToggleNavPane => {
                self.state.nav_pane_visible = !self.state.nav_pane_visible;
                vec![Event::NavPaneToggled {
                    visible: self.state.nav_pane_visible,
                }]
            }
            Command::CreateRelationship { relationship } => self.create_relationship(relationship),
            Command::DeleteRelationship { relationship } => self.delete_relationship(relationship),
            Command::ToggleRelationshipMapper => {
                self.state.relationship_mapper_open = !self.state.relationship_mapper_open;
                vec![Event::RelationshipMapperToggled {
                    open: self.state.relationship_mapper_open,
                }]
            }
            Command::ToggleSqlWindow => {
                self.state.sql_window_open = !self.state.sql_window_open;
                vec![Event::SqlWindowToggled {
                    open: self.state.sql_window_open,
                }]
            }
            Command::RunSql { sql } => self.run_sql_command(&sql),
            Command::RunExpression {
                context,
                expression,
            } => self.run_expression_command(context, &expression),
            Command::CreateQuery { name } => self.create_query(&name),
            Command::SetQbeCell {
                query,
                row,
                col,
                value,
            } => self.set_qbe_cell(&query, row, col, value),
            Command::ClearQbeCriteria { query } => self.clear_qbe_criteria(&query),
            Command::DeleteQuery { name } => self.delete_query(&name),
            Command::RunQuery { query } => self.run_query_command(&query),
            Command::CreateForm { name } => self.create_form(&name),
            Command::DeleteForm { name } => self.delete_form(&name),
            Command::AddFormControl { form, control } => self.add_control(&form, control),
            Command::SetFormControlRect {
                form,
                control,
                rect,
            } => self.set_control_rect(&form, &control, rect),
            Command::DeleteFormControl { form, control } => self.delete_control(&form, &control),
            Command::AddFormField { form, field } => self.add_form_field(&form, &field),
            Command::SetFormSource { form, table } => self.set_form_source(&form, &table),
            Command::CreateReport { name } => self.create_report(&name),
            Command::DeleteReport { name } => self.delete_report(&name),
            Command::AddReportControl { report, control } => {
                self.add_report_control(&report, control)
            }
            Command::SetReportControlRect {
                report,
                control,
                rect,
            } => self.set_report_control_rect(&report, &control, rect),
            Command::DeleteReportControl { report, control } => {
                self.delete_report_control(&report, &control)
            }
            Command::AddReportField { report, field } => self.add_report_field(&report, &field),
            Command::SetReportSource { report, table } => self.set_report_source(&report, &table),
            Command::SetReportGrouping { report, group_by } => {
                self.set_report_grouping(&report, group_by)
            }
            Command::LoadVbaModule { module } => self.load_vba_module(&module),
            Command::AnalyzeVba => self.analyze_vba_command(),
            Command::SaveDesignDocument { path } => self.save_design_document(&path),
            Command::LoadDesignDocument { path } => self.load_design_document(&path),
            Command::OpenDatabase { path, password } => self.open_database(path, password),
            Command::CloseDatabase => self.close_database(),
            Command::CreateDatabase { path, format } => self.create_database(path, format),
            Command::CreateDatabaseFromTemplate { path, template } => {
                self.create_database_from_template(path, template)
            }
            Command::SaveDatabaseAs { path, password } => self.save_database_as(path, password),
            Command::ChangePassword { new_password } => self.change_password(new_password),
            Command::CreateTable { definition } => self.create_table(definition),
            Command::DropTable { table } => self.drop_table(&table),
            Command::AddField { table, field } => {
                self.alter_table(&table, crate::port::SchemaChange::AddField { field })
            }
            Command::DropField { table, field } => {
                self.alter_table(&table, crate::port::SchemaChange::DropField { field })
            }
            Command::RenameField {
                table,
                field,
                new_name,
            } => self.alter_table(
                &table,
                crate::port::SchemaChange::RenameField { field, new_name },
            ),
            Command::SetFieldProperties {
                table,
                field,
                properties,
            } => self.alter_table(
                &table,
                crate::port::SchemaChange::SetFieldProperties { field, properties },
            ),
            Command::InsertRow { table, values } => self.insert_row(&table, &values),
            Command::UpdateRow { table, row, values } => self.update_row(&table, row, &values),
            Command::DeleteRow { table, row } => self.delete_row(&table, row),
            Command::PreviewWrite { table, ops } => self.preview_write(&table, ops),
            Command::ApplyWrite { token } => self.apply_write(token),
            Command::RollbackWrite { token } => self.rollback_write(token),
            Command::DiscardWrite { token } => self.discard_write(token),
            Command::OpenObject(object) => self.open_object(object, None),
            Command::OpenObjectInMode { object, mode } => self.open_object(object, Some(mode)),
            Command::CloseTab { tab_id } => self.close_tab(tab_id),
            Command::CloseOtherTabs { tab_id } => self.close_other_tabs(tab_id),
            Command::CloseAllTabs => self.close_all_tabs(),
            Command::SetActiveTab { tab_id } => self.set_active_tab(tab_id),
            Command::SetTabMode { tab_id, mode } => self.set_tab_mode(tab_id, mode),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
        };
        if clears_history {
            self.clear_history();
        }
        events
    }
}

/// Commands whose row effects are outside the row-image history: schema
/// changes, journal batches and file-level lifecycle (step 28.1, U14).
fn clears_history(command: &Command) -> bool {
    matches!(
        command,
        Command::OpenDatabase { .. }
            | Command::CloseDatabase
            | Command::CreateTable { .. }
            | Command::DropTable { .. }
            | Command::AddField { .. }
            | Command::DropField { .. }
            | Command::RenameField { .. }
            | Command::SetFieldProperties { .. }
            | Command::ApplyWrite { .. }
            | Command::RollbackWrite { .. }
    )
}

pub(crate) fn failed(reason: impl Into<String>) -> Vec<Event> {
    vec![Event::Failed {
        reason: reason.into(),
    }]
}
