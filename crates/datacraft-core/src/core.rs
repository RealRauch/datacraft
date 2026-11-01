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
use crate::schema::TableDef;
use crate::snapshot::UiStateSnapshot;
use crate::state::{AppState, DatabaseState, TabState};

/// The headless application core: reducer + state + storage port.
pub struct Core {
    state: AppState,
    port: Box<dyn StoragePort>,
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

    /// Reads a window of rows through the port (the datasheet's data
    /// source; the port owns the backend mapping).
    pub fn read_rows(
        &self,
        table: &str,
        range: crate::port::RowRange,
    ) -> Result<Vec<Vec<crate::schema::FieldValue>>, PortError> {
        self.port.read_rows(table, range)
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
            Command::CreateQuery { name } => self.create_query(&name),
            Command::SetQbeCell {
                query,
                row,
                col,
                value,
            } => self.set_qbe_cell(&query, row, col, value),
            Command::DeleteQuery { name } => self.delete_query(&name),
            Command::RunQuery { query } => self.run_query_command(&query),
            Command::OpenDatabase { path } => self.open_database(path),
            Command::CloseDatabase => self.close_database(),
            Command::CreateTable { definition } => self.create_table(definition),
            Command::DropTable { table } => self.drop_table(&table),
            Command::OpenObject(object) => self.open_object(object),
            Command::CloseTab { tab_id } => self.close_tab(tab_id),
            Command::SetActiveTab { tab_id } => self.set_active_tab(tab_id),
        }
    }

    fn open_database(&mut self, path: std::path::PathBuf) -> Vec<Event> {
        if self.state.database.is_some() {
            return failed("a database is already open — close it first");
        }
        match self.port.open(&path) {
            Err(e) => failed(e.to_string()),
            Ok(opened) => {
                let name = opened.name.clone();
                let format = opened.format;
                let relationships = self.port.relationships().unwrap_or_default();
                self.state.database = Some(DatabaseState {
                    path,
                    name: name.clone(),
                    format,
                    tables: self.port.tables(),
                    relationships,
                    queries: Vec::new(),
                });
                vec![Event::DatabaseOpened { name, format }]
            }
        }
    }

    fn close_database(&mut self) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed("no database is open");
        }
        let mut events: Vec<Event> = self
            .state
            .tabs
            .drain(..)
            .map(|tab| Event::TabClosed { tab_id: tab.id })
            .collect();
        self.state.active_tab = None;
        self.state.database = None;
        self.state.relationship_mapper_open = false;
        self.port.close();
        events.push(Event::DatabaseClosed);
        events
    }

    fn create_table(&mut self, definition: TableDef) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        if let Err(e) = definition.validate() {
            return failed(e.to_string());
        }
        if db
            .tables
            .iter()
            .any(|t| t.name.to_lowercase() == definition.name.to_lowercase())
        {
            return failed(format!("duplicate table name {:?}", definition.name));
        }
        match self.port.create_table(&definition) {
            Err(e) => failed(e.to_string()),
            Ok(()) => {
                let name = definition.name.clone();
                db.tables.push(definition);
                vec![Event::TableCreated { table: name }]
            }
        }
    }

    fn drop_table(&mut self, table: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed("no database is open — open a database first");
        }
        let canonical = match self.port.drop_table(table) {
            Err(e) => return failed(e.to_string()),
            Ok(canonical) => canonical,
        };
        let dependent: Vec<TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| {
                tab.object.kind == ObjectKind::Table
                    && tab.object.name.to_lowercase() == canonical.to_lowercase()
            })
            .map(|tab| tab.id)
            .collect();
        let mut events: Vec<Event> = Vec::with_capacity(dependent.len() + 1);
        for tab_id in dependent {
            self.remove_tab(tab_id);
            events.push(Event::TabClosed { tab_id });
        }
        if let Some(db) = self.state.database.as_mut() {
            db.tables.retain(|t| t.name != canonical);
            db.relationships
                .retain(|rel| !rel.table.eq_ignore_ascii_case(&canonical));
        }
        events.push(Event::TableDropped { table: canonical });
        events
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
            _ => false,
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

    fn create_query(&mut self, name: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed("no database is open — open a database first");
        }
        if name.trim().is_empty() {
            return failed(
                crate::schema::SchemaError::InvalidName {
                    detail: "names must not be empty or whitespace-only".to_owned(),
                }
                .to_string(),
            );
        }
        let db = self.state.database.as_ref().expect("checked above");
        if db
            .queries
            .iter()
            .any(|q| q.name.to_lowercase() == name.to_lowercase())
        {
            return failed(format!("duplicate query name {name:?}"));
        }
        let canonical = db
            .tables
            .iter()
            .find(|t| t.name.to_lowercase() == name.to_lowercase())
            .map(|t| t.name.clone())
            .unwrap_or_else(|| name.to_owned());
        self.state
            .database
            .as_mut()
            .expect("checked above")
            .queries
            .push(crate::qbe::QueryDef::new(&canonical, ""));
        vec![Event::QueryCreated { name: canonical }]
    }

    fn set_qbe_cell(
        &mut self,
        query: &str,
        row: crate::qbe::QbeRow,
        col: usize,
        value: crate::qbe::QbeValue,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .queries
            .iter_mut()
            .find(|q| q.name.to_lowercase() == query.to_lowercase())
        else {
            return failed(format!("query {query:?} not found"));
        };
        // Whitelists (LESSONS #5): the table against the database, fields
        // against their source table.
        let mut value = value;
        match row {
            crate::qbe::QbeRow::Table => {
                let crate::qbe::QbeValue::Text(table) = &value else {
                    return failed("the Table row takes a name");
                };
                if !db
                    .tables
                    .iter()
                    .any(|t| t.name.to_lowercase() == table.to_lowercase())
                {
                    return failed(PortError::TableNotFound(table.clone()).to_string());
                }
                if let crate::qbe::QbeValue::Text(chosen) = &mut value
                    && let Some(matched) = db
                        .tables
                        .iter()
                        .find(|tbl| tbl.name.to_lowercase() == chosen.to_lowercase())
                {
                    *chosen = matched.name.clone();
                }
            }
            crate::qbe::QbeRow::Field => {
                let crate::qbe::QbeValue::Text(field) = &value else {
                    return failed("the Field row takes a name");
                };
                let Some(source) = db
                    .tables
                    .iter()
                    .find(|t| t.name.to_lowercase() == def.table.to_lowercase())
                else {
                    return failed("set the source table first");
                };
                if !source
                    .fields
                    .iter()
                    .any(|f| f.name.to_lowercase() == field.to_lowercase())
                {
                    return failed(format!("field {field:?} not found in {:?}", source.name));
                }
                value = crate::qbe::QbeValue::Text(
                    source
                        .fields
                        .iter()
                        .find(|f| f.name.to_lowercase() == field.to_lowercase())
                        .expect("checked above")
                        .name
                        .clone(),
                );
            }
            _ => {}
        }
        match def.set_cell(row, col, value) {
            Err(e) => failed(e.to_string()),
            Ok(()) => vec![Event::QbeCellSet {
                query: def.name.clone(),
            }],
        }
    }

    fn delete_query(&mut self, name: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(pos) = db
            .queries
            .iter()
            .position(|q| q.name.to_lowercase() == name.to_lowercase())
        else {
            return failed(format!("query {name:?} not found"));
        };
        let canonical = db.queries.remove(pos).name;
        let dependent: Vec<TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| {
                tab.object.kind == ObjectKind::Query
                    && tab.object.name.to_lowercase() == canonical.to_lowercase()
            })
            .map(|tab| tab.id)
            .collect();
        let mut events: Vec<Event> = Vec::with_capacity(dependent.len() + 1);
        for tab_id in dependent {
            self.remove_tab(tab_id);
            events.push(Event::TabClosed { tab_id });
        }
        events.push(Event::QueryDropped { name: canonical });
        events
    }

    fn run_query_command(&mut self, query: &str) -> Vec<Event> {
        let canonical = self
            .state
            .database
            .as_ref()
            .and_then(|db| {
                db.queries
                    .iter()
                    .find(|q| q.name.eq_ignore_ascii_case(query))
            })
            .map(|q| q.name.clone());
        match self.run_query(query) {
            Ok(result) => vec![Event::QueryRan {
                query: canonical.unwrap_or_else(|| query.to_owned()),
                rows: result.rows.len(),
            }],
            Err(reason) => failed(reason),
        }
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

    fn remove_tab(&mut self, tab_id: TabId) {
        self.state.tabs.retain(|tab| tab.id != tab_id);
    }
}

fn failed(reason: impl Into<String>) -> Vec<Event> {
    vec![Event::Failed {
        reason: reason.into(),
    }]
}
