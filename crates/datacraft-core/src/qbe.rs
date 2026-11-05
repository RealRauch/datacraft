//! QBE query model (step 7.2, item H26; decision default 7).
//!
//! No SQL parser: a query is a source table plus per-column show/sort/
//! criteria rows, exactly like the Access design grid. Identifiers are
//! whitelisted against the schema at dispatch (LESSONS #5); this module
//! carries the structure and structural validation.

use serde::{Deserialize, Serialize};

/// Sort state of one query column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Not sorted (default).
    #[default]
    None,
    /// Ascending.
    Ascending,
    /// Descending.
    Descending,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, ""),
            Self::Ascending => write!(f, "ascending"),
            Self::Descending => write!(f, "descending"),
        }
    }
}

impl SortOrder {
    /// Sort state after a datasheet header click (step 21.4, item U8):
    /// unsorted columns sort ascending, then toggle descending/ascending.
    pub const fn toggled(self) -> Self {
        match self {
            Self::None => Self::Ascending,
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// One column of the QBE grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QbeField {
    /// Source field name, resolved against the source table by name
    /// (case-insensitive) through the schema whitelist — never by
    /// grid position (step 42.3, item L22).
    pub name: String,
    /// Whether the column appears in the result.
    pub shown: bool,
    /// Sort order.
    pub sort: SortOrder,
    /// Criteria strings, OR-combined within the column (row order).
    pub criteria: Vec<String>,
}

/// A query definition: source table plus QBE columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDef {
    /// Query name (unique in its database, case-insensitive).
    pub name: String,
    /// Source table (whitelisted against the database).
    pub table: String,
    /// QBE columns in grid order. The list may be any subset in any
    /// order of the source's fields: shown names resolve by name and
    /// must be unique (case-insensitive) and known, or execution
    /// answers a precise `EngineError`.
    pub fields: Vec<QbeField>,
}

impl QueryDef {
    /// Creates an empty query over `table` (may be empty until chosen).
    pub fn new(name: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            table: table.into(),
            fields: Vec::new(),
        }
    }

    /// Structural cell edit (no whitelist — dispatch owns that).
    pub fn set_cell(&mut self, row: QbeRow, col: usize, value: QbeValue) -> Result<(), QueryError> {
        match row {
            QbeRow::Table => match value {
                QbeValue::Text(table) => {
                    self.table = table;
                    Ok(())
                }
                QbeValue::Bool(_) => Err(QueryError::InvalidCell {
                    detail: "the Table row takes a name".to_owned(),
                }),
            },
            QbeRow::Field => match value {
                QbeValue::Text(name) => {
                    if col == self.fields.len() {
                        if self
                            .fields
                            .iter()
                            .any(|f| f.name.eq_ignore_ascii_case(&name))
                        {
                            return Err(QueryError::DuplicateField { field: name });
                        }
                        self.fields.push(QbeField {
                            name,
                            shown: true,
                            sort: SortOrder::None,
                            criteria: Vec::new(),
                        });
                        return Ok(());
                    }
                    if self
                        .fields
                        .iter()
                        .enumerate()
                        .any(|(i, f)| i != col && f.name.eq_ignore_ascii_case(&name))
                    {
                        return Err(QueryError::DuplicateField { field: name });
                    }
                    let field = self.fields.get_mut(col).ok_or(QueryError::InvalidCell {
                        detail: "columns extend one at a time".to_owned(),
                    })?;
                    field.name = name;
                    Ok(())
                }
                QbeValue::Bool(_) => Err(QueryError::InvalidCell {
                    detail: "the Field row takes a name".to_owned(),
                }),
            },
            QbeRow::Show => {
                let field = self.fields.get_mut(col).ok_or(QueryError::InvalidCell {
                    detail: "no column at that position".to_owned(),
                })?;
                match value {
                    QbeValue::Bool(shown) => {
                        field.shown = shown;
                        Ok(())
                    }
                    QbeValue::Text(_) => Err(QueryError::InvalidCell {
                        detail: "the Show row takes a checkbox".to_owned(),
                    }),
                }
            }
            QbeRow::Sort => {
                let field = self.fields.get_mut(col).ok_or(QueryError::InvalidCell {
                    detail: "no column at that position".to_owned(),
                })?;
                match value {
                    QbeValue::Text(text) => {
                        field.sort = match text.as_str() {
                            "" => SortOrder::None,
                            "ascending" => SortOrder::Ascending,
                            "descending" => SortOrder::Descending,
                            other => {
                                return Err(QueryError::InvalidCell {
                                    detail: format!(
                                        "sort must be ascending, descending or empty, found {other:?}"
                                    ),
                                });
                            }
                        };
                        Ok(())
                    }
                    QbeValue::Bool(_) => Err(QueryError::InvalidCell {
                        detail: "the Sort row takes text".to_owned(),
                    }),
                }
            }
            QbeRow::Criteria(or_row) => {
                let field = self.fields.get_mut(col).ok_or(QueryError::InvalidCell {
                    detail: "no column at that position".to_owned(),
                })?;
                let QbeValue::Text(text) = value else {
                    return Err(QueryError::InvalidCell {
                        detail: "the Criteria row takes text".to_owned(),
                    });
                };
                if text.is_empty() {
                    return Err(QueryError::InvalidCell {
                        detail: "criteria must not be empty".to_owned(),
                    });
                }
                if or_row == field.criteria.len() {
                    field.criteria.push(text);
                } else if let Some(slot) = field.criteria.get_mut(or_row) {
                    *slot = text;
                } else {
                    return Err(QueryError::InvalidCell {
                        detail: "criteria rows fill in order".to_owned(),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Grid row kinds (the Access QBE layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QbeRow {
    /// Source table row.
    Table,
    /// Field name row.
    Field,
    /// Show-checkbox row.
    Show,
    /// Sort row.
    Sort,
    /// N-th criteria ("or") row.
    Criteria(usize),
}

/// Cell values of the QBE grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QbeValue {
    /// Text cell.
    Text(String),
    /// Checkbox cell.
    Bool(bool),
}

/// Structural errors of QBE edits (whitelist violations come from
/// dispatch as `Event::Failed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// A column references a field twice.
    DuplicateField {
        /// The duplicated name.
        field: String,
    },
    /// The edit does not fit the grid's structure.
    InvalidCell {
        /// Concrete reason.
        detail: String,
    },
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateField { field } => {
                write!(
                    f,
                    "duplicate query column {field:?}: columns are case-insensitive"
                )
            }
            Self::InvalidCell { detail } => write!(f, "invalid cell: {detail}"),
        }
    }
}

impl std::error::Error for QueryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str) -> QbeField {
        QbeField {
            name: name.to_owned(),
            shown: true,
            sort: SortOrder::None,
            criteria: Vec::new(),
        }
    }

    /// Every row kind rejects the wrong value type.
    #[test]
    fn cells_reject_the_wrong_value_type() {
        let mut def = QueryDef::new("q", "t");
        def.fields.push(field("a"));
        let text = QbeValue::Text("x".into());
        let bool_ = QbeValue::Bool(true);

        assert!(def.set_cell(QbeRow::Table, 0, bool_.clone()).is_err());
        assert!(def.set_cell(QbeRow::Field, 0, bool_.clone()).is_err());
        assert!(def.set_cell(QbeRow::Show, 0, text).is_err());
        assert!(def.set_cell(QbeRow::Sort, 0, bool_.clone()).is_err());
        assert!(def.set_cell(QbeRow::Criteria(0), 0, bool_).is_err());
    }

    /// Cells beyond the grid are precise errors (columns extend one at a
    /// time via the Field row; other rows need an existing column).
    #[test]
    fn cells_beyond_the_grid_are_errors() {
        let mut def = QueryDef::new("q", "t");
        assert!(
            def.set_cell(QbeRow::Field, 1, QbeValue::Text("a".into()))
                .is_err()
        );
        assert!(def.set_cell(QbeRow::Show, 0, QbeValue::Bool(true)).is_err());
        assert!(
            def.set_cell(QbeRow::Sort, 0, QbeValue::Text("".into()))
                .is_err()
        );
        assert!(
            def.set_cell(QbeRow::Criteria(0), 0, QbeValue::Text("x".into()))
                .is_err()
        );
    }

    /// Sort accepts exactly the three documented texts.
    #[test]
    fn sort_texts_parse_or_reject() {
        let mut def = QueryDef::new("q", "t");
        def.fields.push(field("a"));
        for (text, expected) in [
            ("ascending", SortOrder::Ascending),
            ("descending", SortOrder::Descending),
            ("", SortOrder::None),
        ] {
            def.set_cell(QbeRow::Sort, 0, QbeValue::Text(text.into()))
                .expect("valid sort text");
            assert_eq!(def.fields[0].sort, expected);
        }
        assert!(
            def.set_cell(QbeRow::Sort, 0, QbeValue::Text("x".into()))
                .is_err()
        );
        assert_eq!(def.fields[0].sort.to_string(), "", "none renders empty");
    }

    /// Criteria rows fill in order; re-editing overwrites in place.
    #[test]
    fn criteria_rows_fill_in_order_and_overwrite() {
        let mut def = QueryDef::new("q", "t");
        def.fields.push(field("a"));
        def.set_cell(QbeRow::Criteria(0), 0, QbeValue::Text("1".into()))
            .unwrap();
        def.set_cell(QbeRow::Criteria(1), 0, QbeValue::Text("2".into()))
            .unwrap();
        assert!(
            def.set_cell(QbeRow::Criteria(3), 0, QbeValue::Text("4".into()))
                .is_err()
        );
        assert_eq!(def.fields[0].criteria, ["1", "2"]);
        assert!(
            def.set_cell(QbeRow::Criteria(0), 0, QbeValue::Text("9".into()))
                .is_ok()
        );
        assert_eq!(def.fields[0].criteria, ["9", "2"]);
    }

    /// Error displays name the rule and the offending value.
    #[test]
    fn query_error_displays_are_precise() {
        let dup = QueryError::DuplicateField { field: "a".into() };
        assert!(dup.to_string().contains("a"));
        assert!(dup.to_string().contains("duplicate"));
        let invalid = QueryError::InvalidCell {
            detail: "no column".into(),
        };
        assert!(invalid.to_string().contains("no column"));
    }

    /// A header click cycles None -> ascending -> descending -> ascending
    /// (step 21.4, U8): the first click sorts, later clicks toggle.
    #[test]
    fn sort_toggle_cycles_ascending_and_descending() {
        assert_eq!(SortOrder::None.toggled(), SortOrder::Ascending);
        assert_eq!(SortOrder::Ascending.toggled(), SortOrder::Descending);
        assert_eq!(SortOrder::Descending.toggled(), SortOrder::Ascending);
    }

    /// Display covers every sort state; wrong cell types stay precise.
    #[test]
    fn sort_display_and_wrong_cell_types_are_precise() {
        assert_eq!(SortOrder::None.to_string(), "");
        assert_eq!(SortOrder::Ascending.to_string(), "ascending");
        assert_eq!(SortOrder::Descending.to_string(), "descending");
        let mut def = QueryDef::new("q", "t");
        def.fields.push(field("a"));
        assert!(
            def.set_cell(QbeRow::Show, 0, QbeValue::Text("x".into()))
                .is_err()
        );
        assert!(def.set_cell(QbeRow::Sort, 0, QbeValue::Bool(true)).is_err());
    }

    /// The Field row names one source field: renaming a column to
    /// another column's name (case-insensitive) and out-of-range columns
    /// fail precisely; renaming itself is always allowed.
    #[test]
    fn field_row_rejects_duplicate_names_and_out_of_range_columns() {
        let mut def = QueryDef::new("q", "t");
        def.fields.push(field("a"));
        def.fields.push(field("b"));
        assert_eq!(
            def.set_cell(QbeRow::Field, 0, QbeValue::Text("B".into())),
            Err(QueryError::DuplicateField { field: "B".into() })
        );
        assert_eq!(
            def.set_cell(QbeRow::Field, 5, QbeValue::Text("c".into())),
            Err(QueryError::InvalidCell {
                detail: "columns extend one at a time".to_owned(),
            })
        );
        assert_eq!(
            def.set_cell(QbeRow::Field, 0, QbeValue::Text("A".into())),
            Ok(()),
            "renaming a column to its own name (case-insensitive) is not a duplicate"
        );
        assert_eq!(def.fields[0].name, "A");
        assert_eq!(def.fields[1].name, "b");
    }
}
