//! Foreign-key navigation state (step 87.2, item M103): the per-tab
//! back/forward history over follow navigation and the referenced-by
//! counts. Both are presentation state on `App` — every state change
//! dispatches the existing bus commands (open, filter, activate);
//! nothing rides the wire.
//!
//! The counts run as separate bounded reads, at most one per frame: the
//! window cap ([`COUNT_CAP`], read as cap + 1) is the read's time box,
//! duplicate keys are answered from the map, and closing the tab or the
//! database drops every answer — a cancelled request never renders.

use std::collections::{BTreeMap, HashMap};

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::RowRange;
use datacraft_core::schema::{FieldDef, TableDef};
use datacraft_core::snapshot::TabSnapshot;

use super::App;

/// How many referencing rows a count reads before it names the
/// truncation (step 87.2, item M103 — the window cap is the read's
/// time box).
pub const COUNT_CAP: usize = 4096;

/// How many navigation steps a tab's history keeps per direction.
const HISTORY_CAP: usize = 32;

/// A stored navigation position: the table and the column filters the
/// tab carried. Sort stays live — the entry restores table and filter
/// only (step 87.2, item M103).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavEntry {
    /// The table the position shows.
    pub table: String,
    /// The column filters the tab carried.
    pub filters: BTreeMap<String, String>,
}

/// The count of referencing rows for one key: exact up to the cap,
/// truncated at the cap when the child carries more rows, or failed
/// (the read answered an error — the menu renders without a count).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencedCount {
    Exact(usize),
    Truncated(usize),
    Failed,
}

/// Per-tab back/forward stacks over [`NavEntry`] (step 87.2, M103).
#[derive(Debug, Default)]
pub struct NavHistory {
    back: Vec<NavEntry>,
    forward: Vec<NavEntry>,
}

/// One bounded read waiting for its frame slot.
#[derive(Debug, Clone)]
pub struct CountRequest {
    /// Map key (`child|column=criteria…`).
    pub key: String,
    /// The referencing (child) table to read.
    pub child: String,
    /// The criteria the child rows must satisfy.
    pub criteria: BTreeMap<String, String>,
}

/// The map key of one count (deterministic over criteria order).
pub(crate) fn count_key(child: &str, criteria: &[(String, String)]) -> String {
    let mut parts: Vec<String> = criteria
        .iter()
        .map(|(column, text)| format!("{column}={text}"))
        .collect();
    parts.sort();
    format!("{child}|{}", parts.join("|"))
}

impl App {
    /// The answered count for one child table and criteria set (step
    /// 87.2, item M103); `None` — the read has not answered (or was
    /// cleared with the tab).
    pub fn referenced_by_count(
        &self,
        child: &str,
        criteria: &[(String, String)],
    ) -> Option<ReferencedCount> {
        self.count_results.get(&count_key(child, criteria)).cloned()
    }

    /// Queue one bounded read for a referencing count; duplicate keys
    /// are ignored (already answered or already queued).
    pub(crate) fn queue_referenced_count(&mut self, child: &str, criteria: &[(String, String)]) {
        let key = count_key(child, criteria);
        if self.count_results.contains_key(&key)
            || self.count_pending.iter().any(|request| request.key == key)
        {
            return;
        }
        self.count_pending.push(CountRequest {
            key,
            child: child.to_owned(),
            criteria: criteria.iter().cloned().collect(),
        });
    }

    /// Marks the tab whose follow context menu is open (step 87.2,
    /// item M103) — the frame-local count context clones only while
    /// this holds.
    pub(crate) fn set_count_menu_tab(&mut self, tab_id: TabId) {
        self.count_menu_tab = Some(tab_id);
    }

    pub(crate) fn clear_count_menu_tab(&mut self) {
        self.count_menu_tab = None;
    }

    pub(crate) fn count_menu_tab(&self) -> Option<TabId> {
        self.count_menu_tab
    }

    /// A clone of the answered counts (the frame-local suffix context
    /// reads it without borrowing `App`).
    pub(crate) fn counts_snapshot(&self) -> HashMap<String, ReferencedCount> {
        self.count_results.clone()
    }

    /// Processes at most one queued count read per frame (step 87.2,
    /// item M103): the separate read runs window-capped, the truncation
    /// rides the answer.
    pub(crate) fn poll_referenced_counts(&mut self) {
        let Some(request) = self.count_pending.first() else {
            return;
        };
        let request = request.clone();
        self.count_pending.remove(0);
        let table_def = self.snapshot.database.as_ref().and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&request.child))
                .map(|entry| {
                    TableDef::new(
                        entry.name.clone(),
                        entry
                            .fields
                            .iter()
                            .map(|field| FieldDef::new(field.name.clone(), field.kind))
                            .collect(),
                    )
                })
        });
        let Some(table_def) = table_def else {
            self.count_results
                .insert(request.key, ReferencedCount::Failed);
            return;
        };
        let rows = self.core.read_rows(
            &request.child,
            RowRange {
                start: 0,
                len: COUNT_CAP + 1,
            },
        );
        match rows {
            Ok(rows) => {
                let capped = rows.len() > COUNT_CAP;
                let survivors = datacraft_core::table_view::view_order(
                    &table_def,
                    &rows,
                    None,
                    &request.criteria,
                    &std::collections::BTreeMap::new(),
                )
                .map(|order| order.len())
                .unwrap_or(0);
                let count = if capped {
                    ReferencedCount::Truncated(survivors.min(COUNT_CAP))
                } else {
                    ReferencedCount::Exact(survivors)
                };
                self.count_results.insert(request.key, count);
            }
            Err(_) => {
                self.count_results
                    .insert(request.key, ReferencedCount::Failed);
            }
        }
    }

    /// Back/forward stack depth of a tab (the trailing buttons' gate).
    pub(crate) fn fk_back_len(&self, tab_id: TabId) -> usize {
        self.fk_histories.get(&tab_id).map_or(0, |h| h.back.len())
    }

    pub(crate) fn fk_forward_len(&self, tab_id: TabId) -> usize {
        self.fk_histories
            .get(&tab_id)
            .map_or(0, |h| h.forward.len())
    }

    /// The follow moved from `from` to the tab `to_id` (step 87.2,
    /// item M103): the source's position rides onto the target's back
    /// stack; the follow starts a fresh forward chain.
    pub(crate) fn fk_follow_moved(&mut self, from: &TabSnapshot, to_id: TabId) {
        let entry = NavEntry {
            table: from.object_name.clone(),
            filters: from.filters.clone(),
        };
        let history = self.fk_histories.entry(to_id).or_default();
        history.back.push(entry);
        if history.back.len() > HISTORY_CAP {
            history.back.remove(0);
        }
        history.forward.clear();
    }

    /// Back (step 87.2, item M103): pops the source position and
    /// navigates — a closed source reopens with its stored filter, a
    /// source with pending edits only activates. The stacks travel with
    /// the chain.
    pub(crate) fn fk_back(&mut self, tab_id: TabId) {
        let Some(current) = self.fk_current_entry(tab_id) else {
            return;
        };
        let Some(entry) = self.fk_pop_back(tab_id) else {
            return;
        };
        let forward_chain = self.fk_take_forward(tab_id);
        let onto_forward: Vec<NavEntry> = std::iter::once(current).chain(forward_chain).collect();
        let onto_back = self.fk_take_back(tab_id);
        self.fk_navigate(entry, onto_back, onto_forward);
    }

    /// Forward (step 87.2, item M103): the mirror of [`App::fk_back`].
    pub(crate) fn fk_forward(&mut self, tab_id: TabId) {
        let Some(current) = self.fk_current_entry(tab_id) else {
            return;
        };
        let Some(entry) = self.fk_pop_forward(tab_id) else {
            return;
        };
        let mut onto_back = self.fk_take_back(tab_id);
        onto_back.push(current);
        let onto_forward = self.fk_take_forward(tab_id);
        self.fk_navigate(entry, onto_back, onto_forward);
    }

    fn fk_current_entry(&self, tab_id: TabId) -> Option<NavEntry> {
        self.snapshot
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| NavEntry {
                table: tab.object_name.clone(),
                filters: tab.filters.clone(),
            })
    }

    fn fk_pop_back(&mut self, tab_id: TabId) -> Option<NavEntry> {
        self.fk_histories
            .get_mut(&tab_id)
            .and_then(|history| history.back.pop())
    }

    fn fk_pop_forward(&mut self, tab_id: TabId) -> Option<NavEntry> {
        self.fk_histories
            .get_mut(&tab_id)
            .and_then(|history| history.forward.pop())
    }

    fn fk_take_back(&mut self, tab_id: TabId) -> Vec<NavEntry> {
        self.fk_histories
            .get_mut(&tab_id)
            .map_or_else(Vec::new, |history| std::mem::take(&mut history.back))
    }

    fn fk_take_forward(&mut self, tab_id: TabId) -> Vec<NavEntry> {
        self.fk_histories
            .get_mut(&tab_id)
            .map_or_else(Vec::new, |history| std::mem::take(&mut history.forward))
    }

    /// Navigates to a stored position (step 87.2, item M103): the open
    /// target restores its stored filter — unless it carries pending
    /// edits, then it only activates — and a closed target reopens.
    /// The navigation stacks land on the target tab (the chain travels).
    fn fk_navigate(
        &mut self,
        entry: NavEntry,
        onto_back: Vec<NavEntry>,
        onto_forward: Vec<NavEntry>,
    ) {
        let open = self
            .snapshot
            .tabs
            .iter()
            .find(|tab| tab.object_name.eq_ignore_ascii_case(&entry.table))
            .map(|tab| tab.id);
        let target_id = match open {
            Some(id) => {
                // A source with pending edits is activated, not replaced
                // (step 87.2, item M103): its live filter state stays.
                if !self.has_staged_edits(id) {
                    crate::datasheet::follow::apply_filter_set(self, &entry.table, &entry.filters);
                }
                self.dispatch(Command::SetActiveTab { tab_id: id });
                Some(id)
            }
            None => {
                self.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
                    datacraft_core::ObjectKind::Table,
                    entry.table.clone(),
                )));
                // A failed open (the table is gone) lets the chain die
                // with the navigation.
                let id = self
                    .snapshot
                    .tabs
                    .iter()
                    .find(|tab| tab.object_name.eq_ignore_ascii_case(&entry.table))
                    .map(|tab| tab.id);
                if id.is_some() {
                    crate::datasheet::follow::apply_filter_set(self, &entry.table, &entry.filters);
                }
                id
            }
        };
        if let Some(id) = target_id {
            match self.fk_histories.get_mut(&id) {
                Some(history) => {
                    history.back = onto_back;
                    history.forward = onto_forward;
                }
                None => {
                    self.fk_histories.insert(
                        id,
                        NavHistory {
                            back: onto_back,
                            forward: onto_forward,
                        },
                    );
                }
            }
        }
    }
}
