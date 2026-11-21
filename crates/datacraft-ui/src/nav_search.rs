//! Nav-pane object search (step 92.3, item L48).
//!
//! A UI-owned presentation filter below the pane header: the query is
//! session state on the app (never persisted, never in the sidecar), the
//! match is a case-insensitive substring over the object name, and an
//! empty or blank query lets the pane render unchanged. Sections with no
//! match hide while a query is active; when nothing matches at all the
//! pane shows one localized empty state instead of a bare pane.

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
