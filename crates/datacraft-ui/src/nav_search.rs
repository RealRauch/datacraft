//! Nav-pane object search (step 92.3, item L48).
//!
//! A UI-owned presentation filter below the pane header: the query is
//! session state on the app (never persisted, never in the sidecar), the
//! match is a case-insensitive substring over the object name, and an
//! empty or blank query lets the pane render unchanged. Sections with no
//! match hide while a query is active; when nothing matches at all the
//! pane shows one localized empty state instead of a bare pane.

use eframe::egui;

use crate::app::App;
use crate::i18n::t;

/// Whether an object name passes the pane's search query: a blank query
/// matches everything, otherwise a case-insensitive substring test.
pub fn matches(query: &str, name: &str) -> bool {
    let query = query.trim();
    query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
}

/// Renders the search field below the pane header and returns the active
/// query. The field records its rect under `nav.search` — a
/// placeholder-only input carries no label node, so the kittest reaches
/// it through the probe seam.
pub fn search_field(app: &mut App, ui: &mut egui::Ui) -> String {
    let mut query = std::mem::take(&mut app.nav_search);
    let has_query = !query.trim().is_empty();
    ui.horizontal(|ui| {
        // The clear affordance appears only while it can do something
        // (LESSONS #29 — no dead controls).
        let clear_room = if has_query { 96.0 } else { 0.0 };
        let field = ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text(t("Search objects"))
                .desired_width((ui.available_width() - clear_room).max(80.0)),
        );
        crate::layout_probe::record("nav.search", field.rect);
        if has_query && ui.small_button(t("Clear")).clicked() {
            query.clear();
        }
    });
    app.nav_search = query;
    app.nav_search.clone()
}

#[cfg(test)]
mod tests {
    use super::matches;

    /// An empty or blank query is no filter — the pane renders unchanged
    /// (step 92.3, item L48).
    #[test]
    fn an_empty_or_blank_query_matches_every_object() {
        assert!(matches("", "Anything"));
        assert!(matches("   ", "Anything"));
        assert!(matches("", ""));
    }

    /// The match is a case-insensitive substring over the object name
    /// (step 92.3, item L48).
    #[test]
    fn the_query_matches_case_insensitively_across_the_whole_name() {
        assert!(matches("cust", "Customers"));
        assert!(matches("ORDERS", "ServiceOrders"));
        assert!(matches("ice", "ServiceOrders"));
        assert!(!matches("cust", "ServiceOrders"));
        assert!(!matches("zyx", "Customers"));
    }
}
