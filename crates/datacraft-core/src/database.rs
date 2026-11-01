//! Database lifecycle dispatch (steps 4.1/6.x; split from `core.rs` to
//! keep every file under the size gate — same pattern as `forms.rs`):
//! open/close database and create/drop table, including dependent-tab
//! closing on drop.

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::ids::TabId;
use crate::schema::TableDef;
use crate::state::DatabaseState;

impl crate::core::Core {
    pub(crate) fn open_database(&mut self, path: std::path::PathBuf) -> Vec<Event> {
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
                    forms: Vec::new(),
                    reports: Vec::new(),
                });
                vec![Event::DatabaseOpened { name, format }]
            }
        }
    }

    pub(crate) fn close_database(&mut self) -> Vec<Event> {
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

    pub(crate) fn create_table(&mut self, definition: TableDef) -> Vec<Event> {
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

    pub(crate) fn drop_table(&mut self, table: &str) -> Vec<Event> {
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
}
