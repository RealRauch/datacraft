//! The datasheet's column order (phase 101, step 101.4; item M132).
//!
//! Display order is presentation state per table tab: a permutation of the
//! table's storage columns. The rows, widths, sort indicators and follow
//! labels are permuted at the datasheet boundary, so the grid only ever
//! sees display positions; the edit paths map back through
//! [`App::storage_column`].

use datacraft_core::ids::TabId;

use crate::app::App;

impl App {
    /// The tab's display order as display → storage indices, or the
    /// identity when no order is stored or it no longer fits the table
    /// (a schema change never panics).
    pub fn column_order(&self, tab_id: TabId, len: usize) -> Vec<usize> {
        let identity = (0..len).collect::<Vec<usize>>();
        let Some(stored) = self.column_order.get(&tab_id) else {
            return identity;
        };
        if stored.len() != len || stored.iter().any(|index| *index >= len) {
            return identity;
        }
        stored.clone()
    }

    /// The storage column behind one display position (the edit paths'
    /// mapping; falls back to the identity).
    pub fn storage_column(&self, tab_id: TabId, display: usize) -> usize {
        self.column_order
            .get(&tab_id)
            .and_then(|order| order.get(display).copied())
            .unwrap_or(display)
    }

    /// Moves one column to a new display position (the header menu's
    /// accessible twin of the drag gesture).
    pub fn move_column(&mut self, tab_id: TabId, len: usize, from: usize, to: usize) {
        let current = self.column_order(tab_id, len);
        let moved = datacraft_core::table_view::move_column(&current, from, to);
        if moved == current {
            return;
        }
        self.column_order.insert(tab_id, moved);
    }
}
