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

mod export;
mod relationships;
mod rename;
mod sql;
mod table_view_state;
mod tabs;
mod views;

/// The headless application core: reducer + state + storage port.
pub struct Core {
    pub(crate) state: AppState,
    pub(crate) port: Box<dyn StoragePort>,
    /// Registered import-source readers (phase 96, M127): the composition
    /// roots add the Excel driver, so core never names a format crate.
    pub(crate) import_readers: Vec<Box<dyn crate::import::ImportReader>>,
}

impl Core {
    /// Creates a core over the given storage backend.
    pub fn new(port: Box<dyn StoragePort>) -> Self {
        let mut state = AppState::new();
        // The composition's driver inventory (M23) is fixed at
        // construction — the registry never mutates at runtime.
        state.drivers = port.driver_inventory();
        Self {
            state,
            port,
            import_readers: Vec::new(),
        }
    }

    /// Registers one out-of-tree import-source reader (phase 96, M127);
    /// the composition roots own the format crates.
    pub fn register_import_reader(&mut self, reader: Box<dyn crate::import::ImportReader>) {
        self.import_readers.push(reader);
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

    /// Rows the named table holds, without reading one (phase 101, step
    /// 101.5; item M132). A backend without a cheap count answers the
    /// port's precise refusal, and the client reads the whole table.
    pub fn row_count(&self, table: &str) -> Result<usize, String> {
        let Some(db) = self.state.database.as_ref() else {
            return Err("no database is open".to_owned());
        };
        let Some(definition) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
        else {
            return Err(format!("unknown table {table:?}"));
        };
        self.port
            .row_count(&definition.name)
            .map_err(|error| error.to_string())
    }

    /// One window of rows for the datasheet's bounded cache (phase 101,
    /// step 101.5; item M132).
    pub fn read_row_window(
        &self,
        table: &str,
        start: usize,
        len: usize,
    ) -> Result<Vec<Vec<crate::schema::FieldValue>>, String> {
        let Some(db) = self.state.database.as_ref() else {
            return Err("no database is open".to_owned());
        };
        let Some(definition) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
        else {
            return Err(format!("unknown table {table:?}"));
        };
        self.port
            .read_rows(&definition.name, crate::port::RowRange { start, len })
            .map_err(|error| error.to_string())
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

    /// The form's own filter criteria, if one is applied (phase 64, M71;
    /// ADR-8). `None` means every source row is visible.
    pub fn form_filter(&self, form: &str) -> Option<&str> {
        self.state
            .form_filters
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(form))
            .map(|(_, criteria)| criteria.as_str())
    }

    /// Row indices of a form's filtered record set, evaluated through the
    /// expression service (Access wildcards included). No filter means
    /// every row; an evaluation failure is precise and local.
    pub fn form_filter_rows(
        &self,
        form: &str,
        rows: &[Vec<crate::schema::FieldValue>],
    ) -> Result<Vec<usize>, String> {
        let Some(criteria) = self.form_filter(form) else {
            return Ok((0..rows.len()).collect());
        };
        let criteria = criteria.to_owned();
        let Some(db) = self.state.database.as_ref() else {
            return Err("no database is open".to_owned());
        };
        let Some(form_def) = db.forms.iter().find(|f| f.name.eq_ignore_ascii_case(form)) else {
            return Err(format!("form {form:?} not found"));
        };
        let Some(table) = db
            .tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(&form_def.source))
        else {
            return Err(format!("source table {:?} not found", form_def.source));
        };
        let expression = crate::expr::parse(&criteria)
            .map_err(|error| format!("filter {criteria:?}: {error}"))?;
        let mut indices = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let mut scope = crate::expr::Scope::new();
            for (field, value) in table.fields.iter().zip(row) {
                scope.set(field.name.clone(), value.clone());
            }
            match expression.evaluate(&scope) {
                Ok(crate::schema::FieldValue::Boolean(true)) => indices.push(index),
                Ok(crate::schema::FieldValue::Boolean(false) | crate::schema::FieldValue::Null) => {
                }
                Ok(other) => {
                    return Err(format!(
                        "filter {criteria:?} evaluated to {other:?}, not a logical value"
                    ));
                }
                Err(error) => return Err(format!("filter {criteria:?}: {error}")),
            }
        }
        Ok(indices)
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
            Command::ToggleImportWindow => {
                self.state.import_window_open = !self.state.import_window_open;
                Vec::new()
            }
            Command::PreviewImportFile { source } => self.preview_import_file(&source),
            Command::ToggleExpressionWindow => {
                self.state.expression_window_open = !self.state.expression_window_open;
                vec![Event::ExpressionWindowToggled {
                    open: self.state.expression_window_open,
                }]
            }
            Command::RunSql { sql } => self.run_sql_command(&sql),
            Command::RunExpression {
                context,
                expression,
            } => self.run_expression_command(context, &expression),
            Command::CreateQuery { name } => self.create_query(&name),
            Command::View(action) => self.view_command(action),
            Command::SetQbeCell {
                query,
                row,
                col,
                value,
            } => self.set_qbe_cell(&query, row, col, value),
            Command::ClearQbeCriteria { query } => self.clear_qbe_criteria(&query),
            Command::SetTableSort {
                table,
                field,
                order,
            } => self.set_table_sort(&table, &field, order),
            Command::SetTableFilter {
                table,
                field,
                criteria,
            } => self.set_table_filter(&table, &field, criteria.as_deref()),
            Command::ClearTableFilters { table } => self.clear_table_filters(&table),
            Command::ListColumnValues { table, field } => self.list_column_values(&table, &field),
            Command::FilterByValue {
                table,
                field,
                value,
            } => self.filter_by_value(&table, &field, value.as_deref()),
            Command::DeleteQuery { name } => self.delete_query(&name),
            Command::RunQuery { query } => self.run_query_command(&query),
            Command::CreateForm { name } => self.create_form(&name),
            Command::DeleteForm { name } => self.delete_form(&name),
            Command::AddFormControl { form, control } => self.add_control(&form, control),
            Command::SetFormControlContent {
                form,
                control,
                value,
            } => self.set_control_content(&form, &control, &value),
            Command::SetFormControlData(data) => self.set_control_data(&data),
            Command::SetFormFilter(filter) => self.set_form_filter(&filter),
            Command::Design(action) => self.design_command(action),
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
            Command::SetReportControlContent {
                report,
                control,
                value,
            } => self.set_report_control_content(&report, &control, &value),
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
            Command::AnalyzeApp => self.analyze_app_command(),
            Command::SaveDesignDocument { path } => self.save_design_document(&path),
            Command::LoadDesignDocument { path } => self.load_design_document(&path),
            Command::ExportReportPdf { report, path } => self.export_report_pdf(&report, &path),
            Command::ExportTable {
                table,
                format,
                path,
            } => self.export_table(&table, format, &path),
            Command::OpenDatabase {
                path,
                password,
                source,
            } => self.open_database(path, password, source),
            Command::CloseDatabase => self.close_database(),
            Command::CompactDatabase => self.compact_database(),
            Command::CreateDatabase { path, format } => self.create_database(path, format),
            Command::CreateDatabaseFromTemplate { path, template } => {
                self.create_database_from_template(path, template)
            }
            Command::SaveDatabaseAs { path, password } => self.save_database_as(path, password),
            Command::ChangePassword { new_password } => self.change_password(new_password),
            Command::CreateTable { definition } => self.create_table(definition),
            Command::ImportFile { source, table } => self.import_file(&source, &table),
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
            Command::UpdateCells { table, edits } => self.update_cells(&table, &edits),
            Command::DeleteRow { table, row } => self.delete_row(&table, row),
            Command::RenameObject { object, new_name } => self.rename_object(&object, &new_name),
            Command::PreviewWrite { table, ops } => self.preview_write(&table, ops),
            Command::ApplyWrite { token } => self.apply_write(token),
            Command::RollbackWrite { token } => self.rollback_write(token),
            Command::DiscardWrite { token } => self.discard_write(token),
            Command::OpenObject(object) => self.open_object(object, None),
            Command::OpenObjectInMode { object, mode } => self.open_object(object, Some(mode)),
            Command::CloseTab { tab_id } => self.close_tab(tab_id),
            Command::CloseOtherTabs { tab_id } => self.close_other_tabs(tab_id),
            Command::CloseTabsToLeft { tab_id } => {
                self.close_tabs_to_side(tab_id, tabs::Side::Left)
            }
            Command::CloseTabsToRight { tab_id } => {
                self.close_tabs_to_side(tab_id, tabs::Side::Right)
            }
            Command::CloseAllTabs => self.close_all_tabs(),
            Command::SetActiveTab { tab_id } => self.set_active_tab(tab_id),
            Command::SetTabMode { tab_id, mode } => self.set_tab_mode(tab_id, mode),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
        };
        if clears_history {
            // A failed barrier command changed nothing: keep the history it
            // would otherwise discard (L15).
            let failed = events.iter().any(|event| {
                matches!(
                    event,
                    Event::Failed { .. }
                        | Event::PasswordRequired { .. }
                        | Event::PasswordRejected { .. }
                )
            });
            if !failed {
                self.clear_history();
            }
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
            | Command::ImportFile { .. }
            | Command::DropTable { .. }
            | Command::AddField { .. }
            | Command::DropField { .. }
            | Command::RenameField { .. }
            | Command::SetFieldProperties { .. }
            | Command::ApplyWrite { .. }
            | Command::RollbackWrite { .. }
    ) || matches!(
        command,
        Command::RenameObject { object, .. } if object.kind == crate::ObjectKind::Table
    )
}

pub(crate) fn failed(code: crate::failure::FailureCode, reason: impl Into<String>) -> Vec<Event> {
    vec![Event::Failed {
        code,
        reason: reason.into(),
    }]
}
