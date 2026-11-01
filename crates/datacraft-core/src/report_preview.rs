//! Print-preview pagination (step 10.5, item H31).
//!
//! The report design acts as the page template: one preview page per
//! source record (single-record pages; grouped/aggregate bands stay
//! deferred to the expression service, M1). Layout math is a pure
//! function over the design and the rows (Principle 4) — the UI only
//! paints the returned pages. Values render through the shared
//! presentation layer, so preview text matches the datasheet.

use crate::canvas::Rect;
use crate::form::ControlKind;
use crate::report::ReportDef;
use crate::schema::{FieldValue, TableDef};

/// One positioned render item in page coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewItem {
    /// The rendered text.
    pub text: String,
    /// Page placement (design coordinates, points).
    pub rect: Rect,
}

/// One preview page (the design template with one record's values).
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewPage {
    /// Render items in control (paint) order.
    pub items: Vec<PreviewItem>,
}

/// Paginates a report design over source rows: `max(records, 1)` pages.
/// Bound controls render the record's field through [`crate::present::display_value`]
/// (unknown fields render empty); unbound controls render literally.
pub fn paginate(
    report: &ReportDef,
    source: Option<&TableDef>,
    rows: &[Vec<FieldValue>],
) -> Vec<PreviewPage> {
    let page_count = rows.len().max(1);
    let index_of = |field: &str| {
        source.and_then(|table| {
            table
                .fields
                .iter()
                .position(|f| f.name.to_lowercase() == field.to_lowercase())
        })
    };
    (0..page_count)
        .map(|page| PreviewPage {
            items: report
                .controls
                .iter()
                .map(|control| {
                    let text = match (&control.kind, rows.first().map(Vec::len)) {
                        (ControlKind::Label { text }, _) => text.clone(),
                        (ControlKind::Button { caption }, _) => caption.clone(),
                        (
                            ControlKind::TextBox {
                                bound_field: Some(field),
                            },
                            Some(width),
                        )
                        | (
                            ControlKind::CheckBox {
                                bound_field: Some(field),
                            },
                            Some(width),
                        )
                        | (
                            ControlKind::ComboBox {
                                bound_field: Some(field),
                            },
                            Some(width),
                        ) => match index_of(field) {
                            Some(col) if col < width => match &rows[page][col] {
                                FieldValue::Null => String::new(),
                                value => crate::present::display_value(value),
                            },
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    PreviewItem {
                        text,
                        rect: control.rect,
                    }
                })
                .collect(),
        })
        .collect()
}

impl crate::core::Core {
    /// Builds the print preview of one report: resolves the whitelisted
    /// source, reads its rows through the port and paginates.
    pub fn report_preview(&self, report: &str) -> Result<Vec<PreviewPage>, String> {
        let db = self.state.database.as_ref().ok_or("no database is open")?;
        let def = db
            .reports
            .iter()
            .find(|r| r.name.to_lowercase() == report.to_lowercase())
            .ok_or_else(|| format!("report {report:?} not found"))?;
        let source = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&def.source));
        let rows = match source {
            Some(table) => self
                .port
                .read_rows(
                    &table.name,
                    crate::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .map_err(|e| e.to_string())?,
            None => Vec::new(),
        };
        Ok(paginate(def, source, &rows))
    }
}
