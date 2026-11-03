//! MSysQueries decoding (step 15.3, item M3).
//!
//! Native saved queries live in `MSysObjects` (Type 5, the query flavor in
//! `Flags & 0xF0`) and their definition in `MSysQueries`, one row per
//! attribute with the attribute code from `docs/FORMAT-JET.md`
//! ("MSysQueries"): 0 start, 1 type bits, 2 parameter, 3 flags, 4 remote
//! db, 5 table, 6 column, 7 join, 8 where, 9 group by, 10 having,
//! 11 order by, 255 end. Rows group by `ObjectId` and follow `Order`.
//!
//! Non-mappable attribute codes are reported as a precise `Unsupported`
//! (M3: unrepresentable constructs stay documented rests).

use crate::error::StorageError;
use crate::jet::reader::Jet4TableReader;
use crate::types::StorageValue;

/// Object type constant for queries in `MSysObjects`.
pub const OBJECT_TYPE_QUERY: i64 = 5;

/// The nine Access query flavors (object-flag bits `0xF0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeQueryKind {
    /// Select query (flag 0x00).
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

impl NativeQueryKind {
    /// Maps the object flags onto the query flavor.
    pub fn from_flags(flags: i64) -> Result<Self, StorageError> {
        Ok(match (flags & 0xF0) as u8 {
            0x00 => Self::Select,
            0x10 => Self::Crosstab,
            0x20 => Self::Delete,
            0x30 => Self::Update,
            0x40 => Self::Append,
            0x50 => Self::MakeTable,
            0x60 => Self::DataDefinition,
            0x70 => Self::Passthrough,
            0x80 => Self::Union,
            other => {
                return Err(StorageError::Unsupported {
                    reason: format!(
                        "query object flags {other:#04x} are not a known Access query flavor"
                    ),
                });
            }
        })
    }
}

/// One column of a native query (`Attribute` 6).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeQueryColumn {
    /// Source table, when the column names one.
    pub table: Option<String>,
    /// Source field or output alias.
    pub field: Option<String>,
    /// Column expression, when the column is computed.
    pub expression: Option<String>,
    /// Whether the column sorts descending (`OrderBy` row with flag 1).
    pub descending: bool,
}

/// Join flavor of a native query (`Attribute` 7 flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJoinKind {
    /// Inner join (flag 1).
    Inner,
    /// Left outer join (flag 2).
    Left,
    /// Right outer join (flag 3).
    Right,
}

/// One JOIN expression of a native query.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeJoin {
    /// Equality expression verbatim.
    pub expression: String,
    /// Outer/inner flavor.
    pub kind: NativeJoinKind,
}

/// One ORDER BY entry of a native query.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeOrder {
    /// Field reference verbatim.
    pub field: String,
    /// Whether the entry sorts descending.
    pub descending: bool,
}

/// One native saved query resolved from the catalog and MSysQueries.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeQuery {
    /// Query name from `MSysObjects`.
    pub name: String,
    /// Query flavor from the object flags.
    pub kind: NativeQueryKind,
    /// FROM tables in attribute order.
    pub tables: Vec<String>,
    /// Selected columns.
    pub columns: Vec<NativeQueryColumn>,
    /// WHERE expressions verbatim (attribute 8).
    pub criteria: Vec<String>,
    /// GROUP BY expressions verbatim (attribute 9).
    pub group_by: Vec<String>,
    /// HAVING expressions verbatim (attribute 10).
    pub having: Vec<String>,
    /// JOIN expressions with their flavors (attribute 7).
    pub joins: Vec<NativeJoin>,
    /// ORDER BY entries (attribute 11).
    pub order_by: Vec<NativeOrder>,
    /// Select-type bits of attribute 1.
    pub select_flags: i64,
    /// Parameter names (attribute 2).
    pub parameters: Vec<String>,
}

impl Jet4TableReader {
    /// Decodes every native saved query (M3). Queries without an
    /// MSysQueries entry stay listed with an empty definition.
    pub fn native_queries(&self) -> Result<Vec<NativeQuery>, StorageError> {
        let entries = self.catalog()?;
        let attribute_rows = match entries
            .iter()
            .find(|entry| entry.name == "MSysQueries" && entry.tdef_page.is_some())
        {
            Some(system) => self.read_table(system.tdef_page.expect("checked"))?.rows,
            None => Vec::new(),
        };
        let mut queries: Vec<NativeQuery> = Vec::new();
        for entry in entries
            .iter()
            .filter(|entry| entry.object_type == OBJECT_TYPE_QUERY)
        {
            let kind = NativeQueryKind::from_flags(entry.flags)?;
            let mut query = NativeQuery {
                name: entry.name.clone(),
                kind,
                tables: Vec::new(),
                columns: Vec::new(),
                criteria: Vec::new(),
                group_by: Vec::new(),
                having: Vec::new(),
                joins: Vec::new(),
                order_by: Vec::new(),
                select_flags: 0,
                parameters: Vec::new(),
            };
            for row in attribute_rows
                .iter()
                .filter(|row| object_id(row) == Some(entry.id))
                .collect::<Vec<_>>()
            {
                apply_attribute(row, &mut query)?;
            }
            queries.push(query);
        }
        Ok(queries)
    }
}

fn object_id(row: &[StorageValue]) -> Option<i64> {
    match row.first() {
        Some(StorageValue::Int(id)) => Some(*id),
        _ => None,
    }
}

fn attribute(row: &[StorageValue]) -> Option<i64> {
    match row.get(1) {
        Some(StorageValue::Int(code)) => Some(*code),
        _ => None,
    }
}

fn text(row: &[StorageValue], index: usize) -> Option<String> {
    match row.get(index) {
        Some(StorageValue::Text(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn flag(row: &[StorageValue]) -> i64 {
    match row.get(6) {
        Some(StorageValue::Int(value)) => *value,
        _ => 0,
    }
}

fn apply_attribute(row: &[StorageValue], query: &mut NativeQuery) -> Result<(), StorageError> {
    match attribute(row) {
        // Start/type bits/remote db rows carry no definition content.
        Some(0 | 3 | 4 | 255) => Ok(()),
        Some(1) => {
            query.select_flags = flag(row);
            Ok(())
        }
        Some(2) => {
            if let Some(name) = text(row, 3) {
                query.parameters.push(name);
            }
            Ok(())
        }
        Some(5) => {
            if let Some(table) = text(row, 3) {
                query.tables.push(table);
            }
            Ok(())
        }
        Some(6) => {
            query.columns.push(NativeQueryColumn {
                table: text(row, 3),
                field: text(row, 4),
                expression: text(row, 5),
                descending: flag(row) == 1,
            });
            Ok(())
        }
        Some(7) => {
            let expression =
                text(row, 5)
                    .or_else(|| text(row, 3))
                    .ok_or_else(|| StorageError::Unsupported {
                        reason: format!("query {:?}: join row without an expression", query.name),
                    })?;
            let kind = match flag(row) {
                1 => NativeJoinKind::Inner,
                2 => NativeJoinKind::Left,
                3 => NativeJoinKind::Right,
                other => {
                    return Err(StorageError::Unsupported {
                        reason: format!(
                            "query {:?}: join flag {other} is not a known join flavor (1 inner, \
                             2 left, 3 right)",
                            query.name
                        ),
                    });
                }
            };
            query.joins.push(NativeJoin { expression, kind });
            Ok(())
        }
        Some(8) => {
            if let Some(criterion) = text(row, 5) {
                query.criteria.push(criterion);
            }
            Ok(())
        }
        Some(9) => {
            if let Some(group) = text(row, 5) {
                query.group_by.push(group);
            }
            Ok(())
        }
        Some(10) => {
            if let Some(having) = text(row, 5) {
                query.having.push(having);
            }
            Ok(())
        }
        Some(11) => {
            let field =
                text(row, 5)
                    .or_else(|| text(row, 3))
                    .ok_or_else(|| StorageError::Unsupported {
                        reason: format!("query {:?}: ORDER BY row without a field", query.name),
                    })?;
            query.order_by.push(NativeOrder {
                field,
                descending: flag(row) == 1,
            });
            Ok(())
        }
        Some(code) => Err(StorageError::Unsupported {
            reason: format!(
                "query {:?}: MSysQueries attribute code {code} is not decoded (FORMAT-JET.md)",
                query.name
            ),
        }),
        None => Err(StorageError::Corrupt {
            context: "MSysQueries".into(),
            detail: "attribute row without an Attribute value".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests;
