//! Form and control dispatch (steps 8.1/8.2, items H28/H29).
//!
//! Split from core.rs to keep every file under the size gate; these
//! are plain impl-Core reducers over the form design documents.

use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;

impl crate::core::Core {
    pub(crate) fn create_form(&mut self, name: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        }
        if name.trim().is_empty() {
            return failed(
                FailureCode::InvalidInput,
                crate::schema::SchemaError::InvalidName {
                    detail: "names must not be empty or whitespace-only".to_owned(),
                }
                .to_string(),
            );
        }
        let db = self.state.database.as_ref().expect("checked above");
        if db
            .forms
            .iter()
            .any(|f| f.name.to_lowercase() == name.to_lowercase())
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate form name {name:?}"),
            );
        }
        self.state
            .database
            .as_mut()
            .expect("checked above")
            .forms
            .push(crate::form::FormDef::new(name));
        vec![Event::FormCreated {
            name: name.to_owned(),
        }]
    }

    pub(crate) fn delete_form(&mut self, name: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(pos) = db
            .forms
            .iter()
            .position(|f| f.name.to_lowercase() == name.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {name:?} not found"));
        };
        let canonical = db.forms.remove(pos).name;
        vec![Event::FormDropped { name: canonical }]
    }

    pub(crate) fn add_control(
        &mut self,
        form: &str,
        mut control: crate::form::ControlDef,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        if control.name.trim().is_empty() {
            return failed(FailureCode::InvalidInput, "control names must not be empty");
        }
        if def
            .controls
            .iter()
            .any(|c| c.name.to_lowercase() == control.name.to_lowercase())
        {
            return failed(
                FailureCode::Duplicate,
                format!("duplicate control name {:?}", control.name),
            );
        }
        if let Some(bound) = bound_field(&control) {
            let detail = validate_binding(def, &db.tables, &bound);
            if let Some(detail) = detail {
                return failed(FailureCode::InvalidInput, detail);
            }
        }
        control.rect = crate::canvas::snap_rect(control.rect, def.grid);
        if control.rect.x + control.rect.width > def.size.width
            || control.rect.y + control.rect.height > def.size.height
        {
            return failed(
                FailureCode::InvalidInput,
                format!(
                    "control {:?} exceeds the canvas bounds ({} x {})",
                    control.name, def.size.width, def.size.height
                ),
            );
        }
        let name = control.name.clone();
        def.controls.push(control);
        vec![Event::ControlAdded {
            form: def.name.clone(),
            control: name,
        }]
    }

    pub(crate) fn set_control_rect(
        &mut self,
        form: &str,
        control: &str,
        rect: crate::canvas::Rect,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        let Some(target) = def
            .controls
            .iter_mut()
            .find(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(
                FailureCode::NotFound,
                format!("control {control:?} not found"),
            );
        };
        let snapped = crate::canvas::snap_rect(rect, def.grid);
        if snapped.x + snapped.width > def.size.width
            || snapped.y + snapped.height > def.size.height
        {
            return failed(
                FailureCode::InvalidInput,
                format!(
                    "control {control:?} exceeds the canvas bounds ({} x {})",
                    def.size.width, def.size.height
                ),
            );
        }
        target.rect = snapped;
        vec![Event::ControlRectSet {
            form: def.name.clone(),
            control: target.name.clone(),
        }]
    }

    pub(crate) fn delete_control(&mut self, form: &str, control: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        let Some(pos) = def
            .controls
            .iter()
            .position(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(
                FailureCode::NotFound,
                format!("control {control:?} not found"),
            );
        };
        let canonical = def.controls.remove(pos).name;
        vec![Event::ControlDropped {
            form: def.name.clone(),
            control: canonical,
        }]
    }
    /// Adds a record-source field from the designer's field list as a
    /// bound control at the next free slot (step 11.6).
    pub(crate) fn add_form_field(&mut self, form: &str, field: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(def) = db.forms.iter().find(|f| f.name.eq_ignore_ascii_case(form)) else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        let canonical = def.name.clone();
        let bounds = (def.size.width, def.size.height);
        let control = match crate::palette::place_field_control(
            &def.source,
            &db.tables,
            field,
            def.grid,
            bounds,
            &def.controls,
        ) {
            Ok(control) => control,
            Err(reason) => return failed(FailureCode::InvalidInput, reason),
        };
        self.add_control(&canonical, control)
    }

    /// Assigns the source table (whitelist + re-validation of bindings).
    pub(crate) fn set_form_source(&mut self, form: &str, table: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(source) = db
            .tables
            .iter()
            .find(|t| t.name.to_lowercase() == table.to_lowercase())
        else {
            return failed(
                FailureCode::NotFound,
                crate::port::PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let canonical = source.name.clone();
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        for control in &def.controls {
            if let Some(bound) = bound_field(control)
                && !source
                    .fields
                    .iter()
                    .any(|f| f.name.to_lowercase() == bound.to_lowercase())
            {
                return failed(
                    FailureCode::NotFound,
                    format!(
                        "field {bound:?} not found in {:?} — rebind the control first",
                        canonical
                    ),
                );
            }
        }
        def.source = canonical.clone();
        vec![Event::FormSourceSet {
            form: def.name.clone(),
            table: canonical,
        }]
    }

    /// Writes forms + queries + reports to a sidecar file (deterministic
    /// bytes).
    pub(crate) fn save_design_document(&mut self, path: &std::path::Path) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let document = crate::schema::SchemaDocument {
            schema_version: crate::schema::SCHEMA_VERSION,
            database_name: db.name.clone(),
            tables: db.tables.clone(),
            queries: db.queries.clone(),
            forms: db.forms.clone(),
            reports: db.reports.clone(),
        };
        match crate::atomic_write::write_replacing(path, document.to_json().as_bytes()) {
            Ok(()) => vec![Event::DesignDocumentSaved {
                path: path.to_path_buf(),
            }],
            Err(e) => failed(
                FailureCode::DesignDocument,
                format!("sidecar write failed: {e}"),
            ),
        }
    }

    /// Loads forms + queries from a sidecar file (version gate and
    /// database-name check; the table list comes from the database file).
    pub(crate) fn load_design_document(&mut self, path: &std::path::Path) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let expected = db.name.clone();
        let json = match std::fs::read_to_string(path) {
            Ok(json) => json,
            Err(e) => {
                return failed(
                    FailureCode::DesignDocument,
                    format!("sidecar read failed: {e}"),
                );
            }
        };
        let document = match crate::schema::SchemaDocument::from_json(&json) {
            Ok(document) => document,
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        if let Err(e) = document.validate() {
            return failed(
                FailureCode::DesignDocument,
                format!("sidecar validation failed: {e}"),
            );
        }
        if document.database_name != expected {
            return failed(
                FailureCode::DesignDocument,
                format!(
                    "the sidecar belongs to {:?}, not to {:?}",
                    document.database_name, expected
                ),
            );
        }
        let forms = document.forms.len();
        let queries = document.queries.len();
        let reports = document.reports.len();
        let mut tables = db.tables.clone();
        if let Err(e) = document.apply_table_properties(&mut tables) {
            return failed(
                FailureCode::DesignDocument,
                format!("sidecar property overlay failed: {e}"),
            );
        }
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        db.tables = tables;
        db.forms = document.forms;
        db.queries = document.queries;
        db.reports = document.reports;
        vec![Event::DesignDocumentLoaded {
            path: path.to_path_buf(),
            forms,
            queries,
            reports,
        }]
    }
}

/// The bound field of a control, if the kind carries one.
fn bound_field(control: &crate::form::ControlDef) -> Option<String> {
    match &control.kind {
        crate::form::ControlKind::TextBox { bound_field }
        | crate::form::ControlKind::CheckBox { bound_field }
        | crate::form::ControlKind::ComboBox { bound_field } => bound_field.clone(),
        _ => None,
    }
}

/// Binding whitelist: a source table must be assigned and the field must
/// exist in it (LESSONS #5).
fn validate_binding(
    def: &crate::form::FormDef,
    tables: &[crate::schema::TableDef],
    bound: &str,
) -> Option<String> {
    if def.source.is_empty() {
        return Some("set the form's source table before binding fields".to_owned());
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
