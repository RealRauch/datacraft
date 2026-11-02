//! Unit tests for the Jet4 port adapter (step 16.1 split; the fixture-level
//! contracts live in `tests/port_adapter.rs`).

use super::*;
use crate::jet::JetType;
use crate::jet::tdef::{JetColumn, flags};
use crate::port_schema::field_def;
use crate::types::StorageValue;
use datacraft_core::schema::{BinaryKind, FieldKind};

/// The physical -> domain mapping is 1:1 and total over the value
/// vocabulary (fixture rows do not exercise every family).
#[test]
fn every_storage_value_maps_to_its_domain_family() {
    let cases: Vec<(StorageValue, FieldValue)> = vec![
        (StorageValue::Null, FieldValue::Null),
        (StorageValue::Bit(true), FieldValue::Boolean(true)),
        (StorageValue::Int(7), FieldValue::Integer(7)),
        (StorageValue::Float(1.5), FieldValue::Float(1.5)),
        (StorageValue::Currency(2500), FieldValue::Currency(2500)),
        (
            StorageValue::DateTime(45_000.5),
            FieldValue::DateTime(45_000.5),
        ),
        (StorageValue::Text("x".into()), FieldValue::Text("x".into())),
        (
            StorageValue::Binary(vec![1, 2]),
            FieldValue::Binary(vec![1, 2]),
        ),
        (StorageValue::Guid([9; 16]), FieldValue::Guid([9; 16])),
        (StorageValue::Decimal(1234), FieldValue::Decimal(1234)),
    ];
    for (stored, expected) in cases {
        assert_eq!(to_field_value(&stored), expected);
    }
}

/// A complex column without a `MSysComplexColumns` mapping keeps the
/// documented attachment fallback (files without the catalog still read).
#[test]
fn an_unmapped_complex_column_falls_back_to_attachment() {
    let file = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("testV2000.mdb"),
    )
    .expect("fixture");
    let reader = Jet4TableReader::new(file).expect("reader");
    let complex = ComplexCatalog::default();
    let column = JetColumn::new("Attachments", JetType::Complex, flags::FIXED, 0, 0, 4);
    let field = field_def(&column, None, &complex, &reader).expect("field");
    assert_eq!(field.kind, FieldKind::Binary(BinaryKind::Attachment));
}
