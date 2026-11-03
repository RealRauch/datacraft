//! Database lifecycle dispatch (steps 4.1/6.x; split from `core.rs` to
//! keep every file under the size gate — same pattern as `forms.rs`):
//! open/close database and create/drop table, including dependent-tab
//! closing on drop.

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::ids::TabId;
use crate::port::{DriverCapabilities, PortError, SchemaChange};
use crate::schema::{FieldValue, TableDef};
use crate::state::DatabaseState;

mod alter;
mod password;
mod sidecar;

use sidecar::read_sidecar;

impl crate::core::Core {
    pub(crate) fn open_database(
        &mut self,
        path: std::path::PathBuf,
        password: Option<String>,
    ) -> Vec<Event> {
        if self.state.database.is_some() {
            return failed("a database is already open — close it first");
        }
        match self.port.open(&path, password.as_deref()) {
            // Dedicated events let every client react without parsing prose
            // (the UI prompt, the CLI exit code, the MCP agent).
            Err(PortError::PasswordRequired { .. }) => {
                vec![Event::PasswordRequired { path }]
            }
            Err(PortError::WrongPassword { .. }) => {
                vec![Event::PasswordRejected { path }]
            }
            Err(e) => failed(e.to_string()),
            Ok(opened) => {
                let name = opened.name.clone();
                let format = opened.format;
                // Driver identity and per-session capabilities (phase 30,
                // M17; ADR-7) ride the open into state and snapshot.
                let driver = self.port.driver_id();
                let capabilities = self.port.capabilities();
                let relationships = self.port.relationships().unwrap_or_default();
                let (macros, macros_error) = match self.port.macros() {
                    Ok(macros) => (macros, None),
                    // Observable degradation, never a silent drop: the
                    // snapshot carries the precise reason.
                    Err(error) => (Vec::new(), Some(error.to_string())),
                };
                let (vba_project, vba_error) = match self.port.vba_project() {
                    Ok(project) => (project, None),
                    Err(error) => (None, Some(error.to_string())),
                };
                let (native_queries, native_queries_error) = match self.port.native_queries() {
                    Ok(queries) => (queries, None),
                    // Observable degradation, never a silent empty list: the
                    // snapshot carries the precise reason (step 23.3, M10).
                    Err(error) => (Vec::new(), Some(error.to_string())),
                };
                // The sibling sidecar belongs to this database (ADR-1): it
                // auto-loads with the open. A missing sidecar is not
                // special; a broken or foreign one surfaces as
                // `sidecar_error` without blocking the open (phase 29,
                // step 29.4).
                let (sidecar, sidecar_error) = match read_sidecar(&path, &name) {
                    Ok(Some(document)) => (Some(document), None),
                    Ok(None) => (None, None),
                    Err(reason) => (None, Some(reason)),
                };
                // The sidecar is the property authority (step 32.1, item
                // H45): overlay its field properties onto the physical
                // catalog. A stale or invalid overlay surfaces as
                // `sidecar_error` without blocking the open or discarding
                // the design documents.
                let mut tables = self.port.tables();
                let mut sidecar_error = sidecar_error;
                if let Some(document) = &sidecar {
                    let mut merged = tables.clone();
                    match document.apply_table_properties(&mut merged) {
                        Ok(()) => tables = merged,
                        Err(error) => sidecar_error = Some(error.to_string()),
                    }
                }
                let sidecar_path = path.with_extension("datacraft");
                self.state.sql_result = None;
                self.state.expression_result = None;
                // A journal belongs to the database it was previewed on
                // (H48): nothing may outlive a lifecycle change.
                self.state.pending_write = None;
                self.state.last_applied = None;
                self.state.database = Some(DatabaseState {
                    path,
                    name: name.clone(),
                    format,
                    driver: driver.clone(),
                    capabilities,
                    tables,
                    relationships,
                    queries: sidecar
                        .as_ref()
                        .map(|document| document.queries.clone())
                        .unwrap_or_default(),
                    forms: sidecar
                        .as_ref()
                        .map(|document| document.forms.clone())
                        .unwrap_or_default(),
                    reports: sidecar
                        .as_ref()
                        .map(|document| document.reports.clone())
                        .unwrap_or_default(),
                    native_queries,
                    native_queries_error,
                    macros,
                    macros_error,
                    vba_project,
                    vba_error,
                    sidecar_error,
                    undo: Vec::new(),
                    redo: Vec::new(),
                });
                let mut events = vec![Event::DatabaseOpened { name, format }];
                if let Some(document) = sidecar {
                    events.push(Event::DesignDocumentLoaded {
                        path: sidecar_path,
                        forms: document.forms.len(),
                        queries: document.queries.len(),
                        reports: document.reports.len(),
                    });
                }
                events
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
        self.state.sql_window_open = false;
        self.state.sql_result = None;
        self.state.expression_result = None;
        // Journals never survive their database (H48).
        self.state.pending_write = None;
        self.state.last_applied = None;
        self.port.close();
        events.push(Event::DatabaseClosed);
        events
    }

    /// Creates a new database file (phase 29, step 29.1, item H44).
    /// Independent of the open database: creation touches neither the open
    /// state nor its tabs, and the new file stays closed until an
    /// `OpenDatabase` command.
    pub(crate) fn create_database(
        &mut self,
        path: std::path::PathBuf,
        format: crate::port::DatabaseFormat,
    ) -> Vec<Event> {
        // Capability gate (phase 30, M17): the driver declared creation
        // impossible — refuse before the port, same Failed shape.
        if !self.port.capabilities().create_database {
            return failed(DriverCapabilities::unsupported(
                &self.port.driver_id(),
                "create database files",
            ));
        }
        match self.port.create_database(&path, format) {
            Ok(()) => vec![Event::DatabaseCreated { path, format }],
            Err(e) => failed(e.to_string()),
        }
    }

    /// Materializes a shipped template into a new file (phase 29, step
    /// 29.2, item H44). Creation plus the template's tables run through the
    /// port; any failure removes the partial file and reports a precise
    /// `Failed`. The new database is not opened. The command requires no
    /// open database: materialization opens the new file through the live
    /// port, so an open session must be closed first (M27).
    pub(crate) fn create_database_from_template(
        &mut self,
        path: std::path::PathBuf,
        template: String,
    ) -> Vec<Event> {
        if self.state.database.is_some() {
            return failed("close the open database before creating a database from a template");
        }
        if !self.port.capabilities().create_database {
            return failed(DriverCapabilities::unsupported(
                &self.port.driver_id(),
                "create database files",
            ));
        }
        let Some(template) = crate::templates::find_template(&template) else {
            let available: Vec<&str> = crate::templates::templates()
                .iter()
                .map(|entry| entry.name)
                .collect();
            return failed(format!(
                "unknown template {template:?} — available: {}",
                available.join(", ")
            ));
        };
        let format = template.format;
        if let Err(e) = self.port.create_database(&path, format) {
            return failed(e.to_string());
        }
        if let Err(e) = self.materialize_template_tables(&path, &template.tables) {
            let _ = std::fs::remove_file(&path);
            return failed(e);
        }
        vec![Event::DatabaseCreated { path, format }]
    }

    /// Writes the template's tables through the port (open, create, insert,
    /// close). Returns the error text on the first failure.
    fn materialize_template_tables(
        &mut self,
        path: &std::path::Path,
        tables: &[crate::templates::TemplateTable],
    ) -> Result<(), String> {
        if tables.is_empty() {
            return Ok(());
        }
        if let Err(e) = self.port.open(path, None) {
            return Err(e.to_string());
        }
        for table in tables {
            if let Err(e) = self.port.create_table(&table.definition) {
                self.port.close();
                return Err(e.to_string());
            }
            for row in &table.rows {
                if let Err(e) = self.port.insert_row(&table.definition.name, row) {
                    self.port.close();
                    return Err(e.to_string());
                }
            }
        }
        self.port.close();
        Ok(())
    }

    pub(crate) fn create_table(&mut self, definition: TableDef) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.create_table {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "create tables for this source",
            ));
        }
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

    /// Appends one row (step 13.2, H36; step 15.2 wires defaults,
    /// calculated fields and rules): row preparation first, then the port
    /// write, then the event — a rejected row never reaches storage.
    pub(crate) fn insert_row(
        &mut self,
        table: &str,
        values: &[crate::schema::FieldValue],
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7): a source the driver
        // declared read-only never reaches the port.
        if !db.capabilities.write_rows {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "write rows for this source",
            ));
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(PortError::TableNotFound(table.to_owned()).to_string());
        };
        let canonical = def.name.clone();
        let prepared = match crate::field_expressions::prepare_insert(def, values) {
            Ok(prepared) => prepared,
            Err(reason) => return failed(reason),
        };
        match self.port.insert_row(&canonical, &prepared) {
            Err(e) => failed(e.to_string()),
            Ok(()) => {
                self.record_insert(&canonical);
                vec![Event::RowInserted { table: canonical }]
            }
        }
    }

    /// Replaces one row (step 11.10, H40): schema-validated like an insert,
    /// with AutoNumber values preserved (Null keeps the current value, any
    /// other value must match it).
    pub(crate) fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.write_rows {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "write rows for this source",
            ));
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(PortError::TableNotFound(table.to_owned()).to_string());
        };
        let canonical = def.name.clone();
        let fields = def.fields.clone();
        if values.len() != fields.len() {
            return failed(format!(
                "update {:?}: expected {} values, found {}",
                canonical,
                fields.len(),
                values.len()
            ));
        }
        let current = match self
            .port
            .read_rows(&canonical, crate::port::RowRange { start: row, len: 1 })
        {
            Ok(rows) if rows.len() == 1 => rows.into_iter().next().expect("length checked"),
            Ok(_) => return failed(format!("row {row} of {canonical:?} is out of range")),
            Err(e) => return failed(e.to_string()),
        };
        let prepared = match crate::field_expressions::prepare_update(def, &current, values) {
            Ok(prepared) => prepared,
            Err(reason) => return failed(reason),
        };
        match self.port.update_row(&canonical, row, &prepared) {
            Err(e) => failed(e.to_string()),
            Ok(()) => {
                self.record_update(&canonical, &current, &prepared);
                vec![Event::RowUpdated {
                    table: canonical,
                    row,
                }]
            }
        }
    }

    /// Deletes one row (step 11.10, H40).
    pub(crate) fn delete_row(&mut self, table: &str, row: usize) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.write_rows {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "write rows for this source",
            ));
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(PortError::TableNotFound(table.to_owned()).to_string());
        };
        let canonical = def.name.clone();
        let current = match self
            .port
            .read_rows(&canonical, crate::port::RowRange { start: row, len: 1 })
        {
            Ok(rows) if rows.len() == 1 => rows.into_iter().next().expect("length checked"),
            Ok(_) => return failed(format!("row {row} of {canonical:?} is out of range")),
            Err(e) => return failed(e.to_string()),
        };
        match self.port.delete_row(&canonical, row) {
            Err(e) => failed(e.to_string()),
            Ok(()) => {
                self.record_delete(&canonical, &current);
                vec![Event::RowDeleted {
                    table: canonical,
                    row,
                }]
            }
        }
    }

    pub(crate) fn drop_table(&mut self, table: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.drop_table {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "drop tables for this source",
            ));
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
            // Both sides: a relationship whose parent table is gone is just
            // as dangling as one whose child is (L16).
            db.relationships.retain(|rel| {
                !rel.table.eq_ignore_ascii_case(&canonical)
                    && !rel.related_table.eq_ignore_ascii_case(&canonical)
            });
        }
        events.push(Event::TableDropped { table: canonical });
        events
    }

    /// Applies one design-time schema change (step 24.3, item M9): validate
    /// against a cloned definition first, then let the port rewrite the
    /// catalog; the in-memory definition is refreshed from the backend.
    pub(crate) fn alter_table(&mut self, table: &str, change: SchemaChange) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.alter_table {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "alter the schema for this source",
            ));
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .cloned()
        else {
            return failed(PortError::TableNotFound(table.to_owned()).to_string());
        };
        let mut trial = def.clone();
        let field = match alter::apply_schema_change(&mut trial, &change) {
            Ok(field) => field,
            Err(reason) => return failed(reason),
        };
        if let Err(error) = trial.validate() {
            return failed(format!("invalid schema change: {error}"));
        }
        let canonical = def.name.clone();
        let old_tables = db.tables.clone();
        if let Err(error) = self.port.alter_table(&canonical, change) {
            return failed(error.to_string());
        }
        let mut tables = self.port.tables();
        // The physical catalog carries no property blocks (H45): keep the
        // sidecar overlay across the refresh, with the validated trial
        // definition winning for the altered table.
        crate::schema::SchemaDocument::merge_field_properties(&mut tables, &old_tables);
        crate::schema::SchemaDocument::merge_field_properties(&mut tables, &[trial.clone()]);
        if let Some(db) = self.state.database.as_mut() {
            db.tables = tables;
        }
        vec![Event::TableAltered {
            table: canonical,
            field,
        }]
    }
}
