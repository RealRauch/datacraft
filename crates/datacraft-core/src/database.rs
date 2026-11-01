//! Database lifecycle dispatch (steps 4.1/6.x; split from `core.rs` to
//! keep every file under the size gate — same pattern as `forms.rs`):
//! open/close database and create/drop table, including dependent-tab
//! closing on drop.

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::ids::TabId;
use crate::port::PortError;
use crate::schema::{FieldKind, FieldValue, TableDef};
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

    /// Appends one row (step 13.2, H36): schema validation first, then the
    /// port write, then the event — a rejected row never reaches storage.
    pub(crate) fn insert_row(
        &mut self,
        table: &str,
        values: &[crate::schema::FieldValue],
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(PortError::TableNotFound(table.to_owned()).to_string());
        };
        if values.len() != def.fields.len() {
            return failed(format!(
                "insert into {:?}: expected {} values, found {}",
                def.name,
                def.fields.len(),
                values.len()
            ));
        }
        for (field, value) in def.fields.iter().zip(values) {
            // Null on an AutoNumber means "engine-assigned" (the Access
            // convention); any other value is rejected by the schema.
            if matches!(field.kind, FieldKind::AutoNumber(_)) && matches!(value, FieldValue::Null) {
                continue;
            }
            if let Err(e) = field.validate_value(value) {
                return failed(format!("field {:?}: {e}", field.name));
            }
        }
        let canonical = def.name.clone();
        match self.port.insert_row(&canonical, values) {
            Err(e) => failed(e.to_string()),
            Ok(()) => vec![Event::RowInserted { table: canonical }],
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
