//! Object renames (phase 72, step 72.1, item M81).
//!
//! One kind-tagged command. Tables rename through the port (the physical
//! catalog/DDL owns the name); queries, forms and reports are sidecar
//! designs and rename in place. Collisions are checked per namespace
//! before anything changes, so a refusal leaves the state untouched.
//! Every structural reference follows: open tabs, datasheet view state,
//! form filters, query/form/report sources, combo row sources, subforms
//! and button actions. Stored view SQL is free text and stays verbatim.

use crate::core::Core;
use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::form::{ButtonAction, ControlDef, ControlKind};
use crate::port::DriverCapabilities;
use crate::{ObjectId, ObjectKind};

impl Core {
    /// Renames one object; the kind rides [`ObjectId`].
    pub(crate) fn rename_object(&mut self, object: &ObjectId, new_name: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        }
        if let Err(error) = crate::schema::validate_object_name(new_name) {
            return failed(FailureCode::InvalidInput, error.to_string());
        }
        match object.kind {
            ObjectKind::Table => self.rename_table(object, new_name),
            ObjectKind::Query => self.rename_query(object, new_name),
            ObjectKind::Form => self.rename_form(object, new_name),
            ObjectKind::Report => self.rename_report(object, new_name),
            other => failed(
                FailureCode::InvalidInput,
                format!("{} objects cannot be renamed", other.slug()),
            ),
        }
    }

    fn rename_table(&mut self, object: &ObjectId, new_name: &str) -> Vec<Event> {
        let db = self
            .state
            .database
            .as_ref()
            .expect("checked by rename_object");
        if !db.capabilities.rename_table {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::AlterTables),
                DriverCapabilities::unsupported(&db.driver, "rename tables for this source"),
            );
        }
        let Some(position) = db
            .tables
            .iter()
            .position(|table| table.name.eq_ignore_ascii_case(&object.name))
        else {
            return failed(
                FailureCode::NotFound,
                format!("table {:?} not found", object.name),
            );
        };
        let canonical_old = db.tables[position].name.clone();
        if db
            .tables
            .iter()
            .enumerate()
            .any(|(index, table)| index != position && table.name.eq_ignore_ascii_case(new_name))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate table name {new_name:?}"),
            );
        }
        // Tables and views share one object namespace (M93).
        if db
            .views
            .iter()
            .any(|view| view.name.eq_ignore_ascii_case(new_name))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate object name {new_name:?} — a view already uses it"),
            );
        }
        let canonical_new = match self.port.rename_table(&canonical_old, new_name) {
            Ok(name) => name,
            Err(error) => return failed(FailureCode::Port, error.to_string()),
        };
        {
            let db = self
                .state
                .database
                .as_mut()
                .expect("checked by rename_object");
            db.tables[position].name = canonical_new.clone();
            if let Some(view) = db.table_views.remove(&canonical_old) {
                db.table_views.insert(canonical_new.clone(), view);
            }
            for relationship in &mut db.relationships {
                if relationship.table.eq_ignore_ascii_case(&canonical_old) {
                    relationship.table = canonical_new.clone();
                }
                if relationship
                    .related_table
                    .eq_ignore_ascii_case(&canonical_old)
                {
                    relationship.related_table = canonical_new.clone();
                }
            }
            for query in &mut db.queries {
                if query.table.eq_ignore_ascii_case(&canonical_old) {
                    query.table = canonical_new.clone();
                }
            }
            for form in &mut db.forms {
                if form.source.eq_ignore_ascii_case(&canonical_old) {
                    form.source = canonical_new.clone();
                }
                rename_combo_sources(&mut form.controls, &canonical_old, &canonical_new);
            }
            for report in &mut db.reports {
                if report.source.eq_ignore_ascii_case(&canonical_old) {
                    report.source = canonical_new.clone();
                }
            }
        }
        rename_tabs(self, ObjectKind::Table, &canonical_old, &canonical_new);
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Table,
            old_name: canonical_old,
            new_name: canonical_new,
        }]
    }

    fn rename_query(&mut self, object: &ObjectId, new_name: &str) -> Vec<Event> {
        let db = self
            .state
            .database
            .as_ref()
            .expect("checked by rename_object");
        let Some(position) = db
            .queries
            .iter()
            .position(|query| query.name.eq_ignore_ascii_case(&object.name))
        else {
            return failed(
                FailureCode::NotFound,
                format!("query {:?} not found", object.name),
            );
        };
        let canonical_old = db.queries[position].name.clone();
        if db
            .queries
            .iter()
            .enumerate()
            .any(|(index, query)| index != position && query.name.eq_ignore_ascii_case(new_name))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate query name {new_name:?}"),
            );
        }
        {
            let db = self
                .state
                .database
                .as_mut()
                .expect("checked by rename_object");
            db.queries[position].name = new_name.to_owned();
            for form in &mut db.forms {
                rename_combo_sources(&mut form.controls, &canonical_old, new_name);
                rename_button_targets(
                    &mut form.controls,
                    ObjectKind::Query,
                    &canonical_old,
                    new_name,
                );
            }
        }
        rename_tabs(self, ObjectKind::Query, &canonical_old, new_name);
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Query,
            old_name: canonical_old,
            new_name: new_name.to_owned(),
        }]
    }

    fn rename_form(&mut self, object: &ObjectId, new_name: &str) -> Vec<Event> {
        let db = self
            .state
            .database
            .as_ref()
            .expect("checked by rename_object");
        let Some(position) = db
            .forms
            .iter()
            .position(|form| form.name.eq_ignore_ascii_case(&object.name))
        else {
            return failed(
                FailureCode::NotFound,
                format!("form {:?} not found", object.name),
            );
        };
        let canonical_old = db.forms[position].name.clone();
        if db
            .forms
            .iter()
            .enumerate()
            .any(|(index, form)| index != position && form.name.eq_ignore_ascii_case(new_name))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate form name {new_name:?}"),
            );
        }
        {
            let db = self
                .state
                .database
                .as_mut()
                .expect("checked by rename_object");
            db.forms[position].name = new_name.to_owned();
            for form in &mut db.forms {
                rename_subforms(&mut form.controls, &canonical_old, new_name);
                rename_button_targets(
                    &mut form.controls,
                    ObjectKind::Form,
                    &canonical_old,
                    new_name,
                );
            }
        }
        if let Some(criteria) = self.state.form_filters.remove(&canonical_old) {
            self.state
                .form_filters
                .insert(new_name.to_owned(), criteria);
        }
        rename_tabs(self, ObjectKind::Form, &canonical_old, new_name);
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Form,
            old_name: canonical_old,
            new_name: new_name.to_owned(),
        }]
    }

    fn rename_report(&mut self, object: &ObjectId, new_name: &str) -> Vec<Event> {
        let db = self
            .state
            .database
            .as_ref()
            .expect("checked by rename_object");
        let Some(position) = db
            .reports
            .iter()
            .position(|report| report.name.eq_ignore_ascii_case(&object.name))
        else {
            return failed(
                FailureCode::NotFound,
                format!("report {:?} not found", object.name),
            );
        };
        let canonical_old = db.reports[position].name.clone();
        if db
            .reports
            .iter()
            .enumerate()
            .any(|(index, report)| index != position && report.name.eq_ignore_ascii_case(new_name))
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate report name {new_name:?}"),
            );
        }
        {
            let db = self
                .state
                .database
                .as_mut()
                .expect("checked by rename_object");
            db.reports[position].name = new_name.to_owned();
            for form in &mut db.forms {
                rename_button_targets(
                    &mut form.controls,
                    ObjectKind::Report,
                    &canonical_old,
                    new_name,
                );
            }
        }
        rename_tabs(self, ObjectKind::Report, &canonical_old, new_name);
        vec![Event::ObjectRenamed {
            kind: ObjectKind::Report,
            old_name: canonical_old,
            new_name: new_name.to_owned(),
        }]
    }
}

/// Rewrites a combo box's row source when it names the renamed object.
fn rename_combo_sources(controls: &mut [ControlDef], old: &str, new: &str) {
    for control in controls {
        if let ControlKind::ComboBox { row_source, .. } = &mut control.kind
            && row_source.eq_ignore_ascii_case(old)
        {
            *row_source = new.to_owned();
        }
    }
}

/// Rewrites an embedded subform's reference when it names the renamed form.
fn rename_subforms(controls: &mut [ControlDef], old: &str, new: &str) {
    for control in controls {
        if let ControlKind::SubForm { form } = &mut control.kind
            && form.eq_ignore_ascii_case(old)
        {
            *form = new.to_owned();
        }
    }
}

/// Rewrites button actions that address the renamed kind (query, form or
/// report).
fn rename_button_targets(controls: &mut [ControlDef], kind: ObjectKind, old: &str, new: &str) {
    for control in controls {
        let ControlKind::Button {
            action: Some(action),
            ..
        } = &mut control.kind
        else {
            continue;
        };
        match (kind, action) {
            (ObjectKind::Query, ButtonAction::RunQuery { query })
                if query.eq_ignore_ascii_case(old) =>
            {
                *query = new.to_owned();
            }
            (ObjectKind::Form, ButtonAction::OpenForm { form })
                if form.eq_ignore_ascii_case(old) =>
            {
                *form = new.to_owned();
            }
            (ObjectKind::Report, ButtonAction::OpenReport { report })
                if report.eq_ignore_ascii_case(old) =>
            {
                *report = new.to_owned();
            }
            _ => {}
        }
    }
}

/// Renames the open tabs of one object kind (the tab id stays stable).
fn rename_tabs(core: &mut Core, kind: ObjectKind, old: &str, new: &str) {
    for tab in &mut core.state.tabs {
        if tab.object.kind == kind && tab.object.name.eq_ignore_ascii_case(old) {
            tab.object.name = new.to_owned();
        }
    }
}
