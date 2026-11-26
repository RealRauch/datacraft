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

/// One find/replace query over the visible rows (phase 101, step 101.3;
/// item M132).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindRequest {
    /// The needle; a blank needle matches nothing.
    pub text: String,
    /// The text every match becomes.
    pub replace_with: String,
    /// One column to search (case-insensitive); `None` searches every
    /// text-like column.
    pub column: Option<String>,
    /// Whether the comparison is case-sensitive (default: insensitive).
    pub match_case: bool,
    /// Whether the whole cell must equal the needle.
    pub whole_field: bool,
}

/// One match: the cache row, the column index, the cell's text and the
/// text a replacement produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindMatch {
    /// Cache row the cell belongs to.
    pub row: usize,
    /// Column index in the table's field order.
    pub column: usize,
    /// The cell's text as it stands.
    pub cell: String,
    /// The text after replacing every occurrence.
    pub replaced: String,
}

/// Finds the cells of the display order that match (phase 101, step
/// 101.3; item M132). Only short-text and memo columns take part — a
/// replacement is a text edit, and coercing a rendered string back into
/// a number or a date would guess. `Null` cells never match, and an
/// empty needle matches nothing.
pub fn find_matches(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    order: &[usize],
    request: &FindRequest,
) -> Result<Vec<FindMatch>, String> {
    let needle = request.text.trim();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let text_columns: Vec<usize> = match &request.column {
        Some(name) => {
            let index = table
                .fields
                .iter()
                .position(|field| field.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("unknown column {name:?}"))?;
            if !is_text_kind(&table.fields[index].kind) {
                return Err(format!(
                    "column {:?} is not a text column — find/replace works on Short Text and Long Text",
                    table.fields[index].name
                ));
            }
            vec![index]
        }
        None => (0..table.fields.len())
            .filter(|index| is_text_kind(&table.fields[*index].kind))
            .collect(),
    };
    let mut found = Vec::new();
    for &row in order {
        for &column in &text_columns {
            let Some(FieldValue::Text(cell)) = rows.get(row).and_then(|cells| cells.get(column))
            else {
                continue;
            };
            let replaced = if request.whole_field {
                if equal(cell, needle, request.match_case) {
                    request.replace_with.clone()
                } else {
                    continue;
                }
            } else if find_span(cell, needle, request.match_case).is_some() {
                replace_all(cell, needle, &request.replace_with, request.match_case)
            } else {
                continue;
            };
            found.push(FindMatch {
                row,
                column,
                cell: cell.clone(),
                replaced,
            });
        }
    }
    Ok(found)
}

/// Whether a column carries free text (the families find/replace edits).
fn is_text_kind(kind: &datacraft_types::FieldKind) -> bool {
    matches!(
        kind,
        datacraft_types::FieldKind::Text { .. } | datacraft_types::FieldKind::Memo
    )
}

/// Case-folded (or exact) string equality.
fn equal(left: &str, right: &str, match_case: bool) -> bool {
    if match_case {
        left == right
    } else {
        left.to_lowercase() == right.to_lowercase()
    }
}

/// Byte range of the first occurrence, case-folded unless `match_case`.
fn find_span(haystack: &str, needle: &str, match_case: bool) -> Option<(usize, usize)> {
    let hay: Vec<(usize, char)> = haystack.char_indices().collect();
    let wanted: Vec<char> = needle.chars().collect();
    if wanted.is_empty() || wanted.len() > hay.len() {
        return None;
    }
    let same = |left: char, right: char| {
        if match_case {
            left == right
        } else {
            left.to_lowercase().eq(right.to_lowercase())
        }
    };
    (0..=hay.len() - wanted.len()).find_map(|start| {
        let matched = wanted
            .iter()
            .enumerate()
            .all(|(offset, wanted)| same(hay[start + offset].1, *wanted));
        matched.then(|| {
            let begin = hay[start].0;
            let end = hay
                .get(start + wanted.len())
                .map_or(haystack.len(), |(offset, _)| *offset);
            (begin, end)
        })
    })
}

/// Replaces every occurrence in one cell.
fn replace_all(cell: &str, needle: &str, replacement: &str, match_case: bool) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut rest = cell;
    while let Some((start, end)) = find_span(rest, needle, match_case) {
        out.push_str(&rest[..start]);
        out.push_str(replacement);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
