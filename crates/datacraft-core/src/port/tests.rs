use super::*;

/// A minimal port that relies on every trait-default method.
struct DefaultPort;

impl StoragePort for DefaultPort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("default")
    }
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities::full()
    }
    fn close(&mut self) {}
    fn open(
        &mut self,
        _path: &std::path::Path,
        _password: Option<&str>,
    ) -> Result<OpenedDatabase, PortError> {
        Err(PortError::OpenFailed {
            detail: "the default port opens nothing".to_owned(),
        })
    }
    fn tables(&self) -> Vec<TableDef> {
        Vec::new()
    }
    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "not supported".to_owned(),
        })
    }
    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(PortError::Unsupported {
            reason: "not supported".to_owned(),
        })
    }
    fn read_rows(&self, _table: &str, _range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        Ok(Vec::new())
    }
    fn insert_row(&mut self, _table: &str, _values: &[FieldValue]) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "not supported".to_owned(),
        })
    }
    fn delete_row(&mut self, _table: &str, _row: usize) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "not supported".to_owned(),
        })
    }
    fn update_row(
        &mut self,
        _table: &str,
        _row: usize,
        _values: &[FieldValue],
    ) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "not supported".to_owned(),
        })
    }
    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(Vec::new())
    }
}

/// A single-purpose port without overrides reports no fixed family
/// and an empty inventory.
#[test]
fn default_source_format_and_inventory_are_empty() {
    let port = DefaultPort;
    assert_eq!(port.source_format(), None);
    assert!(port.driver_inventory().is_empty());
}

/// The table-rename default answers a precise Unsupported — backends
/// with a catalog rewrite override it (phase 72, M81).
#[test]
fn table_rename_defaults_to_unsupported() {
    let mut port = DefaultPort;
    let reason = port.rename_table("T", "U").expect_err("default refuses");
    assert!(reason.to_string().contains("table renames"), "{reason}");
}

/// The relationship write defaults answer a precise Unsupported —
/// backends with a write path override them.
#[test]
fn relationship_writes_default_to_unsupported() {
    let mut port = DefaultPort;
    let relationship = Relationship {
        table: "a".to_owned(),
        column: "id".to_owned(),
        related_table: "b".to_owned(),
        related_column: "id".to_owned(),
    };
    let reason = port
        .create_relationship(&relationship)
        .expect_err("default refuses");
    assert!(
        reason.to_string().contains("creating relationships"),
        "{reason}"
    );
    let reason = port
        .delete_relationship(&relationship)
        .expect_err("default refuses");
    assert!(
        reason.to_string().contains("deleting relationships"),
        "{reason}"
    );
}
