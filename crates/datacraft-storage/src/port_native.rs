//! Native query mapping (step 15.3, item M3): the storage-local
//! `MSysQueries` decoder results onto the core port vocabulary.

use crate::jet::NativeQueryKind as Jet;
use datacraft_core::port::{NativeQuery, NativeQueryColumn, NativeQueryKind as Core};

/// Maps decoded native queries onto the core's read-only vocabulary.
pub(crate) fn to_core(queries: Vec<crate::jet::NativeQuery>) -> Vec<NativeQuery> {
    queries
        .into_iter()
        .map(|query| NativeQuery {
            name: query.name,
            kind: match query.kind {
                Jet::Select => Core::Select,
                Jet::Crosstab => Core::Crosstab,
                Jet::Delete => Core::Delete,
                Jet::Update => Core::Update,
                Jet::Append => Core::Append,
                Jet::MakeTable => Core::MakeTable,
                Jet::DataDefinition => Core::DataDefinition,
                Jet::Passthrough => Core::Passthrough,
                Jet::Union => Core::Union,
            },
            tables: query.tables,
            columns: query
                .columns
                .into_iter()
                .map(|column| NativeQueryColumn {
                    table: column.table,
                    field: column.field,
                    expression: column.expression,
                    descending: column.descending,
                })
                .collect(),
            criteria: query.criteria,
            joins: query.joins,
            order_by: query.order_by,
        })
        .collect()
}
