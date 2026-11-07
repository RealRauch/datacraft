//! Characterisation of the engine's text sort keys (step 49.2, item M63).
//!
//! Every expected byte string below is **observed engine output**: the index
//! leaf entries of databases the DAO oracle created (probe batch 2026-11-07,
//! `docs/FORMAT-JET.md` "Fresh-create template and the engine's open gates").
//! The tests pin the full key grammar — primary weights, the secondary
//! (accent) level, ignorable characters with their position encoding, the
//! space trim and the exact separator/terminator bytes.

use crate::error::StorageError;
use crate::jet::collation::text_sort_key;

fn key(text: &str) -> Vec<u8> {
    text_sort_key(text).unwrap_or_else(|error| panic!("{text:?}: {error}"))
}

/// `fresh-ref.mdb` page 7 (the catalog's ParentIdName leaf) keys.
#[test]
fn catalog_names_match_the_engine_written_leaf_bytes() {
    assert_eq!(
        key("Databases"),
        [
            0x4F, 0x4A, 0x6D, 0x4A, 0x4C, 0x4A, 0x6B, 0x51, 0x6B, 0x01, 0x00
        ]
    );
    assert_eq!(
        key("Relationships"),
        [
            0x69, 0x51, 0x5E, 0x4A, 0x6D, 0x59, 0x64, 0x62, 0x6B, 0x57, 0x59, 0x66, 0x6B, 0x01,
            0x00
        ]
    );
    assert_eq!(
        key("Tables"),
        [0x6D, 0x4A, 0x4C, 0x5E, 0x51, 0x6B, 0x01, 0x00]
    );
    assert_eq!(
        key("MSysObjects"),
        [
            0x60, 0x6B, 0x76, 0x6B, 0x64, 0x4C, 0x5B, 0x51, 0x4D, 0x6D, 0x6B, 0x01, 0x00
        ]
    );
    assert_eq!(
        key("MSysDb"),
        [0x60, 0x6B, 0x76, 0x6B, 0x4F, 0x4C, 0x01, 0x00]
    );
    assert_eq!(
        key("MSysAces"),
        [0x60, 0x6B, 0x76, 0x6B, 0x4A, 0x4D, 0x51, 0x6B, 0x01, 0x00]
    );
    // `eng-table.mdb` page 7: the engine-created user table.
    assert_eq!(key("Foo"), [0x53, 0x64, 0x64, 0x01, 0x00]);
}

/// Case folds at the primary level; trailing spaces are trimmed; the empty
/// string keeps the bare separator + terminator (`01 00`).
#[test]
fn case_folds_and_trailing_spaces_are_trimmed() {
    assert_eq!(key("A"), key("a"));
    assert_eq!(key("aB"), key("ab"));
    assert_eq!(key(""), [0x01, 0x00]);
    assert_eq!(key(" "), [0x01, 0x00]);
    assert_eq!(key("  "), [0x01, 0x00]);
    assert_eq!(key("a "), [0x4A, 0x01, 0x00]);
    assert_eq!(key("a  b"), [0x4A, 0x07, 0x07, 0x4C, 0x01, 0x00]);
    assert_eq!(key(" a"), [0x07, 0x4A, 0x01, 0x00]);
}

/// Accented characters add one secondary byte per sort element; the level is
/// truncated after the last non-default secondary.
#[test]
fn accents_emit_the_secondary_level() {
    assert_eq!(key("é"), [0x51, 0x01, 0x0E, 0x00]);
    assert_eq!(key("éa"), [0x51, 0x4A, 0x01, 0x0E, 0x00]);
    assert_eq!(key("aé"), [0x4A, 0x51, 0x01, 0x02, 0x0E, 0x00]);
    assert_eq!(
        key("Müller"),
        [0x60, 0x6F, 0x5E, 0x5E, 0x51, 0x69, 0x01, 0x02, 0x13, 0x00]
    );
    assert_eq!(
        key("Café"),
        [0x4D, 0x4A, 0x53, 0x51, 0x01, 0x02, 0x02, 0x02, 0x0E, 0x00]
    );
    // Two-element ligatures repeat their (default) secondary per element.
    assert_eq!(key("Æ"), [0x4A, 0x51, 0x01, 0x00]);
    assert_eq!(key("Æé"), [0x4A, 0x51, 0x51, 0x01, 0x02, 0x02, 0x0E, 0x00]);
    assert_eq!(
        key("Straße"),
        [0x6B, 0x6D, 0x69, 0x4A, 0x6B, 0x6B, 0x51, 0x01, 0x00]
    );
}

/// Apostrophe and hyphen are ignorable: they contribute no primary weight and
/// are re-encoded at the last level as `80|hi lo 06 weight`, where the
/// position is `4*k + 7` and `k` counts the sort elements emitted before them
/// (`ß` contributes two, a two-byte weight like `¢` only one — probe 2026-11-07).
#[test]
fn ignorable_characters_encode_their_position() {
    assert_eq!(
        key("-"),
        [0x01, 0x01, 0x01, 0x01, 0x80, 0x07, 0x06, 0x82, 0x00]
    );
    assert_eq!(
        key("'"),
        [0x01, 0x01, 0x01, 0x01, 0x80, 0x07, 0x06, 0x80, 0x00]
    );
    assert_eq!(
        key("a-b"),
        [
            0x4A, 0x4C, 0x01, 0x01, 0x01, 0x01, 0x80, 0x0B, 0x06, 0x82, 0x00
        ]
    );
    assert_eq!(
        key("a--b"),
        [
            0x4A, 0x4C, 0x01, 0x01, 0x01, 0x01, 0x80, 0x0B, 0x06, 0x82, 0x80, 0x0B, 0x06, 0x82,
            0x00
        ]
    );
    assert_eq!(
        key("a-b-c"),
        [
            0x4A, 0x4C, 0x4D, 0x01, 0x01, 0x01, 0x01, 0x80, 0x0B, 0x06, 0x82, 0x80, 0x0F, 0x06,
            0x82, 0x00
        ]
    );
    // An accent level shifts the separators but keeps the ignorable block.
    assert_eq!(
        key("é-"),
        [
            0x51, 0x01, 0x0E, 0x01, 0x01, 0x01, 0x80, 0x0B, 0x06, 0x82, 0x00
        ]
    );
    // Positions count primary bytes: ß contributes two.
    assert_eq!(
        key("a-ß-b"),
        [
            0x4A, 0x6B, 0x6B, 0x4C, 0x01, 0x01, 0x01, 0x01, 0x80, 0x0B, 0x06, 0x82, 0x80, 0x13,
            0x06, 0x82, 0x00
        ]
    );
}

/// Positions above 0xFF widen to the marked two-byte weight form.
#[test]
fn ignorable_positions_above_one_byte() {
    let name = format!("{}-b", "a".repeat(63));
    let mut expected = vec![0x4A; 63];
    expected.extend_from_slice(&[0x4C, 0x01, 0x01, 0x01, 0x01, 0x81, 0x03, 0x06, 0x82, 0x00]);
    assert_eq!(key(&name), expected);
}

/// Characters outside the observed CP1252 repertoire answer a precise
/// `Unsupported` naming the character — no silent replacement.
#[test]
fn characters_outside_the_observed_table_are_refused() {
    for text in ["Д", "\u{1F600}", "\t"] {
        let error = text_sort_key(text).expect_err("character is outside the table");
        match error {
            StorageError::Unsupported { reason } => {
                assert!(reason.contains("sort key"), "{reason}");
                assert!(
                    reason.contains(&format!("U+{:04X}", text.chars().next().unwrap() as u32)),
                    "{reason}"
                );
            }
            other => panic!("expected Unsupported, found {other:?}"),
        }
    }
}
