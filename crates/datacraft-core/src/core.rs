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
use crate::state::{AppState, TabState};

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
        else {
            return Err(format!("query {name:?} not found"));
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
        crate::query_engine::run_query(def, source, &rows).map_err(|e| e.to_string())
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
            Command::SaveDesignDocument { path } => self.save_design_document(&path),
            Command::LoadDesignDocument { path } => self.load_design_document(&path),
            Command::OpenDatabase { path } => self.open_database(path),
            Command::CloseDatabase => self.close_database(),
            Command::CreateTable { definition } => self.create_table(definition),
            Command::DropTable { table } => self.drop_table(&table),
            Command::InsertRow { table, values } => self.insert_row(&table, &values),
            Command::UpdateRow { table, row, values } => self.update_row(&table, row, &values),
            Command::DeleteRow { table, row } => self.delete_row(&table, row),
            Command::PreviewWrite { table, ops } => self.preview_write(&table, ops),
            Command::ApplyWrite { token } => self.apply_write(token),
            Command::RollbackWrite { token } => self.rollback_write(token),
            Command::DiscardWrite { token } => self.discard_write(token),
            Command::OpenObject(object) => self.open_object(object),
            Command::CloseTab { tab_id } => self.close_tab(tab_id),
            Command::SetActiveTab { tab_id } => self.set_active_tab(tab_id),
        }
    }

    fn open_object(&mut self, object: crate::ObjectId) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        let exists = match object.kind {
            ObjectKind::Table => db
                .tables
                .iter()
                .any(|t| t.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Query => db
                .queries
                .iter()
                .any(|q| q.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Form => db
                .forms
                .iter()
                .any(|f| f.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Report => db
                .reports
                .iter()
                .any(|r| r.name.to_lowercase() == object.name.to_lowercase()),
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
            return vec![Event::ActiveTabChanged {
                tab_id: Some(existing.id),
            }];
        }
        let tab_id = TabId::new(self.state.next_tab_id);
        self.state.next_tab_id += 1;
        self.state.tabs.push(TabState {
            id: tab_id,
            object: object.clone(),
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

    fn set_active_tab(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(format!("unknown tab {tab_id}"));
        }
        self.state.active_tab = Some(tab_id);
        vec![Event::ActiveTabChanged {
            tab_id: Some(tab_id),
        }]
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
