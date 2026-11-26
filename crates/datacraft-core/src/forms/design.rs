//! Placed-control design edits (phase 102, item M133).
//!
//! Forms and reports share [`crate::form::ControlDef`], so the design
//! commands address the owning object (`Form` or `Report`) and change
//! exactly one thing per command. Validation runs before the mutation: a
//! refused edit leaves the design document byte-identical.

use crate::ObjectId;
use crate::ObjectKind;
use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;
use crate::form::{ControlDef, ControlOrder, ControlProperty, DesignCommand, SetControlProperties};
use crate::state::DatabaseState;

impl Core {
    /// Dispatches the placed-control design vocabulary behind
    /// `Command::Design` (phase 102).
    pub(crate) fn design_command(&mut self, command: DesignCommand) -> Vec<Event> {
        match command {
            DesignCommand::SetControlProperties(edit) => self.set_control_properties(&edit),
            DesignCommand::SetControlZOrder {
                object,
                control,
                position,
            } => self.set_control_z_order(&object, &control, position),
            DesignCommand::SetControlTabOrder {
                object,
                control,
                position,
            } => self.set_control_tab_order(&object, &control, position),
        }
    }

    /// Sets one property of a placed control (phase 102): name, visibility,
    /// enabled state, bold or alignment. The name is trimmed and must stay
    /// unique case-insensitively within its object; the geometry lives in
    /// the canvas commands, never here.
    pub(crate) fn set_control_properties(&mut self, edit: &SetControlProperties) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return no_database();
        };
        let (object_name, controls) = match controls_mut(db, &edit.object) {
            Ok(found) => found,
            Err(events) => return events,
        };
        let Some(pos) = find_control(controls, &edit.control) else {
            return control_not_found(&edit.control);
        };
        // Validate the whole edit before touching the control: a refused
        // rename or property leaves the design unchanged.
        let renamed = match &edit.property {
            ControlProperty::Name(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return failed(
                        FailureCode::InvalidInput,
                        "control names must not be empty or whitespace-only",
                    );
                }
                if controls.iter().enumerate().any(|(index, control)| {
                    index != pos && control.name.eq_ignore_ascii_case(trimmed)
                }) {
                    return failed(
                        FailureCode::Duplicate,
                        format!("duplicate control name {trimmed:?}"),
                    );
                }
                Some(trimmed.to_owned())
            }
            _ => None,
        };
        let target = &mut controls[pos];
        match &edit.property {
            ControlProperty::Name(_) => {
                // Sound: every name edit above produced a validated value.
                if let Some(name) = renamed {
                    target.name = name;
                }
            }
            ControlProperty::Visible(visible) => target.properties.visible = *visible,
            ControlProperty::Enabled(enabled) => target.properties.enabled = *enabled,
            ControlProperty::Bold(bold) => target.properties.bold = *bold,
            ControlProperty::Align(align) => target.properties.align = *align,
        }
        vec![Event::ControlPropertiesSet {
            object: ObjectId::new(edit.object.kind, object_name),
            control: target.name.clone(),
        }]
    }

    /// Moves one control within the paint order (phase 102, step 102.3):
    /// the control vector order is what the canvas and the print preview
    /// walk, so a front control paints (and prints) later.
    pub(crate) fn set_control_z_order(
        &mut self,
        object: &ObjectId,
        control: &str,
        position: usize,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return no_database();
        };
        let (object_name, controls) = match controls_mut(db, object) {
            Ok(found) => found,
            Err(events) => return events,
        };
        let Some(index) = find_control(controls, control) else {
            return control_not_found(control);
        };
        let target = position.min(controls.len() - 1);
        if index != target {
            let moved = controls.remove(index);
            controls.insert(target, moved);
        }
        let name = controls[target].name.clone();
        vec![Event::ControlOrderSet {
            object: ObjectId::new(object.kind, object_name),
            control: name,
            order: ControlOrder::Z,
            position: target,
        }]
    }

    /// Moves one control within the tab order (phase 102, step 102.3) and
    /// renumbers `tab_index` densely; the paint order is untouched.
    pub(crate) fn set_control_tab_order(
        &mut self,
        object: &ObjectId,
        control: &str,
        position: usize,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return no_database();
        };
        let (object_name, controls) = match controls_mut(db, object) {
            Ok(found) => found,
            Err(events) => return events,
        };
        let Some(index) = find_control(controls, control) else {
            return control_not_found(control);
        };
        // The current tab list without the moved control, still in its
        // (tab_index, declaration) order; the target clamps to the end.
        let mut order: Vec<usize> = (0..controls.len()).filter(|&i| i != index).collect();
        order.sort_by_key(|&i| (controls[i].properties.tab_index, i));
        let target = position.min(order.len());
        order.insert(target, index);
        for (rank, &i) in order.iter().enumerate() {
            controls[i].properties.tab_index = rank as u32;
        }
        let name = controls[index].name.clone();
        vec![Event::ControlOrderSet {
            object: ObjectId::new(object.kind, object_name),
            control: name,
            order: ControlOrder::Tab,
            position: target,
        }]
    }
}

fn no_database() -> Vec<Event> {
    failed(
        FailureCode::NoDatabase,
        "no database is open — open a database first",
    )
}

fn control_not_found(control: &str) -> Vec<Event> {
    failed(
        FailureCode::NotFound,
        format!("control {control:?} not found"),
    )
}

/// Resolves the mutable control list of a design object (`Form` or
/// `Report`); anything else answers the kind boundary precisely.
fn controls_mut<'a>(
    db: &'a mut DatabaseState,
    object: &ObjectId,
) -> Result<(String, &'a mut Vec<ControlDef>), Vec<Event>> {
    match object.kind {
        ObjectKind::Form => {
            let Some(form) = db
                .forms
                .iter_mut()
                .find(|form| form.name.eq_ignore_ascii_case(&object.name))
            else {
                return Err(failed(
                    FailureCode::NotFound,
                    format!("form {:?} not found", object.name),
                ));
            };
            Ok((form.name.clone(), &mut form.controls))
        }
        ObjectKind::Report => {
            let Some(report) = db
                .reports
                .iter_mut()
                .find(|report| report.name.eq_ignore_ascii_case(&object.name))
            else {
                return Err(failed(
                    FailureCode::NotFound,
                    format!("report {:?} not found", object.name),
                ));
            };
            Ok((report.name.clone(), &mut report.controls))
        }
        other => Err(failed(
            FailureCode::InvalidInput,
            format!(
                "control properties apply to forms and reports, not to a {}",
                other.slug()
            ),
        )),
    }
}

fn find_control(controls: &[ControlDef], name: &str) -> Option<usize> {
    controls
        .iter()
        .position(|control| control.name.eq_ignore_ascii_case(name))
}
