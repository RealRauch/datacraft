//! Storage-port defaults and native-query mapping (phase 24 coverage gate;
//! identity/capability meta-tests since M22).
//!
//! The optional surfaces (`native_queries`, `macros`, `vba_project`,
//! `alter_table`, `save_as`, `change_password`, `create_database`) keep
//! their documented trait defaults, and a minimal port pins those
//! defaults. Identity and capabilities have **no** default (M22, ADR-7):
//! every port declares them explicitly — a forgetful driver does not
//! compile — and the minimal port pins the conservative declaration. The
//! native-query mapping is tested across its disqualifiers and its sort
//! projection.

use std::path::Path;

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, NativeJoin, NativeJoinKind, NativeQuery,
    NativeQueryColumn, NativeQueryKind, OpenedDatabase, PortError, Relationship, RowRange,
    SchemaChange, StoragePort, Support,
};
use datacraft_core::qbe::SortOrder;
use datacraft_core::schema::{FieldValue, TableDef};

/// Implements the required trait methods; the optional surfaces keep the
/// trait defaults, and identity/capabilities are declared explicitly.
struct MinimalPort;

impl StoragePort for MinimalPort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("minimal")
    }
    fn capabilities(&self) -> DriverCapabilities {
        // The conservative declaration (M22): this port drives nothing.
        DriverCapabilities::none()
    }
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
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
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
    with_join.joins.push(NativeJoin {
        expression: "Customers.id = Orders.customer".to_owned(),
        kind: NativeJoinKind::Inner,
    });
    assert!(with_join.query_def().is_none());

    let mut grouped = query();
    grouped.group_by.push("name".to_owned());
    assert!(grouped.query_def().is_none());

    let mut with_having = query();
    with_having.having.push("Count(*) > 1".to_owned());
    assert!(with_having.query_def().is_none());

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
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    };
    assert!(no_tables.query_def().is_none());
}

/// Every shipped driver must name itself and state its capabilities
/// explicitly — the compiler enforces it for in-tree ports (M22), and the
/// per-driver capability suites pin the exact declarations
/// (`jet_capabilities`, `sqlite_capabilities`, the csv driver test).
#[test]
fn a_port_declares_its_identity_and_capabilities_explicitly() {
    let port = MinimalPort;
    assert_eq!(port.driver_id().as_str(), "minimal");
    // The declared set is the conservative one, not an inherited default.
    assert_eq!(port.capabilities(), DriverCapabilities::none());
    // The inventory default is empty (M23): a single-purpose port names
    // itself; the detecting composite reports its registry.
    assert!(port.driver_inventory().is_empty());
}

#[test]
fn the_conservative_and_optimistic_reference_sets_are_distinct() {
    let none = DriverCapabilities::none();
    let full = DriverCapabilities::full();

    // none() refuses every driven surface…
    assert!(!none.read_tables);
    assert!(!none.write_rows);
    assert!(!none.create_database);
    assert!(!none.atomic_batch);
    assert_eq!(none.relationships, Support::No);
    assert_eq!(none.autonumber, Support::No);
    // …while full() keeps its documented optimistic shape (readable and
    // writable, but batch atomicity stays undeclared and the reserved
    // vocabulary stays No).
    assert!(full.read_tables);
    assert!(full.write_rows);
    assert!(!full.atomic_batch);
    assert_eq!(full.autonumber, Support::No);
    assert_eq!(full.relationships, Support::Yes);
    assert_ne!(none, full, "the two reference sets are distinct");
}

#[test]
fn minimal_ports_keep_the_password_defaults() {
    let mut port = MinimalPort;

    let error = port
        .save_as(Path::new("copy.mdb"), None)
        .expect_err("default");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(
        error.to_string().contains("saving database copies"),
        "{error}"
    );
    let error = port.change_password(Some("secret")).expect_err("default");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    assert!(
        error.to_string().contains("changing database passwords"),
        "{error}"
    );
    let error = port
        .create_database(Path::new("new.mdb"), DatabaseFormat::Jet4)
        .expect_err("default");
    assert!(
        error.to_string().contains("creating database files"),
        "{error}"
    );

    let required = PortError::PasswordRequired {
        path: "db.mdb".to_owned(),
    };
    assert!(
        required.to_string().contains("password-protected"),
        "{required}"
    );
    let wrong = PortError::WrongPassword {
        path: "db.mdb".to_owned(),
    };
    assert!(wrong.to_string().contains("password is wrong"), "{wrong}");
}
