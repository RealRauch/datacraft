//! Table datasheet view-state commands (step 44.5, item M54) — split
//! from `database.rs` for the file-size gate.
//!
//! Sort and column filters are presentation state on the open database
//! (`DatabaseState::table_views`); the row data itself is never
//! touched. Criteria parse eagerly against the column's kind so
//! garbage fails at set time instead of silently filtering nothing.

use crate::core::Core;
use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::qbe::SortOrder;
use crate::state::TableViewState;

impl Core {
    /// Stores or clears one table's datasheet sort (step 44.5, M54).
    pub(crate) fn set_table_sort(
        &mut self,
        table: &str,
        field: &str,
        order: SortOrder,
    ) -> Vec<Event> {
        let Some(db) = Core::mutable_table(self, table) else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        // The clear accepts any field name (step 91.5, item M113): a
        // rename can leave the sort field stale, and the datasheet's
        // self-healing clear never fails on that name.
        if order == SortOrder::None {
            let view = db.1;
            view.sort_field = None;
            view.sort_order = None;
            return Vec::new();
        }
        let Some(column) =
            db.0.iter()
                .find(|f| f.name.eq_ignore_ascii_case(field))
                .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                format!("field {field:?} is not a field of {table:?}"),
            );
        };
        let view = db.1;
        view.sort_field = Some(column.name.clone());
        view.sort_order = Some(order);
        Vec::new()
    }

    /// Stores or clears one table's column filter (step 44.5, M54);
    /// criteria parse eagerly against the column's kind.
    pub(crate) fn set_table_filter(
        &mut self,
        table: &str,
        field: &str,
        criteria: Option<&str>,
    ) -> Vec<Event> {
        let Some(db) = Core::mutable_table(self, table) else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        let Some(column) =
            db.0.iter()
                .find(|f| f.name.eq_ignore_ascii_case(field))
                .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                format!("field {field:?} is not a field of {table:?}"),
            );
        };
        let view = db.1;
        match criteria {
            Some(text) => {
                // Fail loudly at set time (M54): garbage criteria must
                // never silently filter nothing at render time.
                if let Err(error) = crate::query_criteria::parse_predicate(text, field, &column) {
                    return failed(FailureCode::Port, error.to_string());
                }
                view.filters.insert(column.name.clone(), text.to_owned());
                // One filter per column: the typed criteria replaces a
                // picked value (phase 101, step 101.1; M132).
                view.value_filters.remove(&column.name);
            }
            None => {
                view.filters.remove(&column.name);
                view.value_filters.remove(&column.name);
            }
        }
        Vec::new()
    }

    /// Lists one column's distinct values for the filter-by-value dialog
    /// (phase 101, step 101.1; item M132): read-only, bounded and ordered
    /// by the datasheet's total comparator. The rows come through the
    /// port, so every client lists the same values.
    pub(crate) fn list_column_values(&mut self, table: &str, field: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(FailureCode::NoDatabase, "no database is open");
        };
        let Some(definition) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .cloned()
        else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        let rows = match self.port.read_rows(
            &definition.name,
            crate::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(error) => return failed(FailureCode::Port, error.to_string()),
        };
        match crate::table_view::distinct_values(&definition, &rows, field) {
            Ok(listed) => {
                let canonical = definition
                    .fields
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(field))
                    .map(|candidate| candidate.name.clone())
                    .unwrap_or_else(|| field.to_owned());
                vec![Event::ColumnValuesListed {
                    table: definition.name,
                    field: canonical,
                    values: listed.values,
                    blank: listed.blank,
                    truncated: listed.truncated,
                }]
            }
            Err(error) => failed(FailureCode::NotFound, error.to_string()),
        }
    }

    /// Stores one column's value filter (phase 101, step 101.1; item
    /// M132): `None` picks the blank entry. The value must appear in the
    /// column's current list — a stale pick fails precisely instead of
    /// filtering nothing, and a blank pick needs a `Null` cell to exist.
    pub(crate) fn filter_by_value(
        &mut self,
        table: &str,
        field: &str,
        value: Option<&str>,
    ) -> Vec<Event> {
        // The catalog and the current list come first: a rejected pick
        // must leave the view exactly as it was (immutable reads only).
        let Some(definition) = self
            .state
            .database
            .as_ref()
            .and_then(|db| {
                db.tables
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            })
            .cloned()
        else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        let Some(column) = definition
            .fields
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(field))
            .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                format!("field {field:?} is not a field of {table:?}"),
            );
        };
        let rows = match self.port.read_rows(
            &definition.name,
            crate::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(error) => return failed(FailureCode::Port, error.to_string()),
        };
        let listed = match crate::table_view::distinct_values(&definition, &rows, &column.name) {
            Ok(listed) => listed,
            Err(error) => return failed(FailureCode::NotFound, error.to_string()),
        };
        let filter = match value {
            Some(text) => {
                if !listed.values.iter().any(|candidate| candidate == text) {
                    return failed(
                        FailureCode::NotFound,
                        format!(
                            "no value {text:?} in column {:?} of {table:?} — the list changed",
                            column.name
                        ),
                    );
                }
                crate::state::ValueFilter::Text(text.to_owned())
            }
            None => {
                if !listed.blank {
                    return failed(
                        FailureCode::NotFound,
                        format!(
                            "column {:?} of {table:?} has no blank cells",
                            column.name
                        ),
                    );
                }
                crate::state::ValueFilter::Blank
            }
        };
        let Some(db) = Core::mutable_table(self, table) else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        let view = db.1;
        // One filter per column: the picked value replaces a criteria.
        view.filters.remove(&column.name);
        view.value_filters.insert(column.name, filter);
        Vec::new()
    }

    /// Clears every column filter of one table (step 44.5, M54) — the
    /// status chip's one-click clear. Value filters clear too (phase 101,
    /// step 101.1; M132).
    pub(crate) fn clear_table_filters(&mut self, table: &str) -> Vec<Event> {
        let Some(db) = Core::mutable_table(self, table) else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        db.1.filters.clear();
        db.1.value_filters.clear();
        Vec::new()
    }

    /// Resolves a table mutably: `Some((fields, &mut view entry))` with
    /// the catalog's canonical field list and the table's view state
    /// (created on demand). `None` without an open database or for an
    /// unknown table.
    fn mutable_table<'core>(
        core: &'core mut Core,
        table: &str,
    ) -> Option<(&'core [crate::schema::FieldDef], &'core mut TableViewState)> {
        let db = core.state.database.as_mut()?;
        let canonical = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))?
            .name
            .clone();
        let fields = db
            .tables
            .iter()
            .find(|candidate| candidate.name == canonical)
            .map(|candidate| candidate.fields.as_slice())?;
        let entry = db.table_views.entry(canonical).or_default();
        Some((fields, entry))
    }
}
