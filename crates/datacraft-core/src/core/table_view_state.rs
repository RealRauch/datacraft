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
        if order == SortOrder::None {
            view.sort_field = None;
            view.sort_order = None;
        } else {
            view.sort_field = Some(column.name.clone());
            view.sort_order = Some(order);
        }
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
            }
            None => {
                view.filters.remove(&column.name);
            }
        }
        Vec::new()
    }

    /// Clears every column filter of one table (step 44.5, M54) — the
    /// status chip's one-click clear.
    pub(crate) fn clear_table_filters(&mut self, table: &str) -> Vec<Event> {
        let Some(db) = Core::mutable_table(self, table) else {
            return failed(FailureCode::NotFound, format!("unknown table {table:?}"));
        };
        db.1.filters.clear();
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
