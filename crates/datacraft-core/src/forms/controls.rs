//! The placed-control handlers (split from `forms.rs` for the size gate,
//! item H56): add, move, set the content and delete one control of a
//! form — validation first, then the mutation, then the event.

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;

use super::{binding_error, bound_field, validate_binding};

impl Core {
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

    /// Sets the one content property a placed control carries (step 63.1,
    /// item H56): the label text, button caption, bound source field or
    /// embedded form. Validation runs before the mutation — unknown
    /// fields and unknown subforms answer precisely.
    pub(crate) fn set_control_content(
        &mut self,
        form: &str,
        control: &str,
        value: &str,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let Some(pos) = db
            .forms
            .iter()
            .position(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(FailureCode::NotFound, format!("form {form:?} not found"));
        };
        let Some(control_pos) = db.forms[pos]
            .controls
            .iter()
            .position(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(
                FailureCode::NotFound,
                format!("control {control:?} not found"),
            );
        };
        let source = db.forms[pos].source.clone();
        let kind_is_binding = matches!(
            db.forms[pos].controls[control_pos].kind,
            crate::form::ControlKind::TextBox { .. }
                | crate::form::ControlKind::CheckBox { .. }
                | crate::form::ControlKind::ComboBox { .. }
        );
        let kind_is_subform = matches!(
            db.forms[pos].controls[control_pos].kind,
            crate::form::ControlKind::SubForm { .. }
        );
        if kind_is_binding
            && !value.trim().is_empty()
            && let Some(detail) = binding_error(&source, &db.tables, value)
        {
            return failed(FailureCode::InvalidInput, detail);
        }
        if kind_is_subform {
            let trimmed = value.trim();
            if trimmed.is_empty()
                || !db
                    .forms
                    .iter()
                    .any(|f| f.name.eq_ignore_ascii_case(trimmed))
            {
                return failed(
                    FailureCode::InvalidInput,
                    format!("subform {value:?} not found"),
                );
            }
        }
        let form_name = db.forms[pos].name.clone();
        let control_name = db.forms[pos].controls[control_pos].name.clone();
        let target = &mut db.forms[pos].controls[control_pos];
        match &mut target.kind {
            crate::form::ControlKind::Label { text } => *text = value.to_owned(),
            crate::form::ControlKind::Button { caption } => *caption = value.to_owned(),
            crate::form::ControlKind::TextBox { bound_field }
            | crate::form::ControlKind::CheckBox { bound_field }
            | crate::form::ControlKind::ComboBox { bound_field } => {
                *bound_field = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_owned())
                };
            }
            crate::form::ControlKind::SubForm { form: embedded } => {
                *embedded = value.trim().to_owned();
            }
        }
        vec![Event::ControlContentSet {
            form: form_name,
            control: control_name,
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
}
