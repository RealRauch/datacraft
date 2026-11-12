//! Database lifecycle dispatch (steps 4.1/6.x; split from `core.rs` to
//! keep every file under the size gate — same pattern as `forms.rs`):
//! open/close database and create/drop table, including dependent-tab
//! closing on drop.

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::ids::TabId;
use crate::port::{DriverCapabilities, PortError, SchemaChange};
use crate::schema::TableDef;
use crate::state::DatabaseState;

mod alter;
mod import;
mod password;
mod rows;
mod sidecar;
mod template;

use sidecar::read_sidecar;

impl crate::core::Core {
    pub(crate) fn open_database(
        &mut self,
        path: std::path::PathBuf,
        password: Option<String>,
        source: Option<crate::command::SourceSpec>,
    ) -> Vec<Event> {
        if self.state.database.is_some() {
            return failed(
                FailureCode::Lifecycle,
                "a database is already open — close it first",
            );
        }
        // Source resolution (step 65.1, item L8) precedes the port: a
        // `file://` URI opens exactly like its path equivalent; every
        // other scheme answers the precise no-server-driver boundary
        // before any storage is touched.
        let path = match crate::command::effective_path(&path, source.as_ref()) {
            Ok(path) => path,
            Err(error) => return failed(error.code(), error.to_string()),
        };
        match self.port.open(&path, password.as_deref()) {
            // Dedicated events let every client react without parsing prose
            // (the UI prompt, the CLI exit code, the MCP agent).
            Err(PortError::PasswordRequired { .. }) => {
                vec![Event::PasswordRequired { path }]
            }
            Err(PortError::WrongPassword { .. }) => {
                vec![Event::PasswordRejected { path }]
            }
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(opened) => {
                let name = opened.name.clone();
                let format = opened.format;
                // Access co-existence (step 53.1, item M59): the port
                // reports a detected sibling lock file as data — the
                // session opened read-only and every client localizes the
                // notice from this field (ADR-4).
                let lock_file = opened.lock_file.clone();
                // Driver identity and per-session capabilities (phase 30,
                // M17; ADR-7) ride the open into state and snapshot.
                let driver = self.port.driver_id();
                let capabilities = self.port.capabilities();
                // System relationships (MSys*) are Access's internal
                // navigation metadata, not user relationships: filtering
                // them here keeps every client honest — the mapper never
                // presents (or offers to delete) them (step 69.1, H57).
                let relationships: Vec<_> = self
                    .port
                    .relationships()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|rel| {
                        !is_system_table(&rel.table) && !is_system_table(&rel.related_table)
                    })
                    .collect();
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
                // auto-loads with the open. The sidecar is file-bound, so
                // it applies to filesystem-backed sessions only — which is
                // every session today: a `file://` URI resolves to the
                // same local file (65.1) and non-file schemes are refused
                // at the source boundary before this point. A missing
                // sidecar is not special; a broken or foreign one surfaces
                // as `sidecar_error` without blocking the open (phase 29,
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
                // File-owned views (phase 73, step 73.1; M90): the source
                // reports them; the sidecar stays the design authority, so a
                // design with the same name filters the foreign entry.
                let mut native_views = self.port.native_views().unwrap_or_default();
                if let Some(document) = &sidecar {
                    native_views.retain(|view| {
                        !document
                            .views
                            .iter()
                            .any(|design| design.name.eq_ignore_ascii_case(&view.name))
                    });
                }
                let sidecar_path = path.with_extension("datacraft");
                self.state.sql_result = None;
                self.state.expression_result = None;
                // A journal belongs to the database it was previewed on
                // (H48): nothing may outlive a lifecycle change.
                self.state.pending_write = None;
                self.state.last_applied = None;
                self.state.database = Some(DatabaseState {
                    table_views: std::collections::BTreeMap::new(),
                    path,
                    source,
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
                    views: sidecar
                        .as_ref()
                        .map(|document| document.views.clone())
                        .unwrap_or_default(),
                    native_views,
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
                    lock_file,
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
            return failed(FailureCode::NoDatabase, "no database is open");
        }
        // Form filters never survive their database (ADR-8).
        self.state.form_filters.clear();
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
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::NoRelationships),
                DriverCapabilities::unsupported(&self.port.driver_id(), "create database files"),
            );
        }
        match self.port.create_database(&path, format) {
            Ok(()) => vec![Event::DatabaseCreated { path, format }],
            Err(e) => failed(FailureCode::Port, e.to_string()),
        }
    }

    pub(crate) fn create_table(&mut self, definition: TableDef) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.create_table {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::CreateTables),
                DriverCapabilities::unsupported(&db.driver, "create tables for this source"),
            );
        }
        if let Err(e) = definition.validate() {
            return failed(FailureCode::Port, e.to_string());
        }
        if db
            .tables
            .iter()
            .any(|t| t.name.to_lowercase() == definition.name.to_lowercase())
            || db
                .native_views
                .iter()
                .any(|v| v.name.to_lowercase() == definition.name.to_lowercase())
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate table name {:?}", definition.name),
            );
        }
        match self.port.create_table(&definition) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                let name = definition.name.clone();
                db.tables.push(definition);
                vec![Event::TableCreated { table: name }]
            }
        }
    }
    pub(crate) fn drop_table(&mut self, table: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.drop_table {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::DropTables),
                DriverCapabilities::unsupported(&db.driver, "drop tables for this source"),
            );
        }
        let canonical = match self.port.drop_table(table) {
            Err(e) => return failed(FailureCode::Port, e.to_string()),
            Ok(canonical) => canonical,
        };
        // The dropped table's datasheet view state goes with it
        // (step 44.5, M54): no stale sort or filter can outlive it.
        if let Some(db) = self.state.database.as_mut() {
            db.table_views.remove(&canonical);
        }
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
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.alter_table {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::AlterTables),
                DriverCapabilities::unsupported(&db.driver, "alter the schema for this source"),
            );
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let mut trial = def.clone();
        let field = match alter::apply_schema_change(&mut trial, &change) {
            Ok(field) => field,
            Err(reason) => return failed(FailureCode::InvalidInput, reason),
        };
        if let Err(error) = trial.validate() {
            return failed(
                FailureCode::InvalidInput,
                format!("invalid schema change: {error}"),
            );
        }
        let canonical = def.name.clone();
        let old_tables = db.tables.clone();
        if let Err(error) = self.port.alter_table(&canonical, change) {
            return failed(FailureCode::Port, error.to_string());
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

/// Access system tables carry the `MSys` prefix (case-insensitive).
fn is_system_table(name: &str) -> bool {
    name.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("MSys"))
}
