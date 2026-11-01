//! In-memory query engine (step 7.3, item H27; decision default 7).
//!
//! No SQL parser: criteria strings parse to typed predicates per column
//! kind, execution filters port rows (AND across columns, OR within a
//! column — SQL NULL never satisfies a predicate), projects the shown
//! columns in grid order and sorts by the sort columns (stable,
//! leftmost-first).

use crate::qbe::{QueryDef, SortOrder};
use crate::query_criteria::{Predicate, compare, parse_predicate};
use crate::schema::FieldValue;
use crate::schema::TableDef;

/// Result of a query execution.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryResult {
    /// Projected column names in grid order.
    pub columns: Vec<String>,
    /// Result rows, positional to `columns`.
    pub rows: Vec<Vec<FieldValue>>,
}

/// Engine errors: whitelist violations and coercion failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// A criteria column is not a field of the source table.
    UnknownField {
        /// The unknown name.
        field: String,
        /// The source table.
        table: String,
    },
    /// A criteria literal does not fit the column's kind.
    BadLiteral {
        /// The column.
        field: String,
        /// The offending criteria text.
        criteria: String,
        /// Concrete reason.
        detail: String,
    },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownField { field, table } => {
                write!(f, "field {field:?} is not a field of {table:?}")
            }
            Self::BadLiteral {
                field,
                criteria,
                detail,
            } => write!(f, "criteria {criteria:?} on {field:?}: {detail}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Executes a query over the source table's rows.
pub fn run_query(
    def: &QueryDef,
    source: &TableDef,
    rows: &[Vec<FieldValue>],
) -> Result<QueryResult, EngineError> {
    let predicates = parse_predicates(def, source)?;
    let filtered: Vec<&Vec<FieldValue>> = rows
        .iter()
        .filter(|row| {
            predicates
                .iter()
                .all(|(index, ors)| ors.iter().any(|p| p.matches(&row[*index])))
        })
        .collect();

    let shown: Vec<(usize, &String)> = def
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.shown)
        .map(|(i, f)| (i, &f.name))
        .collect();
    let columns: Vec<String> = shown.iter().map(|(_, name)| (*name).clone()).collect();

    // Sort on the FULL rows (hidden sort keys are legal), then project.
    let mut filtered = filtered;
    sort_rows(def, source, &mut filtered)?;

    let result_rows: Vec<Vec<FieldValue>> = filtered
        .iter()
        .map(|row| shown.iter().map(|(i, _)| row[*i].clone()).collect())
        .collect();

    Ok(QueryResult {
        columns,
        rows: result_rows,
    })
}

/// Parses all criteria into per-column predicate lists (grid order).
fn parse_predicates(
    def: &QueryDef,
    source: &TableDef,
) -> Result<Vec<(usize, Vec<Predicate>)>, EngineError> {
    def.fields
        .iter()
        .filter(|f| !f.criteria.is_empty())
        .map(|field| {
            let Some((index, column)) = source
                .fields
                .iter()
                .enumerate()
                .find(|(_, c)| c.name.eq_ignore_ascii_case(&field.name))
            else {
                return Err(EngineError::UnknownField {
                    field: field.name.clone(),
                    table: source.name.clone(),
                });
            };
            let ors = field
                .criteria
                .iter()
                .map(|text| parse_predicate(text, &field.name, column))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((index, ors))
        })
        .collect()
}

/// Parses one criteria string: optional operator prefix, then a typed
/// rows (sort keys may be hidden columns).
fn sort_rows(
    def: &QueryDef,
    source: &TableDef,
    rows: &mut [&Vec<FieldValue>],
) -> Result<(), EngineError> {
    let keys: Vec<(usize, bool)> = def
        .fields
        .iter()
        .filter(|f| f.sort != SortOrder::None)
        .map(|f| {
            let index = source
                .fields
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(&f.name))
                .ok_or(EngineError::UnknownField {
                    field: f.name.clone(),
                    table: source.name.clone(),
                })?;
            Ok((index, f.sort == SortOrder::Descending))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Ok(());
    }
    rows.sort_by(|a, b| {
        for (index, descending) in &keys {
            let ordering = compare(&a[*index], &b[*index]).unwrap_or(std::cmp::Ordering::Equal);
            let ordering = if *descending {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qbe::QbeField;
    use crate::schema::FieldValue;
    use crate::schema::TableDef;
    use crate::schema::{FieldDef, FieldKind, NumericKind};

    fn one_row(value: FieldValue) -> Vec<Vec<FieldValue>> {
        vec![vec![value]]
    }

    /// Hidden sort keys sort the full rows before projection.
    #[test]
    fn hidden_sort_keys_apply() {
        let source = TableDef::new(
            "T",
            vec![
                FieldDef::new("name", FieldKind::Text { max_len: 5 }),
                FieldDef::new("rank", FieldKind::Number(NumericKind::LongInteger)),
            ],
        );
        let data = vec![
            vec![FieldValue::Text("b".into()), FieldValue::Integer(1)],
            vec![FieldValue::Text("a".into()), FieldValue::Integer(2)],
        ];
        let def = QueryDef {
            name: "q".into(),
            table: "T".into(),
            fields: vec![QbeField {
                name: "name".into(),
                shown: true,
                sort: SortOrder::Descending,
                criteria: Vec::new(),
            }],
        };
        let mut def = def;
        // A second, hidden sort column (rank ascending) breaks the tie.
        def.fields.push(QbeField {
            name: "rank".into(),
            shown: false,
            sort: SortOrder::Ascending,
            criteria: Vec::new(),
        });
        let result = run_query(&def, &source, &data).expect("runs");
        assert_eq!(result.columns, ["name"]);
        assert_eq!(result.rows[0][0], FieldValue::Text("b".into()));
        assert_eq!(result.rows[1][0], FieldValue::Text("a".into()));
    }

    /// An unknown sort column is a whitelist error (direct engine calls).
    #[test]
    fn unknown_sort_columns_are_whitelist_errors() {
        let source = TableDef::new("T", vec![FieldDef::new("v", FieldKind::Currency)]);
        let def = QueryDef {
            name: "q".into(),
            table: "T".into(),
            fields: vec![QbeField {
                name: "ghost".into(),
                shown: true,
                sort: SortOrder::Ascending,
                criteria: Vec::new(),
            }],
        };
        let err =
            run_query(&def, &source, &one_row(FieldValue::Null)).expect_err("unknown sort key");
        assert!(matches!(err, EngineError::UnknownField { .. }), "{err}");
    }

    /// prefix parsing: each prefix selects its band.
    #[test]
    fn operator_prefixes_select_their_band() {
        let source = TableDef::new("T", vec![FieldDef::new("v", FieldKind::Currency)]);
        let data = one_row(FieldValue::Currency(10_000));
        for (criteria, expected) in [
            ("= 1", 1usize),
            ("<> 1", 0usize),
            ("< 2", 1usize),
            ("<= 1", 1usize),
            ("> 0", 1usize),
            (">= 2", 0usize),
            ("1", 1usize),
        ] {
            let def = QueryDef {
                name: "q".into(),
                table: "T".into(),
                fields: vec![QbeField {
                    name: "v".into(),
                    shown: true,
                    sort: SortOrder::None,
                    criteria: vec![criteria.to_owned()],
                }],
            };
            let result = run_query(&def, &source, &data).expect("runs");
            assert_eq!(result.rows.len(), expected, "criteria {criteria:?}");
        }
    }
}
