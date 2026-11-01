//! Report design documents (step 10.4, item H31; ADR-1).
//!
//! Reports share the form [`ControlKind`] canvas (the palette, snapping
//! and sidecar persistence are proven) but carry **print geometry**: an
//! A4-portrait page with half-inch margins replaces the form's free
//! canvas. The design surface is the page — controls must stay inside it
//! so a report prints exactly what the designer shows.

use serde::{Deserialize, Serialize};

use crate::form::ControlDef;

/// Paper geometry of one report (points; 72 pt = 1 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageSetup {
    /// Page width (A4 portrait: 595 pt).
    pub width: f32,
    /// Page height (A4 portrait: 842 pt).
    pub height: f32,
    /// Left margin (default 36 pt = half inch).
    pub margin_left: f32,
    /// Top margin (default 36 pt).
    pub margin_top: f32,
    /// Right margin (default 36 pt).
    pub margin_right: f32,
    /// Bottom margin (default 36 pt).
    pub margin_bottom: f32,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            margin_left: 36.0,
            margin_top: 36.0,
            margin_right: 36.0,
            margin_bottom: 36.0,
        }
    }
}

impl PageSetup {
    /// Printable width (page minus side margins).
    pub fn printable_width(&self) -> f32 {
        (self.width - self.margin_left - self.margin_right).max(0.0)
    }

    /// Printable height (page minus top and bottom margins).
    pub fn printable_height(&self) -> f32 {
        (self.height - self.margin_top - self.margin_bottom).max(0.0)
    }
}

/// Design document of one report (step 10.4, item H31).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportDef {
    /// Report name (unique in its database, case-insensitive).
    pub name: String,
    /// Source table for bound controls (empty until assigned; whitelisted
    /// by `Command::SetReportSource`).
    #[serde(default)]
    pub source: String,
    /// Grouping field (step 15.2, H41): rows with equal values form a
    /// group whose header and aggregate footer render as bands.
    #[serde(default)]
    pub group_by: Option<String>,
    /// Snap grid in points (Access-like default: 8).
    pub grid: f32,
    /// Print geometry (default A4 portrait, half-inch margins).
    #[serde(default)]
    pub page: PageSetup,
    /// Controls in design order (paint order: back to front).
    pub controls: Vec<ControlDef>,
}

impl ReportDef {
    /// Creates an empty report with print-oriented defaults.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: String::new(),
            group_by: None,
            grid: 8.0,
            page: PageSetup::default(),
            controls: Vec::new(),
        }
    }
}
