//! Placed-control design edits (phase 102, item M133).
//!
//! Forms and reports share [`crate::form::ControlDef`], so the property
//! command addresses the owning object (`Form` or `Report`) and changes
//! exactly one property. Validation runs before the mutation: a refused
//! edit leaves the design document byte-identical.

use crate::ObjectKind;
use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;
use crate::form::{ControlProperty, DesignCommand, SetControlProperties};

impl Core {
    /// Dispatches the placed-control design vocabulary behind
    /// `Command::Design` (phase 102).
    pub(crate) fn design_command(&mut self, command: DesignCommand) -> Vec<Event> {
        match command {
            DesignCommand::SetControlProperties(edit) => self.set_control_properties(&edit),
        }
    }

    /// Sets one property of a placed control (phase 102): name, visibility,
    /// enabled state, bold or alignment. The name is trimmed and must stay
    /// unique case-insensitively within its object; the geometry lives in
    /// the canvas commands, never here.
    pub(crate) fn set_control_properties(&mut self, edit: &SetControlProperties) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        let (object_name, controls) = match edit.object.kind {
            ObjectKind::Form => {
                let Some(form) = db
                    .forms
                    .iter_mut()
                    .find(|form| form.name.eq_ignore_ascii_case(&edit.object.name))
                else {
                    return failed(
                        FailureCode::NotFound,
                        format!("form {:?} not found", edit.object.name),
                    );
                };
                (form.name.clone(), &mut form.controls)
            }
            ObjectKind::Report => {
                let Some(report) = db
                    .reports
                    .iter_mut()
                    .find(|report| report.name.eq_ignore_ascii_case(&edit.object.name))
                else {
                    return failed(
                        FailureCode::NotFound,
                        format!("report {:?} not found", edit.object.name),
                    );
                };
                (report.name.clone(), &mut report.controls)
            }
            other => {
                return failed(
                    FailureCode::InvalidInput,
                    format!(
                        "control properties apply to forms and reports, not to a {}",
                        other.slug()
                    ),
                );
            }
        };
        let Some(pos) = controls
            .iter()
            .position(|control| control.name.eq_ignore_ascii_case(&edit.control))
        else {
            return failed(
                FailureCode::NotFound,
                format!("control {:?} not found", edit.control),
            );
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
            object: crate::ObjectId::new(edit.object.kind, object_name),
            control: target.name.clone(),
        }]
    }
}
