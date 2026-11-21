//! Navigation-pane local groups and view preference (step 92.5, item
//! L48).
//!
//! The pane grows the last two L48 surfaces: an **Object Details** view
//! (one flat, searchable list across the categories) and **local groups
//! with colours** — the reference client's custom groups. Both are
//! UI-owned preferences stored in `ui.json` keyed by the database path
//! (ADR-5); the `.datacraft` sidecar and the database file never see
//! them.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use datacraft_core::{ObjectKind, ObjectId};

    use super::{NavColour, NavPrefs, NavView, prefs_key};

    /// The view values round-trip; unknown or absent values stay in the
    /// tree view (step 92.5, item L48).
    #[test]
    fn nav_view_values_round_trip_and_unknowns_tree() {
        assert_eq!(NavView::from_value(Some("tree")), NavView::Tree);
        assert_eq!(NavView::from_value(Some("details")), NavView::Details);
        assert_eq!(NavView::from_value(None), NavView::Tree);
        assert_eq!(NavView::from_value(Some("grid")), NavView::Tree);
        for view in [NavView::Tree, NavView::Details] {
            assert_eq!(NavView::from_value(Some(view.as_value())), view);
        }
    }

    /// The colour values round-trip; unknown or absent values take the
    /// neutral default (step 92.5, item L48).
    #[test]
    fn nav_colour_values_round_trip_and_unknowns_default() {
        for colour in NavColour::ALL {
            assert_eq!(NavColour::from_value(Some(colour.as_value())), *colour);
        }
        assert_eq!(NavColour::from_value(None), NavColour::default());
        assert_eq!(NavColour::from_value(Some("pink")), NavColour::default());
    }

    /// The default preference is the tree view without groups.
    #[test]
    fn nav_prefs_default_is_the_tree_without_groups() {
        let prefs = NavPrefs::default();
        assert_eq!(prefs.view, NavView::Tree);
        assert!(prefs.groups.is_empty());
    }

    /// The per-database key is the database path itself.
    #[test]
    fn prefs_key_uses_the_database_path() {
        assert_eq!(prefs_key(Path::new("shop.mdb")), "shop.mdb");
    }

    /// Group creation assigns palette colours deterministically by
    /// position (step 92.5, item L48).
    #[test]
    fn group_colours_rotate_through_the_palette() {
        let first = NavColour::for_position(0);
        let second = NavColour::for_position(1);
        let ninth = NavColour::for_position(8);
        assert_ne!(first, second);
        assert_eq!(ninth, first, "the palette wraps");
    }

    /// Removing one object leaves the other group members untouched
    /// (step 92.5, item L48).
    #[test]
    fn removing_an_object_keeps_the_group() {
        let mut group = super::NavGroup {
            name: "Mine".to_owned(),
            colour: NavColour::Teal,
            objects: vec![
                ObjectId::new(ObjectKind::Table, "Customers"),
                ObjectId::new(ObjectKind::Table, "Orders"),
            ],
        };
        group.retain_object(&ObjectId::new(ObjectKind::Table, "Customers"));
        assert_eq!(group.objects.len(), 1);
        assert_eq!(group.objects[0].name, "Orders");
    }
}
