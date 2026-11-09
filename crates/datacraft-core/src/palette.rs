//! Designer palettes and field-list placement (step 11.6).
//!
//! Pure mapping and layout math shared by the form and report design
//! views: the Access toolbox, the source field list, unique control names
//! and the next free grid slot. Keeping the decisions here leaves the
//! design views as pure paint (ARCHITECTURE §8) and makes every rule
//! testable without a window.

use crate::canvas::{Rect, snap_rect};
use crate::form::{ControlDef, ControlKind};
use crate::schema::{FieldDef, FieldKind, TableDef};

/// One toolbox entry: an Access control kind with its default size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolboxEntry {
    /// Stable snake_case id (also the UI test-id salt).
    pub id: &'static str,
    /// Button label.
    pub label: &'static str,
    /// Default width in points (grid-aligned).
    pub width: f32,
    /// Default height in points (grid-aligned).
    pub height: f32,
}

/// The Access toolbox in render order.
pub const TOOLBOX: &[ToolboxEntry] = &[
    ToolboxEntry {
        id: "label",
        label: "Label",
        width: 96.0,
        height: 24.0,
    },
    ToolboxEntry {
        id: "text_box",
        label: "Text Box",
        width: 120.0,
        height: 24.0,
    },
    ToolboxEntry {
        id: "check_box",
        label: "Check Box",
        width: 120.0,
        height: 16.0,
    },
    ToolboxEntry {
        id: "combo_box",
        label: "Combo Box",
        width: 120.0,
        height: 24.0,
    },
    ToolboxEntry {
        id: "button",
        label: "Button",
        width: 96.0,
        height: 24.0,
    },
    ToolboxEntry {
        id: "subform",
        label: "Subform",
        width: 200.0,
        height: 120.0,
    },
];

/// One entry of a source table's field list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPaletteEntry {
    /// Field name as bound by the designer.
    pub name: String,
    /// Access UI type name (design grid convention).
    pub kind_display: String,
    /// Whether the toolbox has a matching control (OLE/attachment
    /// columns have none).
    pub bindable: bool,
}

/// The field list of a record source (empty when no source is assigned).
pub fn field_palette(table: &TableDef) -> Vec<FieldPaletteEntry> {
    table
        .fields
        .iter()
        .map(|field| field_palette_entry(&field.name, &field.kind))
        .collect()
}

/// One field-list entry from a snapshot field (the design views' source).
pub fn field_palette_entry(name: &str, kind: &FieldKind) -> FieldPaletteEntry {
    FieldPaletteEntry {
        name: name.to_owned(),
        kind_display: crate::present::field_kind_display(kind),
        bindable: field_control_size(kind).is_some(),
    }
}

/// Default designer control kind for a source field; `None` when the
/// toolbox has no matching control (OLE/attachment columns need a bound
/// object frame, which is not part of the palette).
pub fn field_control_kind(kind: &FieldKind) -> Option<ControlKind> {
    match kind {
        FieldKind::Binary(_) => None,
        FieldKind::Boolean { .. } => Some(ControlKind::CheckBox { bound_field: None }),
        _ => Some(ControlKind::TextBox { bound_field: None }),
    }
}

/// Default size of a source field's bound control.
pub fn field_control_size(kind: &FieldKind) -> Option<(f32, f32)> {
    match field_control_kind(kind)? {
        ControlKind::CheckBox { .. } => Some((120.0, 16.0)),
        _ => Some((120.0, 24.0)),
    }
}

/// The default control of a toolbox entry (unplaced).
pub fn toolbox_control(entry_id: &str, name: &str, rect: Rect) -> Option<ControlDef> {
    let entry = TOOLBOX.iter().find(|entry| entry.id == entry_id)?;
    let kind = match entry.id {
        "label" => ControlKind::Label {
            text: "Label".to_owned(),
        },
        "text_box" => ControlKind::TextBox { bound_field: None },
        "check_box" => ControlKind::CheckBox { bound_field: None },
        "combo_box" => ControlKind::ComboBox {
            bound_field: None,
            row_source: String::new(),
            display_field: String::new(),
        },
        "button" => ControlKind::Button {
            caption: "Button".to_owned(),
            action: None,
        },
        _ => ControlKind::SubForm {
            form: String::new(),
        },
    };
    Some(ControlDef {
        name: name.to_owned(),
        kind,
        rect,
    })
}

/// Default name base of a toolbox entry (Access names controls by kind).
fn toolbox_name_base(entry_id: &str) -> &'static str {
    match entry_id {
        "label" => "Label",
        "text_box" => "Text",
        "check_box" => "Check",
        "combo_box" => "Combo",
        "button" => "Button",
        _ => "Subform",
    }
}

/// Places a toolbox control at the next free slot; `None` for unknown
/// entries or a canvas without space.
pub fn place_toolbox_control(
    entry_id: &str,
    grid: f32,
    bounds: (f32, f32),
    controls: &[ControlDef],
) -> Option<ControlDef> {
    let entry = TOOLBOX.iter().find(|entry| entry.id == entry_id)?;
    let names: Vec<String> = controls
        .iter()
        .map(|control| control.name.clone())
        .collect();
    let name = unique_control_name(toolbox_name_base(entry.id), &names);
    let occupied: Vec<Rect> = controls.iter().map(|control| control.rect).collect();
    let rect = free_slot(grid, (entry.width, entry.height), &occupied, bounds)?;
    toolbox_control(entry.id, &name, rect)
}

/// Prepares the bound control for a record-source field at the next free
/// slot. Errors are precise strings for the command bus.
pub fn place_field_control(
    source: &str,
    tables: &[TableDef],
    field: &str,
    grid: f32,
    bounds: (f32, f32),
    controls: &[ControlDef],
) -> Result<ControlDef, String> {
    if source.is_empty() {
        return Err("set the record source before adding fields".to_owned());
    }
    let Some(table) = tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(source))
    else {
        return Err(format!("record source {source:?} not found"));
    };
    let Some(field_def) = table
        .fields
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(field))
    else {
        return Err(format!("field {field:?} not found in {:?}", table.name));
    };
    let Some((width, height)) = field_control_size(&field_def.kind) else {
        return Err(format!(
            "field {:?} ({}) has no designer control — the toolbox has no bound object frame",
            field_def.name,
            crate::present::field_kind_display(&field_def.kind)
        ));
    };
    let occupied: Vec<Rect> = controls.iter().map(|control| control.rect).collect();
    let Some(rect) = free_slot(grid, (width, height), &occupied, bounds) else {
        return Err(format!("no free space left on the {:?} canvas", table.name));
    };
    let names: Vec<String> = controls
        .iter()
        .map(|control| control.name.clone())
        .collect();
    Ok(bound_control(field_def, &names, rect))
}

/// The bound control for a field, named uniquely against `existing`.
fn bound_control(field: &FieldDef, existing: &[String], rect: Rect) -> ControlDef {
    let bound_field = Some(field.name.clone());
    let kind = match field.kind {
        FieldKind::Boolean { .. } => ControlKind::CheckBox { bound_field },
        _ => ControlKind::TextBox { bound_field },
    };
    ControlDef {
        name: unique_control_name(&field.name, existing),
        kind,
        rect,
    }
}

/// First free `base`/`base2`/`base3`… name (case-insensitive).
pub fn unique_control_name(base: &str, existing: &[String]) -> String {
    let taken = |candidate: &str| {
        existing
            .iter()
            .any(|name| name.eq_ignore_ascii_case(candidate))
    };
    if !taken(base) {
        return base.to_owned();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}{suffix}");
        if !taken(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

/// Next free grid slot (row-major scan) that fits `size` inside `bounds`
/// and overlaps no occupied rect. `None` when the surface is full.
pub fn free_slot(
    grid: f32,
    size: (f32, f32),
    occupied: &[Rect],
    bounds: (f32, f32),
) -> Option<Rect> {
    if !(grid.is_finite() && grid > 0.0) {
        return None;
    }
    let snapped = snap_rect(
        Rect {
            x: 0.0,
            y: 0.0,
            width: size.0,
            height: size.1,
        },
        grid,
    );
    let (width, height) = (snapped.width, snapped.height);
    if width > bounds.0 || height > bounds.1 {
        return None;
    }
    let mut y = 0.0;
    while y + height <= bounds.1 {
        let mut x = 0.0;
        while x + width <= bounds.0 {
            let candidate = Rect {
                x,
                y,
                width,
                height,
            };
            if !occupied.iter().any(|rect| overlaps(candidate, *rect)) {
                return Some(candidate);
            }
            x += grid;
        }
        y += grid;
    }
    None
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_positive_grids_have_no_slots() {
        assert!(free_slot(0.0, (8.0, 8.0), &[], (100.0, 100.0)).is_none());
        assert!(free_slot(f32::NAN, (8.0, 8.0), &[], (100.0, 100.0)).is_none());
    }

    #[test]
    fn slots_skip_occupied_rows() {
        let occupied = Rect {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 24.0,
        };
        let slot = free_slot(8.0, (120.0, 24.0), &[occupied], (640.0, 100.0)).expect("slot");
        assert_eq!(slot.y, 24.0);
        assert_eq!(slot.x, 0.0);
    }
}
