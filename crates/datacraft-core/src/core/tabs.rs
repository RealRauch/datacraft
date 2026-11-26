//! Tab reducers (split from `core.rs` to keep every file under the size
//! gate): open/close/activate/switch document tabs.

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::ids::TabId;
use crate::port::PortError;
use crate::state::{TabMode, TabState};

/// The side the close-to-side commands anchor on (step 88.3, item L49).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// Every tab strictly left of the anchor.
    Left,
    /// Every tab strictly right of the anchor.
    Right,
}

impl Side {
    /// The word for the precise empty-side failure.
    fn word(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

impl crate::core::Core {
    /// Opens (or re-activates) an object tab. An explicit mode is honoured
    /// for kinds with a design view and applied to existing tabs as well
    /// (step 21.5, item U9).
    pub(crate) fn open_object(
        &mut self,
        object: crate::ObjectId,
        mode: Option<TabMode>,
    ) -> Vec<Event> {
        if mode.is_some() && !object.kind.has_design_view() {
            return failed(
                FailureCode::Unsupported,
                format!(
                    "the {} object {:?} has no design view",
                    object.kind.slug(),
                    object.name
                ),
            );
        }
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // A linked table is a catalog record, not a readable table (phase
        // 100, step 100.2; item M131): opening one answers a precise
        // refusal instead of a lookup error — roadmap §9 stops at the
        // inventory, nothing resolves or dials.
        if object.kind == ObjectKind::Table
            && db
                .linked_tables
                .iter()
                .any(|link| link.name.eq_ignore_ascii_case(&object.name))
            && !db
                .tables
                .iter()
                .any(|table| table.name.eq_ignore_ascii_case(&object.name))
        {
            return failed(
                FailureCode::Unsupported,
                format!(
                    "{:?} is a linked table — its target is not resolved (the link inventory is read-only)",
                    object.name
                ),
            );
        }
        let exists = match object.kind {
            ObjectKind::Table => db
                .tables
                .iter()
                .any(|t| t.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Query => {
                db.queries
                    .iter()
                    .any(|q| q.name.to_lowercase() == object.name.to_lowercase())
                    || db
                        .native_queries
                        .iter()
                        .any(|q| q.name.to_lowercase() == object.name.to_lowercase())
            }
            ObjectKind::View => {
                db.views
                    .iter()
                    .any(|v| v.name.eq_ignore_ascii_case(&object.name))
                    || db
                        .native_views
                        .iter()
                        .any(|v| v.name.eq_ignore_ascii_case(&object.name))
            }
            ObjectKind::Form => db
                .forms
                .iter()
                .any(|f| f.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Report => db
                .reports
                .iter()
                .any(|r| r.name.to_lowercase() == object.name.to_lowercase()),
            ObjectKind::Macro => db
                .macros
                .iter()
                .any(|m| m.name.eq_ignore_ascii_case(&object.name)),
            ObjectKind::Module => db
                .vba_project
                .as_ref()
                .is_some_and(|project| project.module(&object.name).is_some()),
        };
        if !exists {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(object.name).to_string(),
            );
        }
        // Idle-tab reuse (step 88.3, L49): an object that is already open
        // re-activates its tab — never a second one. The match is
        // case-insensitive like the existence check, so a differently
        // cased name cannot open a duplicate tab of the same object.
        if let Some(existing) = self
            .state
            .tabs
            .iter()
            .find(|tab| {
                tab.object.kind == object.kind
                    && tab.object.name.to_lowercase() == object.name.to_lowercase()
            })
            .cloned()
        {
            self.state.active_tab = Some(existing.id);
            let mut events = vec![Event::ActiveTabChanged {
                tab_id: Some(existing.id),
            }];
            if let Some(mode) = mode {
                if let Some(tab) = self.state.tabs.iter_mut().find(|tab| tab.id == existing.id) {
                    tab.mode = mode;
                }
                events.push(Event::TabModeSet {
                    tab_id: existing.id,
                    mode,
                });
            }
            return events;
        }
        let tab_id = TabId::new(self.state.next_tab_id);
        self.state.next_tab_id += 1;
        self.state.tabs.push(TabState {
            id: tab_id,
            object: object.clone(),
            mode: mode.unwrap_or(TabMode::Datasheet),
        });
        self.state.active_tab = Some(tab_id);
        vec![
            Event::TabOpened { tab_id, object },
            Event::ActiveTabChanged {
                tab_id: Some(tab_id),
            },
        ]
    }

    pub(crate) fn close_tab(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        }
        self.remove_tab(tab_id);
        if self.state.active_tab == Some(tab_id) {
            self.state.active_tab = self.neighbor_of(tab_id);
            let mut events = vec![Event::TabClosed { tab_id }];
            events.push(Event::ActiveTabChanged {
                tab_id: self.state.active_tab,
            });
            return events;
        }
        vec![Event::TabClosed { tab_id }]
    }

    /// Closes every tab but `tab_id` (step 21.6, item U10); the kept tab
    /// stays open and becomes active.
    pub(crate) fn close_other_tabs(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        }
        let closed: Vec<TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| tab.id != tab_id)
            .map(|tab| tab.id)
            .collect();
        self.state.tabs.retain(|tab| tab.id == tab_id);
        let mut events: Vec<Event> = closed
            .into_iter()
            .map(|tab_id| Event::TabClosed { tab_id })
            .collect();
        if self.state.active_tab != Some(tab_id) {
            self.state.active_tab = Some(tab_id);
            events.push(Event::ActiveTabChanged {
                tab_id: Some(tab_id),
            });
        }
        events
    }

    /// Closes every tab (step 21.6, item U10); the workspace ends empty.
    pub(crate) fn close_all_tabs(&mut self) -> Vec<Event> {
        if self.state.tabs.is_empty() {
            return failed(FailureCode::Lifecycle, "no tabs are open");
        }
        let closed: Vec<TabId> = self.state.tabs.iter().map(|tab| tab.id).collect();
        self.state.tabs.clear();
        self.state.active_tab = None;
        let mut events: Vec<Event> = closed
            .into_iter()
            .map(|tab_id| Event::TabClosed { tab_id })
            .collect();
        events.push(Event::ActiveTabChanged { tab_id: None });
        events
    }

    /// Closes every tab strictly to one side of the anchor tab (step
    /// 88.3, item L49); the anchor survives and becomes active only when
    /// the active tab was on the closed side. An empty side answers the
    /// same precise failure as `close_all_tabs`.
    pub(crate) fn close_tabs_to_side(&mut self, tab_id: TabId, side: Side) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        }
        let Some(anchor) = self.state.tabs.iter().position(|tab| tab.id == tab_id) else {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        };
        let side_range = match side {
            Side::Left => 0..anchor,
            Side::Right => anchor + 1..self.state.tabs.len(),
        };
        if side_range.is_empty() {
            return failed(
                FailureCode::Lifecycle,
                format!("no tabs to the {} of the open document", side.word()),
            );
        }
        let closed: Vec<TabId> = self.state.tabs[side_range.clone()]
            .iter()
            .map(|tab| tab.id)
            .collect();
        let closed_set: std::collections::HashSet<TabId> = closed.iter().copied().collect();
        self.state.tabs.retain(|tab| !closed_set.contains(&tab.id));
        let mut events: Vec<Event> = closed
            .iter()
            .map(|closed_id| Event::TabClosed { tab_id: *closed_id })
            .collect();
        if self
            .state
            .active_tab
            .is_some_and(|active| closed_set.contains(&active))
        {
            self.state.active_tab = Some(tab_id);
            events.push(Event::ActiveTabChanged {
                tab_id: Some(tab_id),
            });
        }
        events
    }

    pub(crate) fn set_active_tab(&mut self, tab_id: TabId) -> Vec<Event> {
        if !self.state.tabs.iter().any(|tab| tab.id == tab_id) {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        }
        self.state.active_tab = Some(tab_id);
        vec![Event::ActiveTabChanged {
            tab_id: Some(tab_id),
        }]
    }

    /// Switches one tab's view (step 21.1, item U5). Only kinds with a
    /// design view accept the switch; the failure is precise.
    pub(crate) fn set_tab_mode(&mut self, tab_id: TabId, mode: TabMode) -> Vec<Event> {
        let Some(tab) = self.state.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return failed(FailureCode::NotFound, format!("unknown tab {tab_id}"));
        };
        if !tab.object.kind.has_design_view() {
            return failed(
                FailureCode::Unsupported,
                format!(
                    "the {} tab {:?} has no design view",
                    tab.object.kind.slug(),
                    tab.object.name
                ),
            );
        }
        tab.mode = mode;
        vec![Event::TabModeSet { tab_id, mode }]
    }

    /// Activation rule: the highest remaining tab id below the closed one,
    /// else the lowest remaining id, else none.
    fn neighbor_of(&self, closed: TabId) -> Option<TabId> {
        let below = self
            .state
            .tabs
            .iter()
            .map(|tab| tab.id)
            .filter(|id| id.value() < closed.value())
            .max();
        below.or_else(|| self.state.tabs.iter().map(|tab| tab.id).min())
    }

    pub(crate) fn remove_tab(&mut self, tab_id: TabId) {
        self.state.tabs.retain(|tab| tab.id != tab_id);
    }
}
