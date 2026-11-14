//! Mapping from jetdb's form/report properties to sidecar designs.
//!
//! The jetdb types are converted into the internal [`RawObject`] vocabulary
//! first, so the mapping rules are unit-testable without constructing
//! third-party types (their structs are `#[non_exhaustive]`).

use datacraft_core::canvas::Rect;
use datacraft_core::form::{CanvasSize, ControlDef, ControlKind, FormDef};
use datacraft_core::report::{PageSetup, ReportDef};
use jetdb::form::{self, BlobProperty, BlobValue, FormObjectType, FormProperties};

use crate::report::{ObjectReport, UnsupportedControl, UnsupportedProperty};

/// Designer snap grid (points); imported rects are not re-snapped.
pub(crate) const GRID: f32 = 8.0;
/// Access stores geometry in twips: 1440 twips = 1 inch = 72 points.
const TWIPS_PER_POINT: f32 = 20.0;

/// One property value, independent of the third-party representation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawValue {
    /// A text property.
    Text(String),
    /// A numeric property; `short` marks the 2-byte representation.
    Number {
        /// Stored value (possibly signed for a 2-byte field).
        value: i64,
        /// Whether the value was stored in two bytes.
        short: bool,
    },
    /// A Yes/No property.
    YesNo(bool),
    /// A Yes/No property holding its non-default state.
    NotDefault,
    /// A value kind outside the mapped subset.
    Other,
}

/// One named property.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawProp {
    /// Known `SaveAsText` key; `None` for an unnamed id.
    pub name: Option<String>,
    /// The value.
    pub value: RawValue,
}

/// One control (or section) as read from the database.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawControl {
    /// Control name.
    pub name: String,
    /// Known control type name (`jetdb::form::control_type_name`).
    pub type_name: Option<String>,
    /// Display label for the report (`TextBox`, `0x1234`, …).
    pub type_label: String,
    /// Properties.
    pub props: Vec<RawProp>,
}

/// One form or report as read from the database.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawObject {
    /// Object name.
    pub name: String,
    /// Whether this is a form (otherwise a report).
    pub is_form: bool,
    /// Form/report-level properties.
    pub props: Vec<RawProp>,
    /// Sections and controls in blob order.
    pub controls: Vec<RawControl>,
}

impl RawObject {
    /// Converts jetdb's property model into the internal vocabulary.
    pub(crate) fn from_jetdb(object: FormProperties) -> Self {
        Self {
            name: object.form_name,
            is_form: matches!(object.object_type, FormObjectType::Form),
            props: object.properties.iter().map(raw_prop).collect(),
            controls: object
                .controls
                .iter()
                .map(|control| RawControl {
                    name: control.name.clone(),
                    type_name: form::control_type_name(control.type_code).map(str::to_owned),
                    type_label: form::control_type_label(control.type_code),
                    props: control.properties.iter().map(raw_prop).collect(),
                })
                .collect(),
        }
    }
}

fn raw_prop(prop: &BlobProperty) -> RawProp {
    RawProp {
        name: prop.name().map(str::to_owned),
        value: match &prop.value {
            BlobValue::Text(value) => RawValue::Text(value.clone()),
            BlobValue::Short(value) => RawValue::Number {
                value: i64::from(*value),
                short: true,
            },
            BlobValue::Long(value) => RawValue::Number {
                value: i64::from(*value),
                short: false,
            },
            BlobValue::Double(value) => RawValue::Number {
                value: *value as i64,
                short: false,
            },
            BlobValue::Bool(value) => RawValue::YesNo(*value),
            BlobValue::NotDefault => RawValue::NotDefault,
            _ => RawValue::Other,
        },
    }
}

/// The mapped sidecar design of one object.
#[derive(Debug)]
pub(crate) enum Design {
    /// A form design.
    Form(FormDef),
    /// A report design.
    Report(ReportDef),
}

/// Maps one form/report; the report names every skip. The design is `None`
/// only when the object cannot be represented at all.
pub(crate) fn map_object(
    object: &RawObject,
    tables: &[String],
    queries: &[String],
) -> (Option<Design>, ObjectReport) {
    let mut report = ObjectReport {
        object: object.name.clone(),
        kind: if object.is_form { "form" } else { "report" }.to_owned(),
        imported_controls: 0,
        unsupported_controls: Vec::new(),
        unsupported_properties: Vec::new(),
        ignored_properties: Vec::new(),
    };
    let mut controls = Vec::new();
    let mut section_twips: Option<u16> = None;
    let mut max_right = 0.0f32;
    let mut max_bottom = 0.0f32;
    for control in &object.controls {
        if control.type_name.as_deref().is_some_and(is_section) {
            if let Some(height) = find(&control.props, "Height").and_then(unsigned) {
                section_twips = Some(section_twips.unwrap_or(0).saturating_add(height));
            }
            continue;
        }
        if let Some(def) = map_control(control, &mut report) {
            max_right = max_right.max(def.rect.x + def.rect.width);
            max_bottom = max_bottom.max(def.rect.y + def.rect.height);
            controls.push(def);
        }
    }
    report.imported_controls = controls.len();

    let source = match text_prop(&object.props, "RecordSource") {
        Some(value) if plain_source(&value, tables, queries) => value,
        Some(value) => {
            report.unsupported_properties.push(UnsupportedProperty {
                control: None,
                property: "RecordSource".to_owned(),
                reason: format!("{value:?} is not a plain table or query name"),
            });
            String::new()
        }
        None => String::new(),
    };
    collect_ignored(object, &mut report);

    let design = if object.is_form {
        let default = CanvasSize::default();
        let width = find(&object.props, "Width")
            .and_then(unsigned)
            .map(points)
            .unwrap_or_else(|| {
                if controls.is_empty() {
                    default.width
                } else {
                    max_right
                }
            });
        let height = section_twips.map(points).unwrap_or_else(|| {
            if controls.is_empty() {
                default.height
            } else {
                max_bottom
            }
        });
        Some(Design::Form(FormDef {
            name: object.name.clone(),
            source,
            grid: GRID,
            size: CanvasSize { width, height },
            controls,
        }))
    } else {
        Some(Design::Report(ReportDef {
            name: object.name.clone(),
            source,
            group_by: None,
            grid: GRID,
            page: PageSetup::default(),
            controls,
        }))
    };
    (design, report)
}

fn map_control(control: &RawControl, report: &mut ObjectReport) -> Option<ControlDef> {
    let Some(type_name) = control.type_name.as_deref() else {
        report.unsupported_controls.push(UnsupportedControl {
            name: control.name.clone(),
            control_type: control.type_label.clone(),
        });
        return None;
    };
    if control.name.trim().is_empty() {
        report.unsupported_properties.push(UnsupportedProperty {
            control: None,
            property: "Name".to_owned(),
            reason: "the control has no name".to_owned(),
        });
        return None;
    }
    let kind = match type_name {
        "Label" => ControlKind::Label {
            text: text_prop(&control.props, "Caption").unwrap_or_default(),
        },
        "TextBox" => ControlKind::TextBox {
            bound_field: bound_field(control, report),
        },
        "CheckBox" => ControlKind::CheckBox {
            bound_field: bound_field(control, report),
        },
        "ComboBox" => {
            if find(&control.props, "RowSource").is_some() {
                report.unsupported_properties.push(UnsupportedProperty {
                    control: Some(control.name.clone()),
                    property: "RowSource".to_owned(),
                    reason: "combo row sources are not imported in the first cut".to_owned(),
                });
            }
            ControlKind::ComboBox {
                bound_field: bound_field(control, report),
                row_source: String::new(),
                display_field: String::new(),
            }
        }
        "CommandButton" => ControlKind::Button {
            caption: text_prop(&control.props, "Caption").unwrap_or_default(),
            action: None,
        },
        "SubReport" | "SubForm" => match subform_name(control) {
            Some(form_name) => ControlKind::SubForm { form: form_name },
            None => {
                report.unsupported_properties.push(UnsupportedProperty {
                    control: Some(control.name.clone()),
                    property: "SourceObject".to_owned(),
                    reason: "the subform source is not a plain form name".to_owned(),
                });
                return None;
            }
        },
        other => {
            report.unsupported_controls.push(UnsupportedControl {
                name: control.name.clone(),
                control_type: other.to_owned(),
            });
            return None;
        }
    };
    let rect = map_rect(control, report)?;
    Some(ControlDef {
        name: control.name.clone(),
        kind,
        rect,
    })
}

fn map_rect(control: &RawControl, report: &mut ObjectReport) -> Option<Rect> {
    let mut missing: Vec<&'static str> = Vec::new();
    let mut value = |property: &'static str| -> Option<u16> {
        let found = find(&control.props, property)
            .and_then(unsigned)
            .or_else(|| default_twips(control.type_name.as_deref(), property));
        if found.is_none() {
            missing.push(property);
        }
        found
    };
    let values = (value("Left"), value("Top"), value("Width"), value("Height"));
    let (Some(x), Some(y), Some(width), Some(height)) = values else {
        for property in missing {
            report.unsupported_properties.push(UnsupportedProperty {
                control: Some(control.name.clone()),
                property: property.to_owned(),
                reason: "no value and no confirmed Access default".to_owned(),
            });
        }
        return None;
    };
    if width == 0 || height == 0 {
        report.unsupported_properties.push(UnsupportedProperty {
            control: Some(control.name.clone()),
            property: "Width/Height".to_owned(),
            reason: "the control has no visible extent".to_owned(),
        });
        return None;
    }
    Some(Rect {
        x: points(x),
        y: points(y),
        width: points(width),
        height: points(height),
    })
}

/// Confirmed Access defaults for omitted properties (77.1 fixture).
fn default_twips(type_name: Option<&str>, property: &str) -> Option<u16> {
    match property {
        "Left" | "Top" => Some(0),
        "Width" if type_name == Some("TextBox") => Some(1440),
        _ => None,
    }
}

fn bound_field(control: &RawControl, report: &mut ObjectReport) -> Option<String> {
    let value = text_prop(&control.props, "ControlSource")?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('=') || trimmed.contains(['[', ']']) {
        report.unsupported_properties.push(UnsupportedProperty {
            control: Some(control.name.clone()),
            property: "ControlSource".to_owned(),
            reason: "calculated or bracketed sources are not imported".to_owned(),
        });
        return None;
    }
    Some(trimmed.to_owned())
}

fn subform_name(control: &RawControl) -> Option<String> {
    let value = text_prop(&control.props, "SourceObject")?;
    let mut name = value.trim();
    if name.len() > 5 && name[..5].eq_ignore_ascii_case("form.") {
        name = name[5..].trim();
    }
    if name.is_empty()
        || name.contains(['=', ';', '[', ']'])
        || name.to_ascii_lowercase().starts_with("select ")
    {
        None
    } else {
        Some(name.to_owned())
    }
}

fn plain_source(value: &str, tables: &[String], queries: &[String]) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains(['=', ';', '[', ']', '\n', '\r'])
        || trimmed.to_ascii_lowercase().starts_with("select ")
    {
        return false;
    }
    tables
        .iter()
        .chain(queries)
        .any(|name| name.eq_ignore_ascii_case(trimmed))
}

fn is_section(type_name: &str) -> bool {
    matches!(
        type_name,
        "Detail"
            | "FormHeader"
            | "FormFooter"
            | "PageHeader"
            | "PageFooter"
            | "GroupHeader"
            | "GroupFooter"
            | "ReportHeader"
            | "ReportFooter"
    )
}

fn collect_ignored(object: &RawObject, report: &mut ObjectReport) {
    const OBJECT_CONSUMED: [&str; 2] = ["Width", "RecordSource"];
    const CONTROL_CONSUMED: [&str; 9] = [
        "Name",
        "Left",
        "Top",
        "Width",
        "Height",
        "Caption",
        "ControlSource",
        "RowSource",
        "SourceObject",
    ];
    let mut ignored: Vec<String> = Vec::new();
    for prop in &object.props {
        if let Some(name) = &prop.name
            && !OBJECT_CONSUMED.contains(&name.as_str())
        {
            ignored.push(name.clone());
        }
    }
    for control in &object.controls {
        for prop in &control.props {
            if let Some(name) = &prop.name
                && !CONTROL_CONSUMED.contains(&name.as_str())
            {
                ignored.push(name.clone());
            }
        }
    }
    ignored.sort();
    ignored.dedup();
    report.ignored_properties = ignored;
}

fn find<'a>(props: &'a [RawProp], name: &str) -> Option<&'a RawProp> {
    props.iter().find(|prop| prop.name.as_deref() == Some(name))
}

fn text_prop(props: &[RawProp], name: &str) -> Option<String> {
    match &find(props, name)?.value {
        RawValue::Text(value) => Some(value.clone()),
        _ => None,
    }
}

/// Reads a 2-byte property as unsigned: jetdb renders values above 32767
/// as negative `Short`s, so the sign is reinterpreted, never trusted.
fn unsigned(prop: &RawProp) -> Option<u16> {
    match prop.value {
        RawValue::Number { value, short: true } => Some((value as i32 & 0xFFFF) as u16),
        RawValue::Number {
            value,
            short: false,
        } if (0..=0xFFFF).contains(&value) => Some(value as u16),
        _ => None,
    }
}

fn points(twips: u16) -> f32 {
    f32::from(twips) / TWIPS_PER_POINT
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
