//! Index metadata encode/decode contracts (step 32.2, item M15).

use datacraft_core::schema::IndexMode;

use super::*;
use crate::jet::JetType;
use crate::jet::tdef::JetColumn;

fn index(name: &str, columns: Vec<u16>, unique: bool) -> JetIndex {
    JetIndex {
        name: name.to_owned(),
        columns,
        unique,
        required: false,
        primary: false,
        foreign: false,
    }
}

#[test]
fn single_and_multi_column_indexes_round_trip() {
    let indexes = vec![
        index("one", vec![1], true),
        JetIndex {
            primary: true,
            required: true,
            ..index("two", vec![0, 3], false)
        },
        JetIndex {
            foreign: true,
            ..index("three", vec![2], false)
        },
    ];
    let (section, logical, real) = encode_section(&indexes);
    assert_eq!(logical, 3);
    assert_eq!(real, 3);
    let (decoded, consumed) = decode_section(&section, 3, 3).expect("decode");
    assert_eq!(consumed, section.len());
    assert_eq!(decoded, indexes);
}

#[test]
fn single_column_helpers_map_the_index_mode() {
    assert_eq!(
        index("a", vec![0], true).index_mode(),
        Some(IndexMode::NoDuplicates)
    );
    assert_eq!(
        index("b", vec![0], false).index_mode(),
        Some(IndexMode::DuplicatesOk)
    );
    assert_eq!(index("c", vec![0, 1], true).index_mode(), None);
    assert_eq!(index("d", vec![], true).single_column(), None);
}

#[test]
fn build_section_uses_the_domain_index_mode() {
    let columns = vec![
        JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
        JetColumn::new("name", JetType::Text, 0x02, 1, 0, 20),
    ];
    let mut columns = columns;
    columns[0].indexed = IndexMode::NoDuplicates;
    columns[1].indexed = IndexMode::DuplicatesOk;
    let (section, logical, real) = build_section(&columns);
    assert_eq!((logical, real), (2, 2));
    let (indexes, _) = decode_section(&section, 2, 2).expect("decode");
    assert_eq!(indexes[0].name, "id");
    assert!(indexes[0].unique);
    assert_eq!(indexes[1].name, "name");
    assert!(!indexes[1].unique);
}

#[test]
fn truncated_or_unknown_sections_are_corrupt() {
    let (section, _, _) = encode_section(&[index("a", vec![0], true)]);
    let error = decode_section(&section[..10], 1, 1).expect_err("truncated blocks");
    assert!(error.to_string().contains("shorter"), "{error}");

    let mut bad_real = section.clone();
    bad_real[0] = 0;
    let error = decode_section(&bad_real, 1, 1).expect_err("real tag");
    assert!(
        error.to_string().contains("real index block tag"),
        "{error}"
    );

    let mut bad_logical = section.clone();
    bad_logical[REAL_INDEX_BLOCK] = 0;
    let error = decode_section(&bad_logical, 1, 1).expect_err("logical tag");
    assert!(
        error.to_string().contains("logical index block tag"),
        "{error}"
    );

    let mut bad_reference = section.clone();
    bad_reference[REAL_INDEX_BLOCK + 8..REAL_INDEX_BLOCK + 12].copy_from_slice(&7u32.to_le_bytes());
    let error = decode_section(&bad_reference, 1, 1).expect_err("missing real");
    assert!(error.to_string().contains("missing real block"), "{error}");

    let mut bad_name = section.clone();
    bad_name[REAL_INDEX_BLOCK + LOGICAL_INDEX_BLOCK..REAL_INDEX_BLOCK + LOGICAL_INDEX_BLOCK + 2]
        .copy_from_slice(&999u16.to_le_bytes());
    let error = decode_section(&bad_name, 1, 1).expect_err("truncated name");
    assert!(
        error.to_string().contains("truncated index name"),
        "{error}"
    );
}
