//! Display-order computation for the table datasheet (step 44.5, item
//! M54). The sort/filter view state is core state (see
//! `SetTableSort`/`SetTableFilter`/`ClearTableFilters`); this helper
//! turns it into a display→cache index map so every client — GUI, CLI,
//! MCP — renders and addresses the same order without re-implementing
//! the row math (the engine's own predicates and typed compare do the
//! work).

use std::collections::BTreeMap;

use crate::qbe::SortOrder;
use datacraft_types::compare;

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
    if let Some((field, direction)) = sort {
        let index = column_of(field)?;
        let descending = direction == SortOrder::Descending;
        order.sort_by(|&a, &b| {
            let ordering =
                compare(&rows[a][index], &rows[b][index]).unwrap_or(std::cmp::Ordering::Equal);
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    Ok(order)
}
