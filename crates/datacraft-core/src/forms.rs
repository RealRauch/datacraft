//! Form and control dispatch (steps 8.1/8.2, items H28/H29).
//!
//! Split from core.rs to keep every file under the size gate; these
//! are plain impl-Core reducers over the form design documents.

use crate::core::failed;
use crate::event::Event;

impl crate::core::Core {
    pub(crate) fn create_form(&mut self, name: &str) -> Vec<Event> {
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
            .forms
            .iter()
            .any(|f| f.name.to_lowercase() == name.to_lowercase())
        {
            return failed(format!("duplicate form name {name:?}"));
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
            return failed("no database is open — open a database first");
        };
        let Some(pos) = db
            .forms
            .iter()
            .position(|f| f.name.to_lowercase() == name.to_lowercase())
        else {
            return failed(format!("form {name:?} not found"));
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
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(format!("form {form:?} not found"));
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
        control.rect = crate::canvas::snap_rect(control.rect, def.grid);
        if control.rect.x + control.rect.width > def.size.width
            || control.rect.y + control.rect.height > def.size.height
        {
            return failed(format!(
                "control {:?} exceeds the canvas bounds ({} x {})",
                control.name, def.size.width, def.size.height
            ));
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
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(format!("form {form:?} not found"));
        };
        let Some(target) = def
            .controls
            .iter_mut()
            .find(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(format!("control {control:?} not found"));
        };
        let snapped = crate::canvas::snap_rect(rect, def.grid);
        if snapped.x + snapped.width > def.size.width
            || snapped.y + snapped.height > def.size.height
        {
            return failed(format!(
                "control {control:?} exceeds the canvas bounds ({} x {})",
                def.size.width, def.size.height
            ));
        }
        target.rect = snapped;
        vec![Event::ControlRectSet {
            form: def.name.clone(),
            control: target.name.clone(),
        }]
    }

    pub(crate) fn delete_control(&mut self, form: &str, control: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .forms
            .iter_mut()
            .find(|f| f.name.to_lowercase() == form.to_lowercase())
        else {
            return failed(format!("form {form:?} not found"));
        };
        let Some(pos) = def
            .controls
            .iter()
            .position(|c| c.name.to_lowercase() == control.to_lowercase())
        else {
            return failed(format!("control {control:?} not found"));
        };
        let canonical = def.controls.remove(pos).name;
        vec![Event::ControlDropped {
            form: def.name.clone(),
            control: canonical,
        }]
    }
}
