//! Mock builders and lookup helpers (split from `port_mock.rs` for the
//! file-size gate in phase 73).

use super::*;

impl MockPort {
    /// Creates an empty mock that opens as `"mock"` (Jet4).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a mock pre-seeded with table definitions.
    pub fn with_tables(tables: Vec<TableDef>) -> Self {
        Self {
            tables,
            ..Self::default()
        }
    }

    /// Builder: attaches row data for a table (case-insensitive reads).
    pub fn with_row_data(
        &mut self,
        table: impl Into<String>,
        rows: Vec<Vec<FieldValue>>,
    ) -> &mut Self {
        self.rows.insert(table.into(), rows);
        self
    }

    /// Builder: attaches referential relationships.
    pub fn with_relationships(&mut self, relationships: Vec<Relationship>) -> &mut Self {
        self.relationships = relationships;
        self
    }

    /// Builder: attaches the named-macro inventory (step 17.1, L4).
    pub fn with_macros(&mut self, macros: Vec<crate::macro_def::MacroDef>) -> &mut Self {
        self.macros = macros;
        self
    }

    /// Builder: attaches an extracted VBA project (step 17.2, L5).
    pub fn with_vba(&mut self, project: crate::vba::VbaProject) -> &mut Self {
        self.vba = Some(project);
        self
    }

    /// Builder: makes the VBA extraction fail with `reason` (observable
    /// degradation; the database still opens).
    pub fn with_vba_error(&mut self, reason: impl Into<String>) -> &mut Self {
        self.vba_error = Some(reason.into());
        self
    }

    /// Builder: makes the native-query decode fail with `reason` (M10).
    pub fn with_native_queries_error(&mut self, reason: impl Into<String>) -> &mut Self {
        self.native_queries_error = Some(reason.into());
        self
    }

    /// Builder: attaches a native query inventory (M3).
    pub fn with_native_queries(&mut self, queries: Vec<crate::port::NativeQuery>) -> &mut Self {
        self.native_queries = queries;
        self
    }

    /// Builder: attaches file-owned views (phase 73, step 73.1; M90).
    pub fn with_indexes(&mut self, indexes: Vec<crate::port::SourceIndex>) -> &mut Self {
        self.indexes = indexes;
        self
    }

    /// Builder: attaches the read-only trigger inventory (M90).
    pub fn with_triggers(&mut self, triggers: Vec<crate::port::SourceTrigger>) -> &mut Self {
        self.triggers = triggers;
        self
    }

    /// Builder: attaches driver facts and the health verdict (M91).
    pub fn with_facts(&mut self, facts: crate::port::SourceFacts) -> &mut Self {
        self.source_facts = facts;
        self
    }

    /// Builder: attaches file-owned views (phase 73, step 73.1; M90).
    pub fn with_native_views(&mut self, views: Vec<crate::port::NativeView>) -> &mut Self {
        self.view_defs = views
            .iter()
            .map(|view| TableDef::new(&view.name, view.fields.clone()))
            .collect();
        self.native_views = views;
        self
    }

    /// Creates a mock whose `open` always fails with `detail`.
    pub fn failing_open(detail: impl Into<String>) -> Self {
        Self {
            open_fail: Some(detail.into()),
            ..Self::default()
        }
    }

    /// Creates a mock whose write operations always fail with `detail`.
    pub fn failing_writes(detail: impl Into<String>) -> Self {
        Self {
            write_fail: Some(detail.into()),
            ..Self::default()
        }
    }

    /// Builder: makes every subsequent write fail with `detail`.
    pub fn with_write_failure(&mut self, detail: impl Into<String>) -> &mut Self {
        self.write_fail = Some(detail.into());
        self
    }

    /// Builder: makes every subsequent row read fail with `detail`
    /// (materialization-failure tests).
    pub fn with_read_failure(&mut self, detail: impl Into<String>) -> &mut Self {
        self.read_fail = Some(detail.into());
        self
    }

    /// Builder: requires `password` on `open` (database-password flow, M5).
    pub fn with_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.password = Some(password.into());
        self
    }

    /// Builder: overrides the reported capabilities (phase 30, M17 —
    /// simulated restricted drivers).
    pub fn with_capabilities(&mut self, capabilities: DriverCapabilities) -> &mut Self {
        self.reported_capabilities = Some(capabilities);
        self
    }

    /// Builder: `open` reports a detected Access lock file (step 53.2,
    /// item M59 — the name travels as data; combine with
    /// `with_capabilities` for the read-only shape).
    pub fn with_lock_file(&mut self, name: impl Into<String>) -> &mut Self {
        self.lock_file = Some(name.into());
        self
    }

    /// Resolves a table definition case-insensitively.
    pub(super) fn table_def(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .chain(self.view_defs.iter())
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Precise write rejection for a file-owned view (M90).
    pub(super) fn reject_view(&self, table: &str) -> Result<(), PortError> {
        if self
            .native_views
            .iter()
            .any(|view| view.name.eq_ignore_ascii_case(table))
        {
            return Err(PortError::Unsupported {
                reason: format!("{table:?} is a native view and read-only"),
            });
        }
        Ok(())
    }
}
