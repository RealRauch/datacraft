//! Native saved queries contract (step 15.3, item M3).
//!
//! The pinned `queryTestV2000.mdb` fixture carries nine native queries of
//! every object-flag type. MSysQueries rows decode into `NativeQueryDef`s
//! (attribute codes in `docs/FORMAT-JET.md`); inline memo payloads carry
//! the Access 8-byte zero header and the `FF FE` compression header with
//! 0x00 segment switches, so expressions read back verbatim.

use datacraft_storage::jet::{Jet4TableReader, NativeQueryKind};

fn reader() -> Jet4TableReader {
    let file = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/queryTestV2000.mdb"
    ))
    .expect("pinned fixture");
    Jet4TableReader::new(file).expect("fixture opens")
}

#[test]
fn every_object_flag_type_decodes_with_its_kind() {
    let queries = reader().native_queries().expect("queries decode");
    let kinds: Vec<(&str, NativeQueryKind)> = queries
        .iter()
        .map(|query| (query.name.as_str(), query.kind))
        .collect();
    assert_eq!(kinds.len(), 9, "the fixture carries nine queries");
    for (name, kind) in [
        ("SelectQuery", NativeQueryKind::Select),
        ("CrosstabQuery", NativeQueryKind::Crosstab),
        ("DeleteQuery", NativeQueryKind::Delete),
        ("UpdateQuery", NativeQueryKind::Update),
        ("AppendQuery", NativeQueryKind::Append),
        ("MakeTableQuery", NativeQueryKind::MakeTable),
        ("DataDefinitionQuery", NativeQueryKind::DataDefinition),
        ("PassthroughQuery", NativeQueryKind::Passthrough),
        ("UnionQuery", NativeQueryKind::Union),
    ] {
        assert!(
            kinds.contains(&(name, kind)),
            "{name} must decode as {kind:?}: {kinds:?}"
        );
    }
}

#[test]
fn tables_and_expressions_read_back_verbatim() {
    let queries = reader().native_queries().expect("queries decode");
    let make_table = queries
        .iter()
        .find(|query| query.name == "MakeTableQuery")
        .expect("MakeTableQuery present");
    assert!(make_table.tables.contains(&"Table1".to_owned()), "{make_table:?}");
    assert!(make_table.tables.contains(&"Table2".to_owned()), "{make_table:?}");
    // The join expression is a memo payload: 8-byte zero header + FF FE
    // compression header + 1-byte text.
    let joined = queries
        .iter()
        .flat_map(|query| query.columns.iter())
        .find(|column| {
            column
                .expression
                .as_deref()
                .is_some_and(|expression| expression.contains("Table1"))
        })
        .expect("a join/criterion expression mentioning Table1");
    assert!(
        !joined.expression.expect("checked").contains('\u{feff}'),
        "the compression header must be stripped: {joined:?}"
    );
}
