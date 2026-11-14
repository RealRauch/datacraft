//! Per-object import report: imported counts, unsupported entries and the
//! names of properties that were deliberately ignored.

use serde::{Deserialize, Serialize};

/// One skipped control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedControl {
    /// Control name as read from the database.
    pub name: String,
    /// Control type label (`jetdb` name or `0xXXXX`).
    #[serde(rename = "type")]
    pub control_type: String,
}

/// One property that prevented an exact import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedProperty {
    /// Control name, or `null` for a form/report-level property.
    pub control: Option<String>,
    /// Property name (`SaveAsText` key or `0xXXXX`).
    pub property: String,
    /// Why the value could not be mapped.
    pub reason: String,
}

/// What happened to one form or report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectReport {
    /// Object name.
    pub object: String,
    /// `"form"` or `"report"`.
    pub kind: String,
    /// Number of controls written into the sidecar document.
    pub imported_controls: usize,
    /// Controls outside the supported subset.
    pub unsupported_controls: Vec<UnsupportedControl>,
    /// Properties that made a mapping inexact.
    pub unsupported_properties: Vec<UnsupportedProperty>,
    /// Known properties that were deliberately not imported (appearance,
    /// events, print settings, captions).
    pub ignored_properties: Vec<String>,
}

/// Report of one import run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// One entry per inspected form/report, in import order.
    pub objects: Vec<ObjectReport>,
}
