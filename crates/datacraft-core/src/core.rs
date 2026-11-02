//! Command dispatch core (step 4.1, item H13; ARCHITECTURE §5).
//!
//! `Core::dispatch` is a total reducer: every command produces a
//! deterministic event list, failures become `Event::Failed` with precise
//! reasons. Storage I/O happens only behind the [`StoragePort`](crate::port)
//! — the reducer itself is pure over [`AppState`].

use crate::ObjectKind;
use crate::command::Command;
use crate::event::Event;
use crate::ids::TabId;
use crate::port::{PortError, StoragePort};
use crate::snapshot::UiStateSnapshot;
use crate::state::{AppState, TabMode, TabState};

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
            let Some(def) = native.query_def() else {
                return Err(format!(
                    "native query {name:?} carries constructs outside the decoded subset                      (criteria, joins, grouping or computed columns) — BACKLOG M3 rest"
                ));
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
            return crate::query_engine::run_query(&def, source, &rows).map_err(|e| e.to_string());
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
    pub fn run_sql(&self, sql: &str) -> Result<crate::sql_engine::SqlResult, String> {
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

    /// Result of the last successful `RunSql` (the UI's data source).
    pub fn sql_result(&self) -> Option<&crate::sql_engine::SqlResult> {
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
        match command {
            Command::ToggleNavPane => {
                self.state.nav_pane_visible = !self.state.nav_pane_visible;
                vec![Event::NavPaneToggled {
                    visible: self.state.nav_pane_visible,
                }]
            }
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
        }
    }

    /// Opens (or re-activates) an object tab. An explicit mode is honoured
    /// for kinds with a design view and applied to existing tabs as well
    /// (step 21.5, item U9).
    fn open_object(&mut self, object: crate::ObjectId, mode: Option<TabMode>) -> Vec<Event> {
        if mode.is_some() && !object.kind.has_design_view() {
            return failed(format!(
                "the {} object {:?} has no design view",
                object.kind.slug(),
                object.name
            ));
        }
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        let exists = match object.kind {
            ObjectKind::Table => db
                .tables
                .iter()
                .any(|t| t.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Query => {
                db.queries
                    .iter()
                    .any(|q| q.name.to_lowercase() == object.name.to_lowercase())
                    || db
                        .native_queries
                        .iter()
                        .any(|q| q.name.to_lowercase() == object.name.to_lowercase())
            }
            ObjectKind::Form => db
                .forms
                .iter()
                .any(|f| f.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Report => db
                .reports
                .iter()
                .any(|r| r.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Macro => db
                .macros
                .iter()
                .any(|m| m.name.eq_ignore_ascii_case(&object.name)),
            ObjectKind::Module => db
                .vba_project
                .as_ref()
                .is_some_and(|project| project.module(&object.name).is_some()),
        };
        if !exists {
            return failed(PortError::TableNotFound(object.name).to_string());
        }
        if let Some(existing) = self
            .state
            .tabs
            .iter()
            .find(|tab| tab.object == object)
            .cloned()
        {
            self.state.active_tab = Some(existing.id);
            let mut events = vec![Event::ActiveTabChanged {
                tab_id: Some(existing.id),
            }];
            if let Some(mode) = mode {
                if let Some(tab) = self.state.tabs.iter_mut().find(|tab| tab.id == existing.id) {
                    tab.mode = mode;
                }
                events.push(Event::TabModeSet {
                    tab_id: existing.id,
                    mode,
                });
            }
            return events;
        }
        let tab_id = TabId::new(self.state.next_tab_id);
        self.state.next_tab_id += 1;
        self.state.tabs.push(TabState {
            id: tab_id,
            object: object.clone(),
            mode: mode.unwrap_or(TabMode::Datasheet),
        });
        self.state.active_tab = Some(tab_id);
        vec![
            Event::TabOpened { tab_id, object },
            Event::ActiveTabChanged {
                tab_id: Some(tab_id),
            },
        ]
    }

    fn close_tab(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(format!("unknown tab {tab_id}"));
        }
        self.remove_tab(tab_id);
        if self.state.active_tab == Some(tab_id) {
            self.state.active_tab = self.neighbor_of(tab_id);
            let mut events = vec![Event::TabClosed { tab_id }];
            events.push(Event::ActiveTabChanged {
                tab_id: self.state.active_tab,
            });
            return events;
        }
        vec![Event::TabClosed { tab_id }]
    }

    /// Closes every tab but `tab_id` (step 21.6, item U10); the kept tab
    /// stays open and becomes active.
    fn close_other_tabs(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(format!("unknown tab {tab_id}"));
        }
        let closed: Vec<TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| tab.id != tab_id)
            .map(|tab| tab.id)
            .collect();
        self.state.tabs.retain(|tab| tab.id == tab_id);
        let mut events: Vec<Event> = closed
            .into_iter()
            .map(|tab_id| Event::TabClosed { tab_id })
            .collect();
        if self.state.active_tab != Some(tab_id) {
            self.state.active_tab = Some(tab_id);
            events.push(Event::ActiveTabChanged {
                tab_id: Some(tab_id),
            });
        }
        events
    }

    /// Closes every tab (step 21.6, item U10); the workspace ends empty.
    fn close_all_tabs(&mut self) -> Vec<Event> {
        if self.state.tabs.is_empty() {
            return failed("no tabs are open");
        }
        let closed: Vec<TabId> = self.state.tabs.iter().map(|tab| tab.id).collect();
        self.state.tabs.clear();
        self.state.active_tab = None;
        let mut events: Vec<Event> = closed
            .into_iter()
            .map(|tab_id| Event::TabClosed { tab_id })
            .collect();
        events.push(Event::ActiveTabChanged { tab_id: None });
        events
    }

    fn set_active_tab(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(format!("unknown tab {tab_id}"));
        }
        self.state.active_tab = Some(tab_id);
        vec![Event::ActiveTabChanged {
            tab_id: Some(tab_id),
        }]
    }

    /// Switches one tab's view (step 21.1, item U5). Only kinds with a
    /// design view accept the switch; the failure is precise.
    fn set_tab_mode(&mut self, tab_id: TabId, mode: TabMode) -> Vec<Event> {
        let Some(tab) = self.state.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return failed(format!("unknown tab {tab_id}"));
        };
        if !tab.object.kind.has_design_view() {
            return failed(format!(
                "the {} tab {:?} has no design view",
                tab.object.kind.slug(),
                tab.object.name
            ));
        }
        tab.mode = mode;
        vec![Event::TabModeSet { tab_id, mode }]
    }

    /// Activation rule: the highest remaining tab id below the closed one,
    /// else the lowest remaining id, else none.
    fn neighbor_of(&self, closed: TabId) -> Option<TabId> {
        let below = self
            .state
            .tabs
            .iter()
            .map(|tab| tab.id)
            .filter(|id| id.value() < closed.value())
            .max();
        below.or_else(|| self.state.tabs.iter().map(|tab| tab.id).min())
    }

    pub(crate) fn remove_tab(&mut self, tab_id: TabId) {
        self.state.tabs.retain(|tab| tab.id != tab_id);
    }
}

pub(crate) fn failed(reason: impl Into<String>) -> Vec<Event> {
    vec![Event::Failed {
        reason: reason.into(),
    }]
}
