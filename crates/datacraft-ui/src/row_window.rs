//! The datasheet's bounded row window (phase 101, step 101.5; item M132).
//!
//! A large table is not read up front: the port's row count sizes the
//! virtual list, and only the windows around the visible rows are loaded
//! (each `WINDOW_ROWS` long). Filters, sorts, find/replace and staged
//! edits need the whole table, so they upgrade the tab through
//! `App::load_all_rows` — a documented boundary, not a silent fallback.

use std::collections::{BTreeMap, HashMap};

use datacraft_core::ids::TabId;
use datacraft_core::schema::FieldValue;

use crate::app::App;

/// How many rows one window holds.
pub const WINDOW_ROWS: usize = 200;

/// Above this many rows the datasheet windows instead of loading all.
pub const WINDOW_THRESHOLD: usize = 1000;

/// The loaded windows of one table tab.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RowWindow {
    /// Rows the table holds (the port's count).
    pub total: usize,
    /// Window index → its rows.
    pub loaded: BTreeMap<usize, Vec<Vec<FieldValue>>>,
}

impl RowWindow {
    /// The window index a row belongs to.
    pub fn window_of(row: usize) -> usize {
        row / WINDOW_ROWS
    }

    /// One row's values, when its window is loaded.
    pub fn row(&self, row: usize) -> Option<&Vec<FieldValue>> {
        let index = Self::window_of(row);
        self.loaded
            .get(&index)
            .and_then(|rows| rows.get(row - index * WINDOW_ROWS))
    }

    /// Whether the window carrying `row` is loaded.
    pub fn has_row(&self, row: usize) -> bool {
        self.row(row).is_some()
    }
}

impl App {
    /// The tab's window cache, when it runs windowed.
    pub fn row_window(&self, tab_id: TabId) -> Option<&RowWindow> {
        self.row_windows.get(&tab_id)
    }

    /// How many windows the tab currently holds (test seam and honest
    /// diagnostics).
    pub fn loaded_row_windows(&self, tab_id: TabId) -> Option<usize> {
        self.row_windows
            .get(&tab_id)
            .map(|window| window.loaded.len())
    }

    /// Starts windowed mode for a table above the threshold: the row count
    /// sizes the list, the first window carries the top rows.
    pub(crate) fn start_row_windowing(&mut self, tab_id: TabId, table: &str) -> bool {
        let Ok(total) = self.core.row_count(table) else {
            return false;
        };
        if total <= WINDOW_THRESHOLD {
            return false;
        }
        let mut window = RowWindow {
            total,
            ..RowWindow::default()
        };
        match self.core.read_row_window(table, 0, WINDOW_ROWS) {
            Ok(rows) => {
                window.loaded.insert(0, rows);
            }
            Err(_) => return false,
        }
        self.row_windows.insert(tab_id, window);
        self.rows.remove(&tab_id);
        true
    }

    /// Loads the windows covering `first..=last` (and the one after), so a
    /// scroll paints without a second frame of placeholders.
    pub(crate) fn ensure_row_windows(
        &mut self,
        tab_id: TabId,
        table: &str,
        first: usize,
        last: usize,
    ) {
        let Some(window) = self.row_windows.get(&tab_id) else {
            return;
        };
        let total = window.total;
        if total == 0 {
            return;
        }
        let last = last.min(total - 1);
        let first_window = RowWindow::window_of(first.min(last));
        let last_window = RowWindow::window_of(last);
        for index in first_window..=last_window.min(total / WINDOW_ROWS) {
            if self
                .row_windows
                .get(&tab_id)
                .is_some_and(|window| window.loaded.contains_key(&index))
            {
                continue;
            }
            let start = index * WINDOW_ROWS;
            let rows = match self.core.read_row_window(table, start, WINDOW_ROWS) {
                Ok(rows) => rows,
                Err(_) => return,
            };
            if let Some(window) = self.row_windows.get_mut(&tab_id) {
                window.loaded.insert(index, rows);
            }
        }
    }

    /// Drops the window cache and reads the whole table — the documented
    /// upgrade for filters, sorts, find/replace and staged edits.
    pub(crate) fn load_all_rows(&mut self, tab_id: TabId, table: &str) {
        if self.row_windows.remove(&tab_id).is_none() {
            return;
        }
        if let Ok(rows) = self.core.read_row_window(table, 0, usize::MAX) {
            self.rows.insert(tab_id, rows);
        }
    }
}

/// The `HashMap` type the cache uses (exported for the state field).
pub(crate) type RowWindows = HashMap<TabId, RowWindow>;
