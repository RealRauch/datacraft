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

    /// Applies one command; returns the observable outcomes in order.
    pub fn dispatch(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::ToggleNavPane => {
                self.state.nav_pane_visible = !self.state.nav_pane_visible;
                vec![Event::NavPaneToggled {
                    visible: self.state.nav_pane_visible,
                }]
            }
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
                self.state.database = Some(DatabaseState {
                    path,
                    name: name.clone(),
                    format,
                    tables: self.port.tables(),
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
        }
        events.push(Event::TableDropped { table: canonical });
        events
    }

    fn open_object(&mut self, object: crate::ObjectId) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        if object.kind == ObjectKind::Table {
            let exists = db
                .tables
                .iter()
                .any(|t| t.name.to_lowercase() == object.name.to_lowercase());
            if !exists {
                return failed(PortError::TableNotFound(object.name).to_string());
            }
        } else {
            return failed(format!(
                "object kind {:?} cannot be opened yet (tables only in this phase)",
                object.kind
            ));
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

    fn remove_tab(&mut self, tab_id: TabId) {
        self.state.tabs.retain(|tab| tab.id != tab_id);
    }
}

fn failed(reason: impl Into<String>) -> Vec<Event> {
    vec![Event::Failed {
        reason: reason.into(),
    }]
}
