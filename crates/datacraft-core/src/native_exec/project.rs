//! Projection, criteria and ordering of native query execution
//! (step 27.1, item M12).
//!
//! Expressions are parsed by the phase-15 expression service (the Access
//! grammar) and evaluated against a scope that carries qualified entries
//! plus bare entries for field names unique across the FROM tables —
//! ambiguous bare names are refused, never resolved by insertion order.

use std::collections::{BTreeMap, BTreeSet};

use crate::expr::{Expression, Scope};
use crate::port::NativeQueryColumn;
use crate::query_engine::QueryResult;
use crate::schema::{FieldValue, TableDef};

use super::NativeExecError;
use super::join::PartialRow;

/// One output column of the execution.
#[derive(Debug, Clone)]
pub(crate) enum Projection {
    /// A plain field reference resolved to its table position and column.
    Field {
        /// FROM position.
        position: usize,
        /// Column index in that table.
        column: usize,
        /// Output column name.
        output: String,
    },
    /// One field of a `Table.*` expansion.
    Star {
        /// FROM position.
        position: usize,
        /// Column index in that table.
        column: usize,
        /// Output column name.
        output: String,
    },
}

impl Projection {
    pub(crate) fn output(&self) -> &str {
        match self {
            Self::Field { output, .. } | Self::Star { output, .. } => output,
        }
    }

    fn value(&self, row: &PartialRow) -> FieldValue {
        match self {
            Self::Field {
                position, column, ..
            }
            | Self::Star {
                position, column, ..
            } => row.value(*position, *column),
        }
    }
}

/// One parsed ORDER BY key, resolved to its table position and column.
#[derive(Debug, Clone)]
pub(crate) struct OrderKey {
    /// FROM position.
    pub(crate) position: usize,
    /// Column index in that table.
    pub(crate) column: usize,
    /// Descending flag.
    pub(crate) descending: bool,
}

/// Bare field names that occur in exactly one source table.
pub(crate) fn unique_bare_fields(from_tables: &[&TableDef]) -> BTreeSet<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for table in from_tables {
        for field in &table.fields {
            *counts.entry(field.name.to_lowercase()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(name, _)| name)
        .collect()
}

/// Parses one stored expression and whitelists its identifiers.
pub(crate) fn parse_expression(
    source: &str,
    from_tables: &[&TableDef],
) -> Result<Expression, NativeExecError> {
    let expression =
        crate::expr::parse(source).map_err(|error| NativeExecError::BadExpression {
            source: source.to_owned(),
            detail: error.to_string(),
        })?;
    validate_expression(&expression, from_tables)?;
    Ok(expression)
}

fn validate_expression(
    expression: &Expression,
    from_tables: &[&TableDef],
) -> Result<(), NativeExecError> {
    match expression {
        Expression::Literal(_) => Ok(()),
        Expression::Field {
            table: Some(table),
            name,
        } => {
            let position =
                from_position(from_tables, table).ok_or_else(|| NativeExecError::UnknownTable {
                    table: table.clone(),
                })?;
            field_position(from_tables[position], name).ok_or_else(|| {
                NativeExecError::UnknownField {
                    table: Some(table.clone()),
                    field: name.clone(),
                }
            })?;
            Ok(())
        }
        Expression::Field { table: None, name } => {
            let matches = from_tables
                .iter()
                .filter(|table| field_position(table, name).is_some())
                .count();
            match matches {
                0 => Err(NativeExecError::UnknownField {
                    table: None,
                    field: name.clone(),
                }),
                1 => Ok(()),
                _ => Err(NativeExecError::AmbiguousField {
                    field: name.clone(),
                }),
            }
        }
        Expression::Unary(_, operand) => validate_expression(operand, from_tables),
        Expression::Binary(_, left, right) | Expression::Compare(_, left, right) => {
            validate_expression(left, from_tables)?;
            validate_expression(right, from_tables)
        }
        Expression::Between(operand, low, high) => {
            validate_expression(operand, from_tables)?;
            validate_expression(low, from_tables)?;
            validate_expression(high, from_tables)
        }
        Expression::InList(operand, list) => {
            validate_expression(operand, from_tables)?;
            list.iter()
                .try_for_each(|element| validate_expression(element, from_tables))
        }
        Expression::IsNull { operand, .. } => validate_expression(operand, from_tables),
        Expression::Call { args, .. } => args
            .iter()
            .try_for_each(|argument| validate_expression(argument, from_tables)),
    }
}

/// Builds the evaluation scope: qualified entries plus unique bare names.
pub(crate) fn scope_for(
    row: &PartialRow,
    from_tables: &[&TableDef],
    unique_bare: &BTreeSet<String>,
) -> Scope {
    let mut scope = Scope::new();
    for (position, table) in from_tables.iter().enumerate() {
        for (column, field) in table.fields.iter().enumerate() {
            // A table that an outer join null-extended is still in scope:
            // every field resolves to Null instead of "not found".
            let value = row.value(position, column);
            scope.set_field(table.name.as_str(), field.name.as_str(), value.clone());
            if unique_bare.contains(&field.name.to_lowercase()) {
                scope.set(field.name.as_str(), value);
            }
        }
    }
    scope
}

/// Applies every WHERE expression (implicit AND); only `True` keeps a row.
pub(crate) fn filter_rows(
    rows: Vec<PartialRow>,
    filters: &[(String, Expression)],
    from_tables: &[&TableDef],
    unique_bare: &BTreeSet<String>,
) -> Result<Vec<PartialRow>, NativeExecError> {
    let mut kept = Vec::new();
    for row in rows {
        let scope = scope_for(&row, from_tables, unique_bare);
        let mut keep = true;
        for (source, expression) in filters {
            match expression.evaluate(&scope) {
                Ok(FieldValue::Boolean(true)) => {}
                Ok(_) => {
                    keep = false;
                    break;
                }
                Err(error) => {
                    return Err(NativeExecError::BadExpression {
                        source: source.clone(),
                        detail: error.to_string(),
                    });
                }
            }
        }
        if keep {
            kept.push(row);
        }
    }
    Ok(kept)
}

/// Sorts the joined rows by every ORDER BY key (stable, leftmost first).
pub(crate) fn sort_rows(rows: &mut [PartialRow], orders: &[OrderKey]) {
    if orders.is_empty() {
        return;
    }
    rows.sort_by(|left, right| {
        for order in orders {
            let ordering = crate::query_criteria::compare(
                &left.value(order.position, order.column),
                &right.value(order.position, order.column),
            )
            .unwrap_or(std::cmp::Ordering::Equal);
            let ordering = if order.descending {
                ordering.reverse()
            } else {
                ordering
            };
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// Expands one decoded column into projections.
pub(crate) fn parse_column(
    column: &NativeQueryColumn,
    from_tables: &[&TableDef],
) -> Result<Vec<Projection>, NativeExecError> {
    let Some(source) = column.expression.as_deref().or(column.field.as_deref()) else {
        return super::unsupported("native query column without a field reference".to_owned());
    };
    if let Some(prefix) = wildcard_prefix(source) {
        return expand_wildcard(prefix.as_deref(), from_tables);
    }
    let expression = parse_expression(source, from_tables)?;
    let Expression::Field { name, .. } = &expression else {
        return super::unsupported(format!(
            "column {source:?} is computed — only plain field references and table.* wildcards \
             execute"
        ));
    };
    let Some((position, column_index)) = resolve_field(&expression, from_tables) else {
        return super::unsupported(format!("column {source:?} does not resolve to one field"));
    };
    let output = match &column.field {
        Some(field) if !field.contains('.') && !field.starts_with('[') => field.clone(),
        _ => name.clone(),
    };
    Ok(vec![Projection::Field {
        position,
        column: column_index,
        output,
    }])
}

fn expand_wildcard(
    prefix: Option<&str>,
    from_tables: &[&TableDef],
) -> Result<Vec<Projection>, NativeExecError> {
    let mut out = Vec::new();
    match prefix {
        None => {
            for (position, table) in from_tables.iter().enumerate() {
                push_star(&mut out, position, table);
            }
        }
        Some(name) => {
            let position =
                from_position(from_tables, name).ok_or_else(|| NativeExecError::UnknownTable {
                    table: name.to_owned(),
                })?;
            push_star(&mut out, position, from_tables[position]);
        }
    }
    Ok(out)
}

fn push_star(out: &mut Vec<Projection>, position: usize, table: &TableDef) {
    for (column, field) in table.fields.iter().enumerate() {
        out.push(Projection::Star {
            position,
            column,
            output: field.name.clone(),
        });
    }
}

/// Returns the table prefix of `Table.*` / `[Table].*`; `Some(None)` is the
/// bare `*` (all FROM tables).
fn wildcard_prefix(source: &str) -> Option<Option<String>> {
    let trimmed = source.trim();
    if trimmed == "*" {
        return Some(None);
    }
    let (prefix, star) = trimmed.rsplit_once('.')?;
    if star.trim() != "*" {
        return None;
    }
    let prefix = prefix.trim();
    let prefix = prefix.trim_start_matches('[').trim_end_matches(']').trim();
    if prefix.is_empty() {
        Some(None)
    } else {
        Some(Some(prefix.to_owned()))
    }
}

pub(crate) fn from_position(from_tables: &[&TableDef], name: &str) -> Option<usize> {
    from_tables
        .iter()
        .position(|table| table.name.eq_ignore_ascii_case(name))
}

fn field_position(table: &TableDef, name: &str) -> Option<usize> {
    table
        .fields
        .iter()
        .position(|field| field.name.eq_ignore_ascii_case(name))
}

/// Projects the joined rows into the result grid.
pub(crate) fn project(rows: &[PartialRow], projections: &[Projection]) -> QueryResult {
    let columns: Vec<String> = projections
        .iter()
        .map(|projection| projection.output().to_owned())
        .collect();
    let rows = rows
        .iter()
        .map(|row| {
            projections
                .iter()
                .map(|projection| projection.value(row))
                .collect()
        })
        .collect();
    QueryResult { columns, rows }
}

/// Resolves a field reference to (FROM position, column); ambiguous bare
/// names and non-field expressions return `None`.
pub(crate) fn resolve_field(
    expression: &Expression,
    from_tables: &[&TableDef],
) -> Option<(usize, usize)> {
    match expression {
        Expression::Field {
            table: Some(table),
            name,
        } => {
            let position = from_position(from_tables, table)?;
            let column = field_position(from_tables[position], name)?;
            Some((position, column))
        }
        Expression::Field { table: None, name } => {
            let mut found = None;
            for (position, table) in from_tables.iter().enumerate() {
                if let Some(column) = field_position(table, name) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some((position, column));
                }
            }
            found
        }
        _ => None,
    }
}
