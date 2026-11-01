//! Print-preview pagination (steps 10.5/15.2, items H31/H41).
//!
//! The report design acts as the page template. Since the expression
//! service, pagination lives in [`crate::report_bands`]: pages carry a
//! band kind (`Detail`, or the `GroupHeader`/`GroupFooter` bands of a
//! grouped report) and `=expression` controls evaluate per record or per
//! group. Values render through the shared presentation layer, so
//! preview text matches the datasheet.

use crate::canvas::Rect;
use crate::report::ReportDef;
use crate::schema::{FieldValue, TableDef};

/// Which band a preview page renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// One record's values (also the only band of ungrouped reports).
    Detail,
    /// The group's key value, rendered once per group.
    GroupHeader,
    /// The group's aggregate expressions, rendered once per group.
    GroupFooter,
}

/// One positioned render item in page coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewItem {
    /// The rendered text.
    pub text: String,
    /// Page placement (design coordinates, points).
    pub rect: Rect,
}

/// One preview page: the design template filled with one band's values.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewPage {
    /// The band this page renders.
    pub band: Band,
    /// Render items in control (paint) order.
    pub items: Vec<PreviewItem>,
}

/// Paginates a report design over source rows — grouped reports render
/// header/footer bands, ungrouped ones stay single-record pages.
pub fn paginate(
    report: &ReportDef,
    source: Option<&TableDef>,
    rows: &[Vec<FieldValue>],
) -> Result<Vec<PreviewPage>, String> {
    crate::report_bands::paginate(report, source, rows)
}

/// Renders a bound field against one row (unknown fields and `Null`
/// render empty); the expression path lives in `report_bands`.
pub(crate) fn bound_text(
    source: Option<&TableDef>,
    row: Option<&Vec<FieldValue>>,
    field: &str,
) -> String {
    let (Some(source), Some(row)) = (source, row) else {
        return String::new();
    };
    let Some(column) = source
        .fields
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(field))
    else {
        return String::new();
    };
    match row.get(column) {
        Some(FieldValue::Null) | None => String::new(),
        Some(value) => crate::present::display_value(value),
    }
}

impl crate::core::Core {
    /// Builds the print preview of one report: resolves the whitelisted
    /// source, reads its rows through the port and paginates. Failures
    /// are precise (unknown group/aggregate fields, broken expressions).
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
        paginate(def, source, &rows)
    }
}
