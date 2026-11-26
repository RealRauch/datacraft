//! Form design documents (step 8.1, item H28; ADR-1).
//!
//! Forms are DataCraft design objects — they live in the sidecar, not in
//! the Jet/ACE file (ARCHITECTURE §7 ADR-1). Controls arrive with step
//! 8.2's control kinds; the skeleton carries name, snap grid and the
//! control list.

use serde::{Deserialize, Serialize};

use crate::ObjectId;
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

/// Horizontal text alignment inside a control's rect (phase 102, M133).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    /// Flush left (the Access default).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Flush right.
    Right,
}

/// Editable presentation properties of a placed control (phase 102, item
/// M133): the sidecar property sheet beside name and geometry. The core
/// owns the defaults and validates every edit; the designers, the form
/// runtime and the print preview honour them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlProperties {
    /// Whether the control renders; a hidden control still occupies its
    /// rect in the designer and renders nothing at runtime or in print.
    pub visible: bool,
    /// Whether a runtime control accepts input (labels ignore it).
    pub enabled: bool,
    /// Bold text rendering (the SemiBold face).
    pub bold: bool,
    /// Horizontal text alignment.
    pub align: TextAlign,
    /// Focus order ("Tab Index"): controls are visited in ascending order,
    /// ties by declaration order. The editors renumber it densely; the
    /// paint order stays the control vector order (phase 102, step 102.3).
    pub tab_index: u32,
}

impl Default for ControlProperties {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            bold: false,
            align: TextAlign::Left,
            tab_index: 0,
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
    /// Presentation properties (phase 102); older sidecars default them.
    #[serde(default)]
    pub properties: ControlProperties,
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

/// One placed-control edit (phase 102, item M133). Both designers carry
/// the shared [`ControlDef`], so one payload addresses the owning object
/// (`Form` or `Report`) and the control. Exactly one property changes per
/// command — the panel and the direct-manipulation surfaces never batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetControlProperties {
    /// Owning design object (Form or Report).
    pub object: ObjectId,
    /// Control name (case-insensitive); after a rename this is the old one.
    pub control: String,
    /// The single property edit.
    pub property: ControlProperty,
}

/// The editable properties of a placed control (phase 102). Geometry has
/// its own canvas commands and the kind-specific data keeps its own
/// vocabulary; this set is presentation-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ControlProperty {
    /// Renames the control (trimmed, non-empty, unique in its object).
    Name(String),
    /// Shows or hides the control.
    Visible(bool),
    /// Enables or disables runtime input.
    Enabled(bool),
    /// Toggles bold text.
    Bold(bool),
    /// Sets the text alignment.
    Align(TextAlign),
}

/// The two designer orders of a placed control (phase 102, step 102.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlOrder {
    /// Paint order (back to front) — the control vector order.
    Z,
    /// Focus order — the persisted `tab_index` property.
    Tab,
}

/// Placed-control design edits (phase 102, item M133): the vocabulary
/// behind `Command::Design`. Nested like `Command::View` so the top-level
/// command enum stays below the file-size gate as designer features grow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DesignCommand {
    /// Sets one property of a placed form or report control.
    SetControlProperties(SetControlProperties),
    /// Moves a control within the paint order (z-order); `position` is
    /// zero-based from the back and clamped to the last slot.
    SetControlZOrder {
        /// Owning design object (Form or Report).
        object: ObjectId,
        /// Control name (case-insensitive).
        control: String,
        /// Target position in the paint order (0 = back).
        position: usize,
    },
    /// Moves a control within the tab order; the handler renumbers the
    /// `tab_index` values densely (0 … n-1) afterwards.
    SetControlTabOrder {
        /// Owning design object (Form or Report).
        object: ObjectId,
        /// Control name (case-insensitive).
        control: String,
        /// Target position in the focus order (0 = first), clamped.
        position: usize,
    },
    /// Places a record-source field as a bound control at a drop point
    /// (phase 102, step 102.4): snapped, clamped inside the canvas/page
    /// and named uniquely; the binding is whitelisted against the source.
    PlaceFieldControl {
        /// Owning design object (Form or Report).
        object: ObjectId,
        /// Source field name (case-insensitive).
        field: String,
        /// Drop point in canvas/page points (the control's top-left).
        at: (f32, f32),
    },
}
