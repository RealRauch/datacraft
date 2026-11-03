//! Presentation-cache maintenance (steps 6.2, 28.1, 34.5; items H23/U14/M29).
//!
//! Row and preview caches are presentation state: they are loaded lazily per
//! tab and invalidated by the events that change the underlying rows.

use super::App;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::port::RowRange;

impl App {
    /// Table whose rows a tab renders: its own name for table tabs, the
    /// bound source for form tabs; `None` for every other kind.
    pub(super) fn bound_table(
        &self,
        tab: &datacraft_core::snapshot::TabSnapshot,
    ) -> Option<String> {
        if tab.object_kind == "form" {
            return self
                .snapshot
                .database
                .as_ref()
                .and_then(|db| db.forms.iter().find(|form| form.name == tab.object_name))
                .map(|form| form.source.to_lowercase())
                .filter(|source| !source.is_empty());
        }
        (tab.object_kind == "table").then(|| tab.object_name.to_lowercase())
    }

    /// Loads rows for every open table tab and drops caches after
    /// database/table changes.
    pub(super) fn maintain_caches(&mut self, events: &[Event]) {
        // A closed tab's presentation caches are never addressable again.
        let closed: Vec<TabId> = events
            .iter()
            .filter_map(|event| match event {
                Event::TabClosed { tab_id } => Some(*tab_id),
                _ => None,
            })
            .collect();
        for tab_id in closed {
            self.rows.remove(&tab_id);
            self.results.remove(&tab_id);
            self.new_row.remove(&tab_id);
            self.column_widths.remove(&tab_id);
            self.previews.remove(&tab_id);
            self.record_cursor.remove(&tab_id);
            self.design_selection.remove(&tab_id);
            self.design_new_field.remove(&tab_id);
            self.design_rename.remove(&tab_id);
            self.form_selection.remove(&tab_id);
            self.form_record.remove(&tab_id);
            self.preview_page.remove(&tab_id);
            self.drags.retain(|(id, _), _| *id != tab_id);
        }
        if events
            .iter()
            .any(|e| matches!(e, Event::DatabaseClosed | Event::TableDropped { .. }))
        {
            self.rows.clear();
            self.previews.clear();
            self.new_row.clear();
            self.column_widths.clear();
        }
        if events.iter().any(|e| {
            matches!(
                e,
                Event::DatabaseClosed
                    | Event::QbeCellSet { .. }
                    | Event::QueryCreated { .. }
                    | Event::QueryDropped { .. }
            )
        }) {
            self.results.clear();
        }
        // Every row-mutating event invalidates the caches of the tabs it
        // affects: table tabs by name, form tabs by their bound source
        // (M29). The reload loop below repopulates them from the port.
        let mutated: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                Event::RowInserted { table }
                | Event::RowUpdated { table, .. }
                | Event::RowDeleted { table, .. }
                | Event::Undone { table }
                | Event::Redone { table }
                | Event::TableAltered { table, .. }
                | Event::WriteApplied { table, .. }
                | Event::WriteRolledBack { table, .. } => Some(table.to_lowercase()),
                _ => None,
            })
            .collect();
        if !mutated.is_empty() {
            let stale: Vec<TabId> = self
                .snapshot
                .tabs
                .iter()
                .filter(|tab| {
                    self.bound_table(tab)
                        .is_some_and(|table| mutated.contains(&table))
                })
                .map(|tab| tab.id)
                .collect();
            for tab_id in stale {
                self.rows.remove(&tab_id);
            }
        }
        let Some(db) = &self.snapshot.database else {
            self.rows.clear();
            return;
        };
        for tab in &self.snapshot.tabs {
            if self.rows.contains_key(&tab.id) {
                continue;
            }
            // Table tabs read their own rows; form tabs read their source.
            let source_name = if tab.object_kind == "form" {
                db.forms
                    .iter()
                    .find(|f| f.name == tab.object_name)
                    .map(|f| f.source.clone())
            } else {
                Some(tab.object_name.clone())
            };
            let Some(source_name) = source_name.filter(|n| !n.is_empty()) else {
                continue;
            };
            let Some(definition) = db
                .tables
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(&source_name))
            else {
                continue;
            };
            if let Ok(rows) = self.core.read_rows(
                &definition.name,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            ) {
                self.rows.insert(tab.id, rows);
            }
        }
        // Report previews refresh with every mutation pass — design edits
        // and row changes both flow through dispatch.
        for tab in &self.snapshot.tabs {
            if tab.object_kind == "report"
                && let Ok(pages) = self.core.report_preview(&tab.object_name)
            {
                self.previews.insert(tab.id, pages);
            }
        }
        // Query results refresh whenever the QBE model changed.
        let query_tabs: Vec<(TabId, String)> = self
            .snapshot
            .tabs
            .iter()
            .filter(|tab| tab.object_kind == "query")
            .map(|tab| (tab.id, tab.object_name.clone()))
            .collect();
        for (tab_id, name) in query_tabs {
            if self.results.contains_key(&tab_id) {
                continue;
            }
            if let Ok(result) = self.core.run_query(&name) {
                self.results.insert(tab_id, result);
            }
        }
    }
}
