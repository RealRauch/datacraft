//! Form design documents (step 8.1, item H28; ADR-1).
//!
//! Forms are DataCraft design objects — they live in the sidecar, not in
//! the Jet/ACE file (ARCHITECTURE §7 ADR-1). Controls arrive with step
//! 8.2's control kinds; the skeleton carries name, snap grid and the
//! control list.

use serde::{Deserialize, Serialize};

use crate::canvas::Rect;

/// One control kind (the Access toolbox palette).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ControlKind {
    /// Static text.
    Label {
        /// The displayed text.
        text: String,
    },
    /// Single-line edit bound to a source field (editable when the
    /// runtime sits on a writable source — ADR-8).
    TextBox {
        /// Bound source field, if any.
        bound_field: Option<String>,
    },
    /// Checkbox bound to a boolean source field.
    CheckBox {
        /// Bound source field, if any.
        bound_field: Option<String>,
    },
    /// Drop-down bound to a source field (ADR-8: table/query row source
    /// plus a display field; free entry follows `LimitToList = No`).
    ComboBox {
        /// Bound source field, if any.
        bound_field: Option<String>,
        /// Row source: a table or saved-query name (empty = no list).
        #[serde(default)]
        row_source: String,
        /// Field or column of the row source that feeds the list.
        #[serde(default)]
        display_field: String,
    },
    /// Command button with an optional action (ADR-8).
    Button {
        /// Button caption.
        caption: String,
        /// The chosen action; `None` keeps the button enabled and a click
        /// answers precisely (Access has no auto-disable).
        #[serde(default)]
        action: Option<ButtonAction>,
    },
    /// Embedded subform.
    SubForm {
        /// The embedded form's name.
        form: String,
    },
}

/// Canvas size in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CanvasSize {
    /// Canvas width.
    pub width: f32,
    /// Canvas height.
    pub height: f32,
}

impl Default for CanvasSize {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 400.0,
        }
    }
}

/// One placed control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlDef {
    /// Control name (unique within its form, case-insensitive).
    pub name: String,
    /// Kind-specific payload.
    pub kind: ControlKind,
    /// Canvas placement (snapped).
    pub rect: Rect,
}

/// Design document of one form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormDef {
    /// Form name (unique in its database, case-insensitive).
    pub name: String,
    /// Source table for bound controls (empty until assigned; whitelisted
    /// by `Command::SetFormSource`).
    #[serde(default)]
    pub source: String,
    /// Snap grid in points (Access-like default: 8).
    pub grid: f32,
    /// Canvas size (default 640 x 400).
    #[serde(default)]
    pub size: CanvasSize,
    /// Controls in tab order (paint order: back to front).
    pub controls: Vec<ControlDef>,
}

impl FormDef {
    /// Creates an empty form with the default snap grid.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: String::new(),
            grid: 8.0,
            size: CanvasSize::default(),
            controls: Vec::new(),
        }
    }
}

/// One command-button action (ADR-8): the fixed bus vocabulary replaces
/// Access's macro/event-procedure model. Macros and VBA stay out (ADR-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ButtonAction {
    /// Opens another form in its runtime view.
    OpenForm {
        /// Target form name (canonicalized at set time).
        form: String,
    },
    /// Opens a report's print preview.
    OpenReport {
        /// Target report name (canonicalized at set time).
        report: String,
    },
    /// Runs a saved (sidecar or native) query.
    RunQuery {
        /// Target query name (canonicalized at set time).
        query: String,
    },
    /// Applies a WHERE predicate to the form's own record set
    /// (`ApplyFilter` semantics; never the datasheet filter).
    ApplyFilter {
        /// Access-compatible predicate.
        criteria: String,
    },
}

/// Kind-specific design data of a placed form control (ADR-8). The command
/// payload validates the variant against the control kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ControlData {
    /// Combo box row source: a table or saved query plus its display
    /// field. Empty strings clear both properties.
    ComboSource {
        /// Table or saved-query name.
        row_source: String,
        /// Display field or column of that source.
        display_field: String,
    },
    /// Command-button action; `None` clears it.
    Button {
        /// The action, if any.
        action: Option<ButtonAction>,
    },
}

/// Payload of `Command::SetFormControlData` (one command for the
/// kind-specific properties, keeping the wire vocabulary additive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetControlData {
    /// Target form name (case-insensitive).
    pub form: String,
    /// Control name (case-insensitive).
    pub control: String,
    /// The new design data.
    pub data: ControlData,
}

/// Payload of `Command::SetFormFilter` (ADR-8): the form-scoped WHERE
/// predicate; `None` or a blank string clears the filter
/// (`ShowAllRecords` semantics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormFilter {
    /// Target form name (case-insensitive).
    pub form: String,
    /// Access-compatible WHERE predicate.
    pub criteria: Option<String>,
}
