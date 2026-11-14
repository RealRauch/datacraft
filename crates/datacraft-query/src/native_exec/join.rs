//! Join machinery of native query execution (step 27.1, item M12).
//!
//! Rows stay partially joined while the FROM tables are bound one by one;
//! a slot that is not bound yet reads as `Null`, which gives outer joins
//! their null extension without a second materialization.

use crate::native_query::{NativeJoin, NativeJoinKind};
use datacraft_expr::Expression;
use datacraft_types::{FieldValue, TableDef};

use super::{NativeExecError, unsupported};

/// A partially joined row: one slot per FROM table.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PartialRow {
    pub(super) cells: Vec<Option<Vec<FieldValue>>>,
}

impl PartialRow {
    /// Value at (FROM position, column); unbound or short slots read Null.
    pub(crate) fn value(&self, position: usize, column: usize) -> FieldValue {
        match &self.cells[position] {
            Some(values) => values.get(column).cloned().unwrap_or(FieldValue::Null),
            None => FieldValue::Null,
        }
    }
}

/// One parsed equality join between two FROM positions.
#[derive(Debug, Clone)]
pub(crate) struct JoinPlan {
    /// The stored expression (for precise errors).
    pub(crate) source: String,
    /// Left operand: FROM position and column.
    pub(crate) a: (usize, usize),
    /// Right operand: FROM position and column.
    pub(crate) b: (usize, usize),
    /// Join flavor.
    pub(crate) kind: NativeJoinKind,
}

pub(crate) fn parse(
    join: &NativeJoin,
    from_tables: &[&TableDef],
) -> Result<JoinPlan, NativeExecError> {
    let expression = datacraft_expr::parse(&join.expression).map_err(|error| {
        NativeExecError::BadExpression {
            source: join.expression.clone(),
            detail: error.to_string(),
        }
    })?;
    let Expression::Compare(datacraft_expr::CompareOp::Equal, left, right) = expression else {
        return unsupported(format!(
            "join {:?} is outside the executable subset — only equality joins execute",
            join.expression
        ));
    };
    let (
        Expression::Field {
            table: left_table,
            name: left_name,
        },
        Expression::Field {
            table: right_table,
            name: right_name,
        },
    ) = (*left, *right)
    else {
        return unsupported(format!(
            "join {:?}: only plain field references execute",
            join.expression
        ));
    };
    let (Some(left_table), Some(right_table)) = (left_table, right_table) else {
        return unsupported(format!(
            "join {:?}: join operands must be qualified fields (table.column)",
            join.expression
        ));
    };
    let a_pos =
        from_position(from_tables, &left_table).ok_or_else(|| NativeExecError::UnknownTable {
            table: left_table.clone(),
        })?;
    let b_pos =
        from_position(from_tables, &right_table).ok_or_else(|| NativeExecError::UnknownTable {
            table: right_table.clone(),
        })?;
    let a_col =
        field_position(from_tables[a_pos], &left_name).ok_or(NativeExecError::UnknownField {
            table: Some(left_table),
            field: left_name,
        })?;
    let b_col =
        field_position(from_tables[b_pos], &right_name).ok_or(NativeExecError::UnknownField {
            table: Some(right_table),
            field: right_name,
        })?;
    Ok(JoinPlan {
        source: join.expression.clone(),
        a: (a_pos, a_col),
        b: (b_pos, b_col),
        kind: join.kind,
    })
}

/// Joins one unbound table onto the partial rows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bind_new(
    rows: Vec<PartialRow>,
    new_table: &TableDef,
    new_rows: &[Vec<FieldValue>],
    new_pos: usize,
    new_col: usize,
    bound_pos: usize,
    bound_col: usize,
    kind: NativeJoinKind,
    new_is_a: bool,
    limit: usize,
) -> Result<Vec<PartialRow>, NativeExecError> {
    let (keep_bound, keep_new) = match kind {
        NativeJoinKind::Inner => (false, false),
        NativeJoinKind::Left if new_is_a => (false, true),
        NativeJoinKind::Left => (true, false),
        NativeJoinKind::Right if new_is_a => (true, false),
        NativeJoinKind::Right => (false, true),
    };
    let width = rows
        .first()
        .map_or(new_pos + 1, |row| row.cells.len())
        .max(new_pos + 1);
    let mut out = Vec::new();
    let mut matched = vec![false; new_rows.len()];
    for row in &rows {
        let bound_value = row.value(bound_pos, bound_col);
        let mut any = false;
        for (index, candidate) in new_rows.iter().enumerate() {
            if equal(&bound_value, &candidate[new_col]) {
                let mut next = row.clone();
                next.cells[new_pos] = Some(candidate.clone());
                out.push(next);
                matched[index] = true;
                any = true;
                check_limit(out.len(), limit)?;
            }
        }
        if !any && keep_bound {
            let mut next = row.clone();
            next.cells[new_pos] = Some(vec![FieldValue::Null; new_table.fields.len()]);
            out.push(next);
            check_limit(out.len(), limit)?;
        }
    }
    if keep_new {
        for (index, candidate) in new_rows.iter().enumerate() {
            if matched[index] {
                continue;
            }
            let mut next = PartialRow {
                cells: vec![None; width],
            };
            next.cells[new_pos] = Some(candidate.clone());
            out.push(next);
            check_limit(out.len(), limit)?;
        }
    }
    Ok(out)
}

/// Cartesian extension for FROM tables no join binds.
pub(crate) fn cross(
    rows: Vec<PartialRow>,
    position: usize,
    new_rows: &[Vec<FieldValue>],
    limit: usize,
) -> Result<Vec<PartialRow>, NativeExecError> {
    let mut out = Vec::new();
    for row in &rows {
        for candidate in new_rows {
            let mut next = row.clone();
            next.cells[position] = Some(candidate.clone());
            out.push(next);
            check_limit(out.len(), limit)?;
        }
    }
    Ok(out)
}

/// Family-safe equality; `Null` never matches (SQL join semantics).
pub(crate) fn equal(left: &FieldValue, right: &FieldValue) -> bool {
    !matches!(left, FieldValue::Null)
        && !matches!(right, FieldValue::Null)
        && datacraft_types::compare(left, right) == Some(std::cmp::Ordering::Equal)
}

pub(crate) fn check_limit(rows: usize, limit: usize) -> Result<(), NativeExecError> {
    if rows > limit {
        return Err(NativeExecError::TooManyRows { rows, limit });
    }
    Ok(())
}

fn from_position(from_tables: &[&TableDef], name: &str) -> Option<usize> {
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
