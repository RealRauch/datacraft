//! Referential write parity (phase 104, step 104.2; item M137; roadmap
//! §13(a)).
//!
//! One rule, one source: the check decides whether a child row's values
//! reference an existing parent row for every declared relationship of
//! its table. The Jet writer calls it before a byte is written (SQLite's
//! engine enforces its own foreign keys; this rule gives Jet the same
//! visible outcome), and the write journal's preview calls it so a batch
//! containing an orphan is refused before it can apply.
//!
//! Semantics mirror SQL foreign keys: a `NULL` child value passes (the
//! relationship does not apply), and only the declared column pairs are
//! checked. Deletes are deliberately not part of this rule — cascade
//! behavior is roadmap §13(b), a separate later step.

use std::collections::{BTreeMap, BTreeSet};

use crate::port::Relationship;
use crate::schema::{FieldValue, TableDef};

/// The parent tables a child table's writes must be checked against,
/// de-duplicated, in canonical (sorted) order.
pub fn parent_rows_needed(relationships: &[Relationship], child_table: &str) -> Vec<String> {
    let mut needed = BTreeSet::new();
    for relationship in relationships {
        if relationship.table.eq_ignore_ascii_case(child_table) {
            needed.insert(relationship.related_table.clone());
        }
    }
    needed.into_iter().collect()
}

/// The precise refusal reason when `values` orphans against a declared
/// relationship, or `None` when the row may write. `parents` carries the
/// referenced column's values — one entry per parent row, projected by
/// the caller from the parent table's definition — keyed by lower-cased
/// table name; a parent the caller did not materialize counts as absent
/// (nothing can reference it).
pub fn orphan_reason(
    relationships: &[Relationship],
    child: &TableDef,
    values: &[FieldValue],
    parents: &BTreeMap<String, Vec<Vec<FieldValue>>>,
) -> Option<String> {
    for relationship in relationships {
        if !relationship.table.eq_ignore_ascii_case(&child.name) {
            continue;
        }
        let Some(index) = child
            .fields
            .iter()
            .position(|field| field.name.eq_ignore_ascii_case(&relationship.column))
        else {
            continue;
        };
        let Some(value) = values.get(index) else {
            continue;
        };
        if value == &FieldValue::Null {
            continue;
        }
        let Some(rows) = parents.get(&relationship.related_table.to_lowercase()) else {
            return Some(orphan_text(relationship, value, child));
        };
        let referenced = rows
            .iter()
            .any(|row| row.first().is_some_and(|candidate| candidate == value));
        // A composite relationship is a group of column pairs over the
        // same two tables; each pair carries its own constraint row in
        // the catalog, so the per-pair check here is the declared
        // semantics (the M103 grouping pairs entries between the same
        // tables for navigation, not for enforcement).
        if !referenced {
            return Some(orphan_text(relationship, value, child));
        }
    }
    None
}

/// The precise, locatable refusal text (the port error detail and the
/// journal preview share it).
fn orphan_text(relationship: &Relationship, value: &FieldValue, child: &TableDef) -> String {
    format!(
        "write into {:?}: column {:?} = {} references {:?}.{:?} — no parent row carries that \
         value (roadmap §13(a))",
        child.name,
        relationship.column,
        datacraft_types::display_value_for(value, None),
        relationship.related_table,
        relationship.related_column,
    )
}

impl crate::core::Core {
    /// The shared referential rule over a whole batch (phase 104, step
    /// 104.2; item M137): `Some(reason)` when any insert or update
    /// orphans against a declared relationship. The parent rows are
    /// materialized through the port, only for the tables a declared
    /// relationship references; a dangling relationship (its parent
    /// table is gone) cannot be satisfied and answers the per-row
    /// refusal.
    pub(crate) fn orphan_batch_reason(
        &self,
        def: &TableDef,
        ops: &[crate::journal::WriteOp],
    ) -> Option<String> {
        let db = self.state.database.as_ref()?;
        let relationships = &db.relationships;
        if relationships.is_empty() {
            return None;
        }
        let mut parents: BTreeMap<String, Vec<Vec<FieldValue>>> = BTreeMap::new();
        for parent in parent_rows_needed(relationships, &def.name) {
            let Some(parent_def) = db
                .tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&parent))
                .cloned()
            else {
                continue;
            };
            let rows = self
                .port
                .read_rows(
                    &parent_def.name,
                    crate::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .unwrap_or_default();
            // Project the referenced column: the rule compares one value
            // per parent row. Composite pairs beyond the first declared
            // column stay unenforced (the documented §13(a) boundary —
            // composite keys are rare and the M103 grouping is a
            // navigation concept).
            let referenced = relationships
                .iter()
                .find(|r| r.related_table.eq_ignore_ascii_case(&parent_def.name))
                .map(|r| r.related_column.clone());
            let position = referenced.as_deref().and_then(|column| {
                parent_def
                    .fields
                    .iter()
                    .position(|field| field.name.eq_ignore_ascii_case(column))
            });
            let projected = rows
                .iter()
                .map(|row| {
                    row.get(position.unwrap_or(0))
                        .cloned()
                        .into_iter()
                        .collect::<Vec<_>>()
                })
                .collect();
            parents.insert(parent.to_lowercase(), projected);
        }
        for op in ops {
            let values = match op {
                crate::journal::WriteOp::Insert { values } => values,
                crate::journal::WriteOp::Update { values, .. } => values,
                crate::journal::WriteOp::Delete { .. } => continue,
            };
            if let Some(reason) = orphan_reason(relationships, def, values, &parents) {
                return Some(reason);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, FieldKind, NumericKind};

    fn orders() -> TableDef {
        TableDef::new(
            "Orders",
            vec![FieldDef::new(
                "CustomerId",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        )
    }

    fn relationship() -> Relationship {
        Relationship {
            table: "Orders".to_owned(),
            column: "CustomerId".to_owned(),
            related_table: "Customers".to_owned(),
            related_column: "Id".to_owned(),
        }
    }

    #[test]
    fn the_reason_names_child_column_parent_and_value() {
        let parents = BTreeMap::from([("customers".to_owned(), Vec::new())]);
        let reason = orphan_reason(
            &[relationship()],
            &orders(),
            &[FieldValue::Integer(99)],
            &parents,
        )
        .expect("the orphan refuses");
        assert!(reason.contains("Orders"), "{reason}");
        assert!(reason.contains("CustomerId"), "{reason}");
        assert!(reason.contains("99"), "{reason}");
        assert!(reason.contains("Customers"), "{reason}");
    }

    #[test]
    fn a_missing_child_column_is_skipped_rather_than_guessed() {
        // A dangling relationship (the column was dropped) never refuses.
        let parents = BTreeMap::new();
        assert!(orphan_reason(&[relationship()], &orders(), &[], &parents).is_none());
    }
}
