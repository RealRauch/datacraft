//! Foreign-key navigation model (step 87.1, item M103): pure resolution
//! of a datasheet key column's follow targets and the criteria encoder
//! that expresses the source row's key values as filter criteria for the
//! target table. The input is snapshot data — no I/O, no network:
//! declared relationships come from the open port's inventory, inferred
//! references from the loaded schema metadata alone.

use std::fmt;

use datacraft_types::display_value;

use crate::port::{Relationship, RelationshipGroup};
use crate::query_criteria::parse_predicate;
use crate::schema::{FieldDef, FieldKind, FieldValue, TableDef};

/// Direction of a follow target (step 87.1, item M103).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FkDirection {
    /// This table is the child: the target is the referenced (parent)
    /// record.
    Forward,
    /// This table is the parent: the target rows are the referencing
    /// children.
    Reverse,
    /// No declared key covers the column; the reference is inferred from
    /// the `<table>_id` naming convention over the loaded schema.
    Inferred,
}

/// One follow target of a datasheet key column (step 87.1, item M103).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FkFollowTarget {
    /// Forward (to the parent), reverse (referenced by) or inferred.
    pub direction: FkDirection,
    /// The other table's name as the schema carries it.
    pub table: String,
    /// Column pairs in key order: (this table's column, the other
    /// table's column). A composite key lists more than one pair.
    pub pairs: Vec<(String, String)>,
}

/// Every follow target of `(table, column)`: declared forward keys
/// first (precise constraint grouping when the driver carries names,
/// the documented pair fallback otherwise), then the inferred
/// reference, then the reverse referenced-by targets.
pub fn follow_targets(
    table: &str,
    column: &str,
    relationships: &[Relationship],
    groups: Option<&[RelationshipGroup]>,
    tables: &[TableDef],
) -> Vec<FkFollowTarget> {
    let mut targets = forward_targets(table, column, relationships, groups);
    if !targets
        .iter()
        .any(|target| target.direction != FkDirection::Reverse)
        && let Some(inferred) = infer_forward(column, tables)
    {
        targets.push(inferred);
    }
    targets.extend(reverse_targets(table, column, relationships, groups));
    targets
}

/// Declared forward keys of `(table, column)`: constraint groups
/// containing the column, or — without group names — the entries
/// between the same two tables (documented boundary).
fn forward_targets(
    table: &str,
    column: &str,
    relationships: &[Relationship],
    groups: Option<&[RelationshipGroup]>,
) -> Vec<FkFollowTarget> {
    match groups {
        Some(groups) => groups
            .iter()
            .filter(|group| {
                group.table.eq_ignore_ascii_case(table)
                    && group
                        .pairs
                        .iter()
                        .any(|(child, _)| child.eq_ignore_ascii_case(column))
            })
            .map(|group| FkFollowTarget {
                direction: FkDirection::Forward,
                table: group.related_table.clone(),
                pairs: group.pairs.clone(),
            })
            .collect(),
        None => paired_entries(relationships, table, true)
            .into_iter()
            .filter(|(_, pairs)| {
                pairs
                    .iter()
                    .any(|(source, _)| source.eq_ignore_ascii_case(column))
            })
            .map(|(other, pairs)| FkFollowTarget {
                direction: FkDirection::Forward,
                table: other,
                pairs,
            })
            .collect(),
    }
}

/// Reverse targets of `(table, column)`: the referencing children whose
/// constraint references this column — grouped precisely when the
/// driver carries constraint names.
fn reverse_targets(
    table: &str,
    column: &str,
    relationships: &[Relationship],
    groups: Option<&[RelationshipGroup]>,
) -> Vec<FkFollowTarget> {
    match groups {
        Some(groups) => groups
            .iter()
            .filter(|group| {
                group.related_table.eq_ignore_ascii_case(table)
                    && group
                        .pairs
                        .iter()
                        .any(|(_, parent)| parent.eq_ignore_ascii_case(column))
            })
            .map(|group| FkFollowTarget {
                direction: FkDirection::Reverse,
                table: group.table.clone(),
                pairs: group
                    .pairs
                    .iter()
                    .map(|(child, parent)| (parent.clone(), child.clone()))
                    .collect(),
            })
            .collect(),
        None => paired_entries(relationships, table, false)
            .into_iter()
            .filter(|(_, pairs)| {
                pairs
                    .iter()
                    .any(|(source, _)| source.eq_ignore_ascii_case(column))
            })
            .map(|(other, pairs)| FkFollowTarget {
                direction: FkDirection::Reverse,
                table: other,
                pairs,
            })
            .collect(),
    }
}

/// Entries of `table`'s side grouped by the other table in first-seen
/// order. Forward: this table is the child, pairs map (this column,
/// parent column); reverse: this table is the parent, pairs map (this
/// column, child column).
fn paired_entries(
    relationships: &[Relationship],
    table: &str,
    forward: bool,
) -> Vec<(String, Vec<(String, String)>)> {
    let mut grouped: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for entry in relationships {
        let (other, pair) = if forward {
            if !entry.table.eq_ignore_ascii_case(table) {
                continue;
            }
            (
                entry.related_table.clone(),
                (entry.column.clone(), entry.related_column.clone()),
            )
        } else {
            if !entry.related_table.eq_ignore_ascii_case(table) {
                continue;
            }
            (
                entry.table.clone(),
                (entry.related_column.clone(), entry.column.clone()),
            )
        };
        if let Some((_, pairs)) = grouped
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(&other))
        {
            pairs.push(pair);
        } else {
            grouped.push((other, vec![pair]));
        }
    }
    grouped
}

/// The inferred `<table>_id` reference: the column's base name matches
/// a loaded table (case-insensitively, as the singular or the plural
/// form — Access conventions name tables plural) that carries an `id`
/// field. Inference never fires when a declared target exists — the
/// caller checks that first.
fn infer_forward(column: &str, tables: &[TableDef]) -> Option<FkFollowTarget> {
    let base = column.to_lowercase();
    let base = base.strip_suffix("_id")?;
    let target = tables.iter().find(|t| {
        let name = t.name.to_lowercase();
        name == base || name == format!("{base}s")
    })?;
    let id_field = target
        .fields
        .iter()
        .find(|field| field.name.to_lowercase() == "id")?;
    Some(FkFollowTarget {
        direction: FkDirection::Inferred,
        table: target.name.clone(),
        pairs: vec![(column.to_owned(), id_field.name.clone())],
    })
}

/// Why a key tuple cannot become filter criteria (step 87.1, item M103).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FkCriteriaError {
    /// The key column is empty — fully or partially. An empty key has no
    /// record to follow; the message names the column.
    NullKey { column: String },
    /// The value cannot be encoded exactly in the criteria dialect.
    Unsupported { column: String, detail: String },
}

impl fmt::Display for FkCriteriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullKey { column } => write!(
                f,
                "the key column {column} is empty — an empty key (fully or partially) \
                 has no record to follow"
            ),
            Self::Unsupported { column, detail } => {
                write!(f, "cannot follow {column}: {detail}")
            }
        }
    }
}

impl std::error::Error for FkCriteriaError {}

/// The source row's key values as exact-equality filter criteria for the
/// target table: one `(target column, criteria)` per key pair, in key
/// order. A NULL value anywhere in the tuple answers [`FkCriteriaError::NullKey`] — the
/// follow never guesses a partial key; values the dialect cannot express
/// exactly answer [`FkCriteriaError::Unsupported`] instead of degrading to a wrong
/// filter.
pub fn follow_criteria(
    target: &FkFollowTarget,
    values: &[(String, Option<FieldValue>)],
    tables: &[TableDef],
) -> Result<Vec<(String, String)>, FkCriteriaError> {
    let table_def = tables
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(&target.table))
        .ok_or_else(|| FkCriteriaError::Unsupported {
            column: target.table.clone(),
            detail: "the target table is not loaded".to_owned(),
        })?;
    let mut criteria = Vec::new();
    for (source_column, target_column) in &target.pairs {
        let value = values
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(source_column))
            .map(|(_, value)| value)
            .ok_or_else(|| FkCriteriaError::Unsupported {
                column: source_column.clone(),
                detail: "the column value is not loaded".to_owned(),
            })?;
        let value = match value {
            // The UI carries NULL cells as `Some(FieldValue::Null)` — the
            // missing entry and the explicit NULL both answer NullKey
            // (an empty key has no record to follow).
            None | Some(FieldValue::Null) => {
                return Err(FkCriteriaError::NullKey {
                    column: source_column.clone(),
                });
            }
            Some(value) => value,
        };
        let field = table_def
            .fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(target_column))
            .ok_or_else(|| FkCriteriaError::Unsupported {
                column: target_column.clone(),
                detail: format!(
                    "the target table {} has no column {target_column}",
                    table_def.name
                ),
            })?;
        let text =
            encode_criteria(value, field).map_err(|detail| FkCriteriaError::Unsupported {
                column: source_column.clone(),
                detail,
            })?;
        criteria.push((field.name.clone(), text));
    }
    Ok(criteria)
}

/// One key value as an exact-equality criteria the parser reads back
/// (`= literal`). The explicit operator keeps wildcards literal (step
/// 70.1, H58). The candidate is validated by the real criteria parser
/// (step 7.3) — encoder and dialect cannot drift apart: a literal the
/// target column's kind cannot coerce, quote-wrapped text and the
/// dialect's uncoercible families refuse instead of degrading to a
/// wrong filter.
fn encode_criteria(value: &FieldValue, field: &FieldDef) -> Result<String, String> {
    let kind = &field.kind;
    if matches!(
        kind,
        FieldKind::Guid | FieldKind::Binary(_) | FieldKind::Complex
    ) {
        return Err(format!(
            "criteria are not supported on {} columns",
            datacraft_types::field_kind_display(kind)
        ));
    }
    if let FieldKind::AutoNumber(datacraft_types::AutoKind::ReplicationId) = kind {
        return Err("criteria are not supported on replication-id columns".to_owned());
    }
    // The parser strips wrapping quotes before coercing, so such a value
    // would match the wrong record — refuse precisely.
    if let FieldValue::Text(text) = value
        && text.len() >= 2
        && text.starts_with('"')
        && text.ends_with('"')
    {
        return Err(
            "a value wrapped in double quotes cannot be encoded exactly \
             in the criteria dialect"
                .to_owned(),
        );
    }
    if matches!(
        value,
        FieldValue::Guid(_) | FieldValue::Binary(_) | FieldValue::Complex(_)
    ) {
        return Err(format!(
            "{} values cannot be expressed in the criteria dialect",
            value.family()
        ));
    }
    let text = format!("= {}", display_value(value));
    match parse_predicate(&text, &field.name, field) {
        Ok(_) => Ok(text),
        Err(error) => Err(error.to_string()),
    }
}
