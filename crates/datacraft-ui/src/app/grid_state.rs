//! Datasheet presentation state (steps 11.7 and 21.4).
//!
//! Row/result caches, the new-record buffer and the per-session column
//! widths belong to `App` but stay out of the shell plumbing: this module
//! only exposes the getters/setters the datasheet, the SQL window and the
//! report preview read.

use datacraft_core::ids::TabId;
use datacraft_core::query_engine::QueryResult;
use datacraft_core::schema::FieldValue;

use super::App;

impl App {
    /// Row data of one table tab (cached; `None` until loaded).
    pub fn rows_for(&self, tab_id: TabId) -> Option<&Vec<Vec<FieldValue>>> {
        self.rows.get(&tab_id)
    }

    /// Query result of one query tab (cached; `None` until run).
    pub fn results_for(&self, tab_id: TabId) -> Option<&QueryResult> {
        self.results.get(&tab_id)
    }

    /// The localized load failure of one tab's presentation cache, if the
    /// last load attempt failed (step 39.4, item M31 — the reason renders
    /// in place of the endless loading state and clears on the next
    /// successful pass).
    pub(crate) fn load_error(&self, tab_id: TabId) -> Option<&str> {
        self.load_errors.get(&tab_id).map(String::as_str)
    }

    /// Buffered cells of a datasheet's new-record row, padded to the
    /// table's column count (step 11.7).
    pub fn new_row_cells(&self, tab_id: TabId, columns: usize) -> Vec<String> {
        let mut cells = self.new_row.get(&tab_id).cloned().unwrap_or_default();
        cells.resize(columns, String::new());
        cells
    }

    /// Stores the edited new-record cells of a datasheet tab.
    pub fn set_new_row_cells(&mut self, tab_id: TabId, cells: Vec<String>) {
        self.new_row.insert(tab_id, cells);
    }

    /// Clears the buffered new-record cells of a datasheet tab (after a
    /// successful insert).
    pub fn clear_new_row(&mut self, tab_id: TabId) {
        self.new_row.remove(&tab_id);
    }

    /// Effective column widths of a datasheet tab: the session's resized
    /// widths padded/truncated to the current column count (step 21.4).
    pub fn column_widths(&self, tab_id: TabId, columns: usize) -> Vec<f32> {
        let stored = self
            .column_widths
            .get(&tab_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        datacraft_core::grid::effective_widths(
            stored,
            columns,
            datacraft_core::grid::DEFAULT_COLUMN_WIDTH,
        )
    }

    /// Stores the session widths of a datasheet tab after a resize.
    pub fn set_column_widths(&mut self, tab_id: TabId, widths: Vec<f32>) {
        self.column_widths.insert(tab_id, widths);
    }

    /// Effective column widths of the SQL window's result grid.
    pub fn sql_column_widths(&self, columns: usize) -> Vec<f32> {
        datacraft_core::grid::effective_widths(
            &self.sql_column_widths,
            columns,
            datacraft_core::grid::DEFAULT_COLUMN_WIDTH,
        )
    }

    /// Stores the SQL window's result grid widths after a resize.
    pub fn set_sql_column_widths(&mut self, widths: Vec<f32>) {
        self.sql_column_widths = widths;
    }

    /// Buffered new-field name of a design tab (step 24.3, M9).
    pub fn design_new_field(&self, tab_id: TabId) -> String {
        self.design_new_field
            .get(&tab_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Stores the buffered new-field name of a design tab.
    pub fn set_design_new_field(&mut self, tab_id: TabId, name: String) {
        self.design_new_field.insert(tab_id, name);
    }

    /// Buffered rename text of a design tab (step 24.3, M9).
    pub fn design_rename(&self, tab_id: TabId) -> String {
        self.design_rename.get(&tab_id).cloned().unwrap_or_default()
    }

    /// Stores the buffered rename text of a design tab.
    pub fn set_design_rename(&mut self, tab_id: TabId, name: String) {
        self.design_rename.insert(tab_id, name);
    }
}
