//! Contract suite for navigation-pane ordering (step 21.5, item U9).
//!
//! The ordering is a pure function over the snapshot-derived list, so the
//! pane's group sort is deterministic and testable without a window.

use datacraft_core::ObjectKind;
use datacraft_core::nav::{NavSortOrder, sort_entries};

fn entries() -> Vec<(String, ObjectKind)> {
    vec![
        ("zeta".to_owned(), ObjectKind::Report),
        ("Alpha".to_owned(), ObjectKind::Table),
        ("beta".to_owned(), ObjectKind::Query),
        ("alpha".to_owned(), ObjectKind::Form),
        ("Gamma".to_owned(), ObjectKind::Table),
    ]
}

#[test]
fn name_order_is_case_insensitive_and_total() {
    let mut list = entries();
    sort_entries(&mut list, NavSortOrder::Name);
    let names: Vec<&str> = list.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["Alpha", "alpha", "beta", "Gamma", "zeta"]);
}

#[test]
fn type_order_follows_the_access_kind_sequence() {
    let mut list = entries();
    sort_entries(&mut list, NavSortOrder::Type);
    let kinds: Vec<ObjectKind> = list.iter().map(|(_, kind)| *kind).collect();
    assert_eq!(
        kinds,
        [
            ObjectKind::Table,
            ObjectKind::Table,
            ObjectKind::Query,
            ObjectKind::Form,
            ObjectKind::Report,
        ]
    );
    let names: Vec<&str> = list.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["Alpha", "Gamma", "beta", "alpha", "zeta"],
        "within a kind the name orders case-insensitively"
    );
}

#[test]
fn sorting_is_idempotent_and_handles_empty_lists() {
    let mut empty: Vec<(String, ObjectKind)> = Vec::new();
    sort_entries(&mut empty, NavSortOrder::Name);
    assert!(empty.is_empty());

    let mut list = entries();
    sort_entries(&mut list, NavSortOrder::Type);
    let once = list.clone();
    sort_entries(&mut list, NavSortOrder::Type);
    assert_eq!(list, once);
}
