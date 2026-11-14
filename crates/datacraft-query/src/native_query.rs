//! The native saved-query vocabulary (M3) — split from the port root
//! (step 36.7) to keep every file under the 500-line size gate, then
//! moved with the query layer (step 76.3, item L37). The decodable
//! MSysQueries subset and its QBE projection live here; the command
//! layer's port root re-exports the vocabulary and owns the trait.

use serde::{Deserialize, Serialize};

/// Query flavor of a native saved query (step 15.3, M3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeQueryKind {
    /// Select query (object flags 0x00).
    Select,
    /// Crosstab query (0x10).
    Crosstab,
    /// Delete query (0x20).
    Delete,
    /// Update query (0x30).
    Update,
    /// Append query (0x40).
    Append,
    /// Make-table query (0x50).
    MakeTable,
    /// Data-definition query (0x60).
    DataDefinition,
    /// Pass-through query (0x70).
    Passthrough,
    /// Union query (0x80).
    Union,
}

/// One column of a native saved query (M3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeQueryColumn {
    /// Source table, when the column names one.
    pub table: Option<String>,
    /// Source field or output alias.
    pub field: Option<String>,
    /// Column expression, when the column is computed.
    pub expression: Option<String>,
    /// Whether the column sorts descending.
    pub descending: bool,
}

/// Join flavor of a native query (MSysQueries attribute 7 flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeJoinKind {
    /// Inner join (flag 1).
    Inner,
    /// Left outer join (flag 2): the left operand's rows are preserved.
    Left,
    /// Right outer join (flag 3): the right operand's rows are preserved.
    Right,
}

/// One JOIN expression of a native query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeJoin {
    /// Equality expression verbatim (e.g. `Table1.col1 = Table3.col1`).
    pub expression: String,
    /// Outer/inner flavor.
    pub kind: NativeJoinKind,
}

/// One ORDER BY entry of a native query (MSysQueries attribute 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeOrder {
    /// Field reference verbatim (qualified or bare).
    pub field: String,
    /// Whether the entry sorts descending.
    pub descending: bool,
}

/// One native saved query as stored in the Jet/ACE file (M3, read-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeQuery {
    /// Query name from MSysObjects.
    pub name: String,
    /// Query flavor from the object flags.
    pub kind: NativeQueryKind,
    /// FROM tables in attribute order.
    pub tables: Vec<String>,
    /// Selected columns.
    pub columns: Vec<NativeQueryColumn>,
    /// WHERE expressions verbatim (attribute 8).
    pub criteria: Vec<String>,
    /// GROUP BY expressions verbatim (attribute 9) — a documented rest
    /// for execution (step 27.1, M12).
    #[serde(default)]
    pub group_by: Vec<String>,
    /// HAVING expressions verbatim (attribute 10) — a documented rest.
    #[serde(default)]
    pub having: Vec<String>,
    /// JOIN expressions with their flavors (attribute 7).
    pub joins: Vec<NativeJoin>,
    /// ORDER BY entries (attribute 11).
    pub order_by: Vec<NativeOrder>,
    /// Select-type bits of attribute 1 (`0x01` star, `0x02` distinct,
    /// `0x08` distinct row, `0x10` top, `0x20` percent).
    #[serde(default)]
    pub select_flags: i64,
    /// Parameter names (attribute 2) — parameters stay a documented rest.
    #[serde(default)]
    pub parameters: Vec<String>,
}

impl NativeQuery {
    /// Maps the decodable subset onto a QBE query definition: select
    /// queries over exactly one table, plain field columns, no criteria,
    /// grouping, joins or computed columns. This is the read-only design
    /// grid's projection only; execution goes through
    /// [`crate::native_exec`] (step 27.1, M12).
    pub fn query_def(&self) -> Option<crate::qbe::QueryDef> {
        if self.kind != NativeQueryKind::Select
            || self.tables.len() != 1
            || !self.criteria.is_empty()
            || !self.group_by.is_empty()
            || !self.having.is_empty()
            || !self.joins.is_empty()
        {
            return None;
        }
        let mut fields = Vec::with_capacity(self.columns.len());
        for column in &self.columns {
            if column.expression.is_some() {
                return None;
            }
            let field = column.field.as_ref()?;
            fields.push(crate::qbe::QbeField {
                name: field.clone(),
                shown: true,
                sort: if column.descending {
                    crate::qbe::SortOrder::Descending
                } else {
                    crate::qbe::SortOrder::Ascending
                },
                criteria: Vec::new(),
            });
        }
        Some(crate::qbe::QueryDef {
            name: self.name.clone(),
            table: self.tables[0].clone(),
            fields,
        })
    }
}
