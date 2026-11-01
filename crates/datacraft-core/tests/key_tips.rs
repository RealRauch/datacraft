//! Contract suite for ribbon key tips (step 21.8, item U12).
//!
//! Key tips are derived from the spec ids — never invented labels — and the
//! derivation is deterministic, so the overlay always matches the ribbon.

use datacraft_core::keys::key_tips;

#[test]
fn key_tips_use_the_first_alphanumeric_character() {
    let ids = ["file", "home", "create", "external_data", "database_tools"];
    assert_eq!(key_tips(&ids), ["F", "H", "C", "E", "D"]);
}

#[test]
fn colliding_tips_are_numbered_in_spec_order() {
    let ids = ["table", "table_design", "tablet", "query_design"];
    assert_eq!(key_tips(&ids), ["T", "T2", "T3", "Q"]);
}

#[test]
fn leading_punctuation_is_skipped_and_lists_may_be_empty() {
    assert_eq!(key_tips(&["_hidden", "7zip"]), ["H", "7"]);
    let empty: [&str; 0] = [];
    assert!(key_tips(&empty).is_empty());
}

#[test]
fn tips_are_unique_case_insensitively_across_the_list() {
    let ids = ["alpha", "Alpha", "beta"];
    let tips = key_tips(&ids);
    assert_eq!(tips, ["A", "A2", "B"]);
    assert_eq!(tips.len(), ids.len());
}
