//! Native select-query execution (step 27.1, item M12).
//!
//! The MSysQueries decoder (M3) exposes the stored query verbatim; this
//! module executes the qualified subset over materialized port rows:
//! select queries over one or more tables with **equality** joins
//! (inner/left/right), Access boolean WHERE expressions (the expression
//! service's grammar — `And`/`Or`/`Not`, `In`, `Is [Not] Null`, `Like`,
//! `Between`, brackets and double-quoted text), `Table.*` wildcards and
//! ORDER BY entries.
//!
//! Everything outside that subset answers a precise
//! [`NativeExecError::Unsupported`]: other query flavors, GROUP BY/HAVING,
//! parameters, select modifiers (DISTINCT/TOP/PERCENT), computed columns,
//! self-joins and non-equality joins. Identifiers are whitelisted against
//! the schema (LESSONS #5) before any evaluation; ambiguous bare names are
//! refused instead of resolved by insertion order. A deterministic row cap
//! protects the process from crafted cartesian joins (LESSONS #7).

use std::collections::BTreeSet;

use crate::port::{NativeQuery, NativeQueryKind};
use crate::query_engine::QueryResult;
use crate::schema::{FieldValue, TableDef};

mod join;
mod project;

use join::{JoinPlan, PartialRow};
use project::{OrderKey, Projection, filter_rows, parse_column, parse_expression};

/// Deterministic guard against crafted cartesian joins.
pub const MAX_JOINED_ROWS: usize = 1_000_000;

/// Why a native query could not execute; every variant carries a precise,
/// locatable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExecError {
    /// A decoded construct is outside the executable dialect.
    Unsupported {
        /// What is unsupported and why.
        reason: String,
    },
    /// A FROM entry names no source table.
    UnknownTable {
        /// The unknown table.
        table: String,
    },
    /// A field reference names no column of its table.
    UnknownField {
        /// Qualifying table, when the reference was qualified.
        table: Option<String>,
        /// The unknown field.
        field: String,
    },
    /// A bare field name exists in more than one source table.
    AmbiguousField {
        /// The ambiguous field.
        field: String,
    },
    /// An expression does not parse or fails to evaluate.
    BadExpression {
        /// The stored expression source.
        source: String,
        /// Concrete reason.
        detail: String,
    },
    /// The join would materialize more rows than the guard allows.
    TooManyRows {
        /// The capped row count.
        rows: usize,
        /// The configured limit.
        limit: usize,
    },
}

impl std::fmt::Display for NativeExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { reason } => write!(f, "unsupported: {reason}"),
            Self::UnknownTable { table } => {
                write!(f, "table {table:?} not found in the query's sources")
            }
            Self::UnknownField {
                table: Some(table),
                field,
            } => write!(f, "field {field:?} is not a field of {table:?}"),
            Self::UnknownField { table: None, field } => {
                write!(f, "field {field:?} is not a field of any source table")
            }
            Self::AmbiguousField { field } => {
                write!(f, "field {field:?} is ambiguous across the query's tables")
            }
            Self::BadExpression { source, detail } => {
                write!(f, "native query expression {source:?}: {detail}")
            }
            Self::TooManyRows { rows, limit } => write!(
                f,
                "the query would materialize {rows} joined rows — the limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for NativeExecError {}

/// Executes one decoded native query over materialized source rows.
/// `sources` pairs every FROM table with its rows; lookups are
/// case-insensitive.
pub fn run(
    query: &NativeQuery,
    sources: &[(&TableDef, Vec<Vec<FieldValue>>)],
) -> Result<QueryResult, NativeExecError> {
    run_with_limit(query, sources, MAX_JOINED_ROWS)
}

fn run_with_limit(
    query: &NativeQuery,
    sources: &[(&TableDef, Vec<Vec<FieldValue>>)],
    limit: usize,
) -> Result<QueryResult, NativeExecError> {
    validate_query(query)?;

    let mut from_tables: Vec<&TableDef> = Vec::new();
    let mut from_rows: Vec<&[Vec<FieldValue>]> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for name in &query.tables {
        if !seen.insert(name.to_lowercase()) {
            return unsupported(format!(
                "native query {:?}: duplicate table {name:?} (self-joins need aliases and stay \
                 unsupported)",
                query.name
            ));
        }
        let Some((table, rows)) = sources
            .iter()
            .find(|(table, _)| table.name.eq_ignore_ascii_case(name))
        else {
            return Err(NativeExecError::UnknownTable {
                table: name.clone(),
            });
        };
        from_tables.push(*table);
        from_rows.push(rows.as_slice());
    }
    let unique_bare = project::unique_bare_fields(&from_tables);

    let mut bound = vec![false; from_tables.len()];
    bound[0] = true;
    let mut rows: Vec<PartialRow> = from_rows[0]
        .iter()
        .map(|values| {
            let mut cells = vec![None; from_tables.len()];
            cells[0] = Some(values.clone());
            PartialRow { cells }
        })
        .collect();
    join::check_limit(rows.len(), limit)?;

    let mut pending: Vec<JoinPlan> = query
        .joins
        .iter()
        .map(|join| join::parse(join, &from_tables))
        .collect::<Result<_, _>>()?;
    loop {
        let mut applied = false;
        let mut index = 0;
        while index < pending.len() {
            let plan = pending[index].clone();
            let a_bound = bound[plan.a.0];
            let b_bound = bound[plan.b.0];
            match (a_bound, b_bound) {
                (true, true) => {
                    if plan.kind != crate::port::NativeJoinKind::Inner {
                        return unsupported(format!(
                            "outer join {:?} is outside the subset — it must connect a new table",
                            plan.source
                        ));
                    }
                    rows.retain(|row| {
                        join::equal(
                            &row.value(plan.a.0, plan.a.1),
                            &row.value(plan.b.0, plan.b.1),
                        )
                    });
                    pending.remove(index);
                    applied = true;
                }
                (true, false) | (false, true) => {
                    let new_is_a = !a_bound;
                    let (new_pos, new_col, bound_pos, bound_col) = if new_is_a {
                        (plan.a.0, plan.a.1, plan.b.0, plan.b.1)
                    } else {
                        (plan.b.0, plan.b.1, plan.a.0, plan.a.1)
                    };
                    rows = join::bind_new(
                        rows,
                        from_tables[new_pos],
                        from_rows[new_pos],
                        new_pos,
                        new_col,
                        bound_pos,
                        bound_col,
                        plan.kind,
                        new_is_a,
                        limit,
                    )?;
                    bound[new_pos] = true;
                    pending.remove(index);
                    applied = true;
                }
                (false, false) => index += 1,
            }
        }
        if !applied {
            break;
        }
    }
    if let Some(plan) = pending.first() {
        return unsupported(format!(
            "join {:?} does not connect to the FROM tables in their decoded order",
            plan.source
        ));
    }
    for position in 0..from_tables.len() {
        if !bound[position] {
            rows = join::cross(rows, position, from_rows[position], limit)?;
        }
    }

    let mut filters = Vec::with_capacity(query.criteria.len());
    for criterion in &query.criteria {
        let expression = parse_expression(criterion, &from_tables)?;
        filters.push((criterion.clone(), expression));
    }
    rows = filter_rows(rows, &filters, &from_tables, &unique_bare)?;

    let mut orders = Vec::with_capacity(query.order_by.len());
    for order in &query.order_by {
        let expression = parse_expression(&order.field, &from_tables)?;
        if !matches!(expression, crate::expr::Expression::Field { .. }) {
            return unsupported(format!(
                "ORDER BY {:?} is computed — only field references sort",
                order.field
            ));
        }
        orders.push(OrderKey {
            source: order.field.clone(),
            expression,
            descending: order.descending,
        });
    }
    project::sort_rows(&mut rows, &orders, &from_tables, &unique_bare)?;

    let mut projections: Vec<Projection> = Vec::new();
    for column in &query.columns {
        projections.extend(parse_column(column, &from_tables)?);
    }
    if projections.is_empty() {
        return unsupported(format!("native query {:?} projects no columns", query.name));
    }
    project::project(&rows, &projections, &from_tables, &unique_bare)
}

fn validate_query(query: &NativeQuery) -> Result<(), NativeExecError> {
    if query.kind != NativeQueryKind::Select {
        return unsupported(format!(
            "native query {:?} is a {} query — only select queries execute",
            query.name,
            kind_label(query.kind)
        ));
    }
    if !query.group_by.is_empty() || !query.having.is_empty() {
        return unsupported(format!(
            "native query {:?}: group by / having are outside the executable subset",
            query.name
        ));
    }
    if !query.parameters.is_empty() {
        return unsupported(format!(
            "native query {:?}: parameters are outside the executable subset",
            query.name
        ));
    }
    if query.select_flags & !0x01 != 0 {
        return unsupported(format!(
            "native query {:?}: select modifiers (distinct/top/percent) are outside the \
             executable subset",
            query.name
        ));
    }
    if query.tables.is_empty() {
        return unsupported(format!("native query {:?} has no source table", query.name));
    }
    Ok(())
}

fn unsupported<T>(reason: String) -> Result<T, NativeExecError> {
    Err(NativeExecError::Unsupported { reason })
}

fn kind_label(kind: NativeQueryKind) -> &'static str {
    match kind {
        NativeQueryKind::Select => "select",
        NativeQueryKind::Crosstab => "crosstab",
        NativeQueryKind::Delete => "delete",
        NativeQueryKind::Update => "update",
        NativeQueryKind::Append => "append",
        NativeQueryKind::MakeTable => "make-table",
        NativeQueryKind::DataDefinition => "data-definition",
        NativeQueryKind::Passthrough => "pass-through",
        NativeQueryKind::Union => "union",
    }
}

#[cfg(test)]
mod tests;
