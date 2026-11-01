//! Form design documents (step 8.1, item H28; ADR-1).
//!
//! Forms are DataCraft design objects — they live in the sidecar, not in
//! the Jet/ACE file (ARCHITECTURE §7 ADR-1). Controls arrive with step
//! 8.2's control kinds; the skeleton carries name, snap grid and the
//! control list.

use serde::{Deserialize, Serialize};

use crate::canvas::Rect;

/// One control kind (Access palette; kinds grow in step 8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ControlKind {
    /// Static text.
    Label {
        /// The displayed text.
        text: String,
    },
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
    /// Snap grid in points (Access-like default: 8).
    pub grid: f32,
    /// Controls in tab order (paint order: back to front).
    pub controls: Vec<ControlDef>,
}

impl FormDef {
    /// Creates an empty form with the default snap grid.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            grid: 8.0,
            controls: Vec::new(),
        }
    }
}
