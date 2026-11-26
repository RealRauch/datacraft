//! Display-order computation for the table datasheet (step 44.5, item
//! M54). The sort/filter view state is core state (see
//! `SetTableSort`/`SetTableFilter`/`ClearTableFilters`); this helper
//! turns it into a display→cache index map so every client — GUI, CLI,
//! MCP — renders and addresses the same order without re-implementing
//! the row math (the engine's own predicates and typed compare do the
//! work).

use std::collections::BTreeMap;

use crate::qbe::SortOrder;
use datacraft_types::sort_compare;

use crate::query_criteria::parse_predicate;
use crate::query_engine::EngineError;
use crate::schema::{FieldValue, TableDef};

/// Resolves the display→cache index order for one table's cached rows:
/// the filters AND together, the survivors sort by the sort field
/// (stable, typed compare) and the identity order comes back without a
/// view model. Unknown fields and unparseable criteria answer precise
/// [`EngineError`]s.
pub fn view_order(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    sort: Option<(&str, SortOrder)>,
    filters: &BTreeMap<String, String>,
    values: &BTreeMap<String, crate::state::ValueFilter>,
) -> Result<Vec<usize>, EngineError> {
    let column_of = |name: &str| -> Result<usize, EngineError> {
        table
            .fields
            .iter()
            .position(|field| field.name.eq_ignore_ascii_case(name))
            .ok_or(EngineError::UnknownField {
                field: name.to_owned(),
                table: table.name.clone(),
            })
    };
    let mut predicates = Vec::new();
    for (field, criteria) in filters {
        let index = column_of(field)?;
        let column = &table.fields[index];
        predicates.push((index, parse_predicate(criteria, field, column)?));
    }
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.retain(|&row| {
        predicates
            .iter()
            .all(|(index, predicate)| predicate.matches(&rows[row][*index]))
    });
    // Picked values (phase 101, step 101.1; M132) match the rendered text.
    for (field, filter) in values {
        let index = column_of(field)?;
        order.retain(|&row| filter.matches(&rows[row][index]));
    }
    if let Some((field, direction)) = sort {
        let index = column_of(field)?;
        let descending = direction == SortOrder::Descending;
        // The total comparator (step 91.5, item M113): NULLs first
        // ascending (last descending through the direction reverse),
        // family rank under mixes — the output order is always defined.
        order.sort_by(|&a, &b| {
            let ordering = sort_compare(&rows[a][index], &rows[b][index]);
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    Ok(order)
}

/// How many distinct values a column's value list carries (phase 101,
/// step 101.1; item M132). The list stays bounded so a high-cardinality
/// column cannot stall the dialog; `DistinctValues::truncated` says so.
pub const VALUE_LIST_LIMIT: usize = 100;

/// One column's distinct values for the filter-by-value dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctValues {
    /// Distinct rendered texts in the datasheet's total order.
    pub values: Vec<String>,
    /// Whether the column carries a `Null` cell (the list's blank entry).
    pub blank: bool,
    /// Whether more distinct values exist than [`VALUE_LIST_LIMIT`].
    pub truncated: bool,
}

/// Collects one column's distinct rendered values (phase 101, step 101.1;
/// item M132). Cells that render alike share an entry — the filter
/// matches the text the user picked — and the order is the datasheet's
/// total comparator, so two runs of the same data produce the same list.
pub fn distinct_values(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    field: &str,
) -> Result<DistinctValues, EngineError> {
    let index = table
        .fields
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .ok_or(EngineError::UnknownField {
            field: field.to_owned(),
            table: table.name.clone(),
        })?;
    let mut blank = false;
    let mut seen: std::collections::BTreeMap<String, FieldValue> =
        std::collections::BTreeMap::new();
    for row in rows {
        let value = &row[index];
        if value == &FieldValue::Null {
            blank = true;
            continue;
        }
        seen.entry(datacraft_types::display_value(value))
            .or_insert_with(|| value.clone());
    }
    let mut rendered: Vec<(String, FieldValue)> = seen.into_iter().collect();
    rendered.sort_by(|(_, a), (_, b)| sort_compare(a, b));
    let truncated = rendered.len() > VALUE_LIST_LIMIT;
    rendered.truncate(VALUE_LIST_LIMIT);
    Ok(DistinctValues {
        values: rendered.into_iter().map(|(text, _)| text).collect(),
        blank,
        truncated,
    })
}
