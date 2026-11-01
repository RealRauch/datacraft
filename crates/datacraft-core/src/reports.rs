//! Report design dispatch (step 10.4, item H31).
//!
//! Plain impl-Core reducers over the report design documents, mirroring
//! `forms.rs` — the canvas math, palette and sidecar persistence are
//! shared and already proven; only the geometry semantics differ (page
//! bounds instead of a free canvas).

use crate::core::failed;
use crate::event::Event;

impl crate::core::Core {
    pub(crate) fn create_report(&mut self, name: &str) -> Vec<Event> {
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
            .reports
            .iter()
            .any(|r| r.name.to_lowercase() == name.to_lowercase())
        {
            return failed(format!("duplicate report name {name:?}"));
        }
        self.state
            .database
            .as_mut()
            .expect("checked above")
            .reports
            .push(crate::report::ReportDef::new(name));
        vec![Event::ReportCreated {
            name: name.to_owned(),
        }]
    }

    pub(crate) fn delete_report(&mut self, name: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(pos) = db
            .reports
            .iter()
            .position(|r| r.name.to_lowercase() == name.to_lowercase())
        else {
            return failed(format!("report {name:?} not found"));
        };
        let canonical = db.reports.remove(pos).name;
        vec![Event::ReportDropped { name: canonical }]
    }

    pub(crate) fn add_report_control(
        &mut self,
        report: &str,
        mut control: crate::form::ControlDef,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .reports
            .iter_mut()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
        else {
            return failed(format!("report {report:?} not found"));
        };
        if control.name.trim().is_empty() {
            return failed("control names must not be empty");
        }
        if def
            .controls
            .iter()
            .any(|c| c.name.to_lowercase() == control.name.to_lowercase())
        {
            return failed(format!("duplicate control name {:?}", control.name));
        }
        if let Some(bound) = bound_field(&control)
            && let Some(detail) = validate_binding(def, &db.tables, &bound)
        {
            return failed(detail);
        }
        control.rect = crate::canvas::snap_rect(control.rect, def.grid);
        if control.rect.x + control.rect.width > def.page.width
            || control.rect.y + control.rect.height > def.page.height
        {
            return failed(format!(
                "control {:?} exceeds the page bounds ({} x {})",
                control.name, def.page.width, def.page.height
            ));
        }
        let name = control.name.clone();
        def.controls.push(control);
        vec![Event::ReportControlAdded {
            report: def.name.clone(),
            control: name,
        }]
    }

    pub(crate) fn set_report_control_rect(
        &mut self,
        report: &str,
        control: &str,
        rect: crate::canvas::Rect,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .reports
            .iter_mut()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
        else {
            return failed(format!("report {report:?} not found"));
        };
        let Some(target) = def
            .controls
            .iter_mut()
            .find(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(format!("control {control:?} not found"));
        };
        let snapped = crate::canvas::snap_rect(rect, def.grid);
        if snapped.x + snapped.width > def.page.width
            || snapped.y + snapped.height > def.page.height
        {
            return failed(format!(
                "control {control:?} exceeds the page bounds ({} x {})",
                def.page.width, def.page.height
            ));
        }
        target.rect = snapped;
        vec![Event::ReportControlRectSet {
            report: def.name.clone(),
            control: target.name.clone(),
        }]
    }

    pub(crate) fn delete_report_control(&mut self, report: &str, control: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .reports
            .iter_mut()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
        else {
            return failed(format!("report {report:?} not found"));
        };
        let Some(pos) = def
            .controls
            .iter()
            .position(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(format!("control {control:?} not found"));
        };
        let canonical = def.controls.remove(pos).name;
        vec![Event::ReportControlDropped {
            report: def.name.clone(),
            control: canonical,
        }]
    }

    /// Adds a record-source field from the designer's field list as a
    /// bound control at the next free page slot (step 11.6).
    pub(crate) fn add_report_field(&mut self, report: &str, field: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .reports
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(report))
        else {
            return failed(format!("report {report:?} not found"));
        };
        let canonical = def.name.clone();
        let bounds = (def.page.width, def.page.height);
        let control = match crate::palette::place_field_control(
            &def.source,
            &db.tables,
            field,
            def.grid,
            bounds,
            &def.controls,
        ) {
            Ok(control) => control,
            Err(reason) => return failed(reason),
        };
        self.add_report_control(&canonical, control)
    }

    /// Assigns the source table (whitelist + re-validation of bindings).
    pub(crate) fn set_report_source(&mut self, report: &str, table: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(source) = db
            .tables
            .iter()
            .find(|t| t.name.to_lowercase() == table.to_lowercase())
        else {
            return failed(crate::port::PortError::TableNotFound(table.to_owned()).to_string());
        };
        let canonical = source.name.clone();
        let Some(def) = db
            .reports
            .iter_mut()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
        else {
            return failed(format!("report {report:?} not found"));
        };
        for control in &def.controls {
            if let Some(bound) = bound_field(control)
                && !bound.starts_with('=')
                && !source
                    .fields
                    .iter()
                    .any(|f| f.name.to_lowercase() == bound.to_lowercase())
            {
                return failed(format!(
                    "field {bound:?} not found in {:?} — rebind the control first",
                    canonical
                ));
            }
        }
        def.source = canonical.clone();
        vec![Event::ReportSourceSet {
            report: def.name.clone(),
            table: canonical,
        }]
    }

    /// Sets or clears the report's grouping field (step 15.2, H41); the
    /// field is whitelisted against the source table.
    pub(crate) fn set_report_grouping(
        &mut self,
        report: &str,
        group_by: Option<String>,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .reports
            .iter_mut()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
        else {
            return failed(format!("report {report:?} not found"));
        };
        if let Some(field) = &group_by {
            if def.source.is_empty() {
                return failed("set the report's source table before grouping");
            }
            let Some(source) = db
                .tables
                .iter()
                .find(|t| t.name.to_lowercase() == def.source.to_lowercase())
            else {
                return failed(format!("source table {:?} not found", def.source));
            };
            if !source
                .fields
                .iter()
                .any(|f| f.name.to_lowercase() == field.to_lowercase())
            {
                return failed(format!(
                    "group field {field:?} not found in {:?}",
                    source.name
                ));
            }
        }
        def.group_by = group_by;
        vec![Event::ReportGroupingSet {
            report: def.name.clone(),
            group_by: def.group_by.clone(),
        }]
    }
}

/// The bound field of a control, if the kind carries one (shared with the
/// form reducers in `forms.rs`).
fn bound_field(control: &crate::form::ControlDef) -> Option<String> {
    match &control.kind {
        crate::form::ControlKind::TextBox { bound_field }
        | crate::form::ControlKind::CheckBox { bound_field }
        | crate::form::ControlKind::ComboBox { bound_field } => bound_field.clone(),
        _ => None,
    }
}

/// Binding whitelist: a source table must be assigned and the field must
/// exist in it (LESSONS #5). A bound field starting with `=` is an
/// expression control (the Access `ControlSource` convention, step 15.2):
/// it must parse now so a broken band fails at design time, not in print.
fn validate_binding(
    def: &crate::report::ReportDef,
    tables: &[crate::schema::TableDef],
    bound: &str,
) -> Option<String> {
    if let Some(expression) = bound.strip_prefix('=') {
        return crate::expr::parse(expression)
            .err()
            .map(|e| format!("control expression {bound:?}: {e}"));
    }
    if def.source.is_empty() {
        return Some("set the report's source table before binding fields".to_owned());
    }
    let Some(source) = tables
        .iter()
        .find(|t| t.name.to_lowercase() == def.source.to_lowercase())
    else {
        return Some(format!("source table {:?} not found", def.source));
    };
    if !source
        .fields
        .iter()
        .any(|f| f.name.to_lowercase() == bound.to_lowercase())
    {
        return Some(format!("field {bound:?} not found in {:?}", source.name));
    }
    None
}
