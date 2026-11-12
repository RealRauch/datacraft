//! View design dispatch (step 71.1/71.3, item M93).
//!
//! Sidecar view designs: create/update/delete with name and shape
//! validation; SQLite applies them natively (the port answers
//! `supports_native_views`), while Jet/ACE/CSV execute the stored SELECT
//! through the shadow engine. A native failure rolls the sidecar change
//! back — the two authorities never diverge silently.

use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::view::ViewDef;

impl crate::core::Core {
    /// Routes one sidecar view command (step 71.1, item M93).
    pub(crate) fn view_command(&mut self, action: crate::view::ViewCommand) -> Vec<Event> {
        match action {
            crate::view::ViewCommand::Create { name, sql } => self.create_view(&name, &sql),
            crate::view::ViewCommand::SetSql { name, sql } => self.set_view_sql(&name, &sql),
            crate::view::ViewCommand::Delete { name } => self.delete_view(&name),
        }
    }

    /// Creates a named SELECT view; the name is unique against tables and
    /// other views (case-insensitive). On SQLite the view is applied
    /// natively; a native failure rolls the sidecar push back (71.3).
    pub(crate) fn create_view(&mut self, name: &str, sql: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        }
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return failed(
                FailureCode::InvalidInput,
                crate::schema::SchemaError::InvalidName {
                    detail: "names must not be empty or whitespace-only".to_owned(),
                }
                .to_string(),
            );
        }
        let view = ViewDef::new(trimmed, sql.trim());
        if !view.is_select() {
            return failed(
                FailureCode::InvalidInput,
                format!("view {trimmed:?} must be a SELECT statement"),
            );
        }
        let db = self.state.database.as_ref().expect("checked above");
        if db
            .tables
            .iter()
            .any(|table| table.name.eq_ignore_ascii_case(trimmed))
            || db
                .views
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(trimmed))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate view name {trimmed:?}"),
            );
        }
        let canonical = trimmed.to_owned();
        self.state
            .database
            .as_mut()
            .expect("checked above")
            .views
            .push(ViewDef::new(canonical.clone(), sql.trim()));
        if self.port.supports_native_views()
            && let Err(error) = self.port.create_native_view(&canonical, sql.trim())
        {
            self.state
                .database
                .as_mut()
                .expect("checked above")
                .views
                .pop();
            return failed(FailureCode::Port, error.to_string());
        }
        vec![Event::ViewCreated { name: canonical }]
    }

    /// Replaces a view's SELECT statement; the native view is re-created
    /// on SQLite, and a failure restores the sidecar and the native view.
    pub(crate) fn set_view_sql(&mut self, name: &str, sql: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let updated = ViewDef::new(name, sql.trim());
        if !updated.is_select() {
            return failed(
                FailureCode::InvalidInput,
                format!("view {name:?} must be a SELECT statement"),
            );
        }
        let Some(view) = db
            .views
            .iter_mut()
            .find(|view| view.name.eq_ignore_ascii_case(name))
        else {
            return failed(FailureCode::NotFound, format!("view {name:?} not found"));
        };
        let old_sql = view.sql.clone();
        view.sql = sql.trim().to_owned();
        let canonical = view.name.clone();
        if self.port.supports_native_views() {
            let applied = self
                .port
                .drop_native_view(&canonical)
                .and_then(|()| self.port.create_native_view(&canonical, sql.trim()));
            if let Err(error) = applied {
                if let Some(view) = self
                    .state
                    .database
                    .as_mut()
                    .expect("checked above")
                    .views
                    .iter_mut()
                    .find(|view| view.name == canonical)
                {
                    view.sql = old_sql.clone();
                }
                // Best effort: put the previous native definition back.
                let _ = self.port.create_native_view(&canonical, &old_sql);
                return failed(FailureCode::Port, error.to_string());
            }
        }
        vec![Event::ViewChanged { name: canonical }]
    }

    /// Deletes a view; on SQLite the native view drops first, so a failing
    /// native drop keeps the sidecar unchanged.
    pub(crate) fn delete_view(&mut self, name: &str) -> Vec<Event> {
        let canonical = {
            let Some(db) = self.state.database.as_ref() else {
                return failed(
                    FailureCode::NoDatabase,
                    "no database is open — open a database first",
                );
            };
            let Some(view) = db
                .views
                .iter()
                .find(|view| view.name.eq_ignore_ascii_case(name))
            else {
                return failed(FailureCode::NotFound, format!("view {name:?} not found"));
            };
            view.name.clone()
        };
        if self.port.supports_native_views()
            && let Err(error) = self.port.drop_native_view(&canonical)
        {
            return failed(FailureCode::Port, error.to_string());
        }
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(position) = db.views.iter().position(|view| view.name == canonical) else {
            return failed(FailureCode::NotFound, format!("view {name:?} not found"));
        };
        db.views.remove(position);
        let dependent: Vec<crate::ids::TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| {
                tab.object.kind == crate::ObjectKind::View
                    && tab.object.name.eq_ignore_ascii_case(&canonical)
            })
            .map(|tab| tab.id)
            .collect();
        let mut events: Vec<Event> = Vec::with_capacity(dependent.len() + 1);
        for tab_id in dependent {
            self.remove_tab(tab_id);
            events.push(Event::TabClosed { tab_id });
        }
        events.push(Event::ViewDeleted { name: canonical });
        events
    }
}
