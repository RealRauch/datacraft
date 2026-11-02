//! Storage-port defaults and native-query mapping (phase 24 coverage gate).
//!
//! Backends rely on the trait's default answers for the optional surfaces;
//! a minimal port pins those defaults, and the native-query mapping is
//! tested across its disqualifiers and its sort projection.

use std::path::Path;

use datacraft_core::port::{
    NativeQuery, NativeQueryColumn, NativeQueryKind, OpenedDatabase, PortError, Relationship,
    RowRange, SchemaChange, StoragePort,
};
use datacraft_core::qbe::SortOrder;
use datacraft_core::schema::{FieldValue, TableDef};

/// Implements only the required trait methods; everything else is default.
struct MinimalPort;

impl StoragePort for MinimalPort {
    fn open(&mut self, _path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn close(&mut self) {}
    fn tables(&self) -> Vec<TableDef> {
        Vec::new()
    }
    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn read_rows(&self, _table: &str, _range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        Ok(Vec::new())
    }
    fn insert_row(&mut self, _table: &str, _values: &[FieldValue]) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn update_row(
        &mut self,
        _table: &str,
        _row: usize,
        _values: &[FieldValue],
    ) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn delete_row(&mut self, _table: &str, _row: usize) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "minimal".to_owned(),
        })
    }
    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(Vec::new())
    }
}

fn column(field: &str) -> NativeQueryColumn {
    NativeQueryColumn {
        table: Some("Customers".to_owned()),
        field: Some(field.to_owned()),
        expression: None,
        descending: false,
    }
}

fn query() -> NativeQuery {
    NativeQuery {
        name: "CustomerList".to_owned(),
        kind: NativeQueryKind::Select,
        tables: vec!["Customers".to_owned()],
        columns: vec![column("name"), column("id")],
        criteria: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
    }
}

#[test]
fn minimal_ports_get_the_documented_defaults() {
    let mut port = MinimalPort;
    assert!(port.native_queries().expect("default").is_empty());
    assert!(port.macros().expect("default").is_empty());
    assert!(port.vba_project().expect("default").is_none());
    let error = port
        .alter_table(
            "T",
            SchemaChange::DropField {
                field: "x".to_owned(),
            },
        )
        .expect_err("default is a precise Unsupported");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(error.to_string().contains("not supported"), "{error}");
}

#[test]
fn the_native_query_mapping_projects_columns_and_sort() {
    let mut query = query();
    query.columns[1].descending = true;
    let def = query.query_def().expect("mappable subset");
    assert_eq!(def.name, "CustomerList");
    assert_eq!(def.table, "Customers");
    assert_eq!(def.fields[0].name, "name");
    assert_eq!(def.fields[0].sort, SortOrder::Ascending);
    assert_eq!(def.fields[1].sort, SortOrder::Descending);
}

#[test]
fn every_disqualifier_answers_none() {
    let mut non_select = query();
    non_select.kind = NativeQueryKind::Update;
    assert!(non_select.query_def().is_none());

    let mut two_tables = query();
    two_tables.tables.push("Orders".to_owned());
    assert!(two_tables.query_def().is_none());

    let mut with_criteria = query();
    with_criteria.criteria.push("id > 1".to_owned());
    assert!(with_criteria.query_def().is_none());

    let mut with_join = query();
    with_join.joins.push("INNER JOIN".to_owned());
    assert!(with_join.query_def().is_none());

    let mut computed = query();
    computed.columns.push(NativeQueryColumn {
        table: None,
        field: None,
        expression: Some("name & id".to_owned()),
        descending: false,
    });
    assert!(computed.query_def().is_none());

    let mut unnamed = query();
    unnamed.columns.push(NativeQueryColumn {
        table: None,
        field: None,
        expression: None,
        descending: false,
    });
    assert!(unnamed.query_def().is_none());

    let no_tables = NativeQuery {
        name: "Empty".to_owned(),
        kind: NativeQueryKind::Select,
        tables: Vec::new(),
        columns: Vec::new(),
        criteria: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
    };
    assert!(no_tables.query_def().is_none());
}
