//! Database lifecycle follow-through (steps 69.3/75.1, items M80/L41) —
//! split from `app.rs` for the file-size gate (L30).
//!
//! The landing rule: the shell shows the File backstage while nothing is
//! open (cold start and after closing the last database); an open leaves
//! the backstage and returns Home.

use datacraft_core::event::Event;
use datacraft_core::snapshot::UiStateSnapshot;

use crate::app::App;
use crate::backstage::BackstagePage;

impl App {
    /// The tab that matches `snapshot`: without a database the File
    /// backstage, otherwise the workspace's Home tab.
    pub(crate) fn landing_tab(snapshot: &UiStateSnapshot) -> &'static str {
        if snapshot.database.is_none() {
            "file"
        } else {
            "home"
        }
    }

    /// Follows the database lifecycle events (step 69.3, M80; step 75.1,
    /// L41): an open loads the sidecar path, feeds the recent list and
    /// leaves the backstage; closing the last database returns to it.
    pub(crate) fn follow_database_lifecycle(&mut self, events: &[Event]) {
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. }))
            && let Some(db) = self.snapshot.database.as_ref()
        {
            // ADR-1: the design layer lives next to the database file.
            let path = db.path.clone();
            self.sidecar_path = datacraft_core::schema::SchemaDocument::sidecar_path(&path)
                .to_string_lossy()
                .into_owned();
            // Item U15: a successful open feeds the bounded recent list.
            self.record_recent_database(&path);
            // The shell follows the open: leave the backstage, return Home.
            self.active_ribbon_tab = "home";
            self.backstage_page = BackstagePage::Home;
        }
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseClosed))
        {
            self.sidecar_path.clear();
            // Nothing is open any more: the File backstage is the honest
            // landing surface (step 75.1, L41) — the same rule as the cold
            // start. The ribbon stays reachable through the Back row.
            if self.snapshot.database.is_none() {
                self.active_ribbon_tab = Self::landing_tab(&self.snapshot);
                self.backstage_page = BackstagePage::Home;
            }
        }
    }
}
