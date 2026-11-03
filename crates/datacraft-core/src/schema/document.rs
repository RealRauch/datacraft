//! Versioned schema document (step 3.2, item H11; ADR-1).
//!
//! The `.datacraft` sidecar next to a database file persists DataCraft's
//! own view of the data model — tables, fields and their properties — plus
//! (from Phase 5 on) the form/report design layer. The document carries a
//! `schema_version`; readers accept their own version, migrate older ones
//! forward via a strict ladder and reject newer ones with a precise error.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::def::TableDef;
use super::def::validate_object_name;
use super::error::SchemaError;
use super::properties::IndexMode;
use crate::qbe::QueryDef;

/// Version this DataCraft writes. Bump when the document format changes and
/// append a migration step to [`MIGRATIONS`].
pub const SCHEMA_VERSION: u32 = 1;

/// Initial document version (the ladder's floor).
const FIRST_VERSION: u32 = 1;

/// Migration ladder: `MIGRATIONS[i]` upgrades a document from version
/// `i + FIRST_VERSION` to `i + FIRST_VERSION + 1`, in place. Append-only —
/// steps never change once released.
type Migration = fn(&mut serde_json::Value);
const MIGRATIONS: &[Migration] = &[];

/// Versioned document of the DataCraft data model (ADR-1 sidecar format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDocument {
    /// Format version, written as [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Name of the database the document belongs to.
    pub database_name: String,
    /// Table definitions in navigation order.
    pub tables: Vec<TableDef>,
    /// Query definitions (additive v1 field; older documents load with
    /// an empty list via the serde default).
    #[serde(default)]
    pub queries: Vec<QueryDef>,
    /// Form design documents (additive v1 field, ADR-1).
    #[serde(default)]
    pub forms: Vec<crate::form::FormDef>,
    /// Report design documents (additive v1 field, step 10.4 / H31).
    #[serde(default)]
    pub reports: Vec<crate::report::ReportDef>,
}

impl SchemaDocument {
    /// Creates an empty, current-version document.
    pub fn new(database_name: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            database_name: database_name.into(),
            tables: Vec::new(),
            queries: Vec::new(),
            forms: Vec::new(),
            reports: Vec::new(),
        }
    }

    /// Builder: attaches table definitions.
    pub fn with_tables(mut self, tables: Vec<TableDef>) -> Self {
        self.tables = tables;
        self
    }

    /// Validates the database name, every table and table-name uniqueness
    /// (object names compare case-insensitively, like Access).
    pub fn validate(&self) -> Result<(), SchemaError> {
        validate_object_name(&self.database_name)?;
        let mut seen: Vec<String> = Vec::with_capacity(self.tables.len());
        for table in &self.tables {
            table.validate()?;
            let key = table.name.to_lowercase();
            if seen.contains(&key) {
                return Err(SchemaError::DuplicateName {
                    name: table.name.clone(),
                });
            }
            seen.push(key);
        }
        Ok(())
    }

    /// Serializes deterministically (pretty JSON, stable field order).
    ///
    /// Infallible for this document: it contains no non-string map keys or
    /// other serde failure sources, so serialization cannot fail.
    pub fn to_json(&self) -> String {
        // Unwrap is sound per the doc invariant above (infallible types).
        serde_json::to_string_pretty(self).expect("SchemaDocument serialization is infallible")
    }

    /// Parses a document: migrates older versions forward, rejects newer
    /// ones with the found version named.
    pub fn from_json(json: &str) -> Result<Self, SchemaError> {
        let mut value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| SchemaError::Parse {
                detail: e.to_string(),
            })?;
        let found = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| SchemaError::Parse {
                detail: "missing or non-numeric schema_version field".to_owned(),
            })?;
        migrate_with_ladder(MIGRATIONS, SCHEMA_VERSION, &mut value, found)?;
        serde_json::from_value(value).map_err(|e| SchemaError::Parse {
            detail: e.to_string(),
        })
    }

    /// Path of the sidecar for a database file: same stem, `.datacraft`
    /// extension, next to the database (ADR-1).
    pub fn sidecar_path(database: &Path) -> PathBuf {
        let mut path = database.to_path_buf();
        path.set_extension("datacraft");
        path
    }

    /// Applies this document's table overlays onto the live catalog
    /// (step 32.1, item H45; ADR-1 addendum): only field properties are
    /// copied, never kinds or names. A sidecar table/field that no longer
    /// exists is a precise error instead of a silent property loss; every
    /// merged table is re-validated against the live kinds.
    pub fn apply_table_properties(&self, tables: &mut [TableDef]) -> Result<(), SchemaError> {
        for source in &self.tables {
            let Some(target) = tables
                .iter_mut()
                .find(|table| table.name.eq_ignore_ascii_case(&source.name))
            else {
                return Err(SchemaError::InvalidProperty {
                    detail: format!(
                        "the sidecar table {:?} is not in the database — properties were not                          applied",
                        source.name
                    ),
                });
            };
            for source_field in &source.fields {
                let Some(target_field) = target
                    .fields
                    .iter_mut()
                    .find(|field| field.name.eq_ignore_ascii_case(&source_field.name))
                else {
                    return Err(SchemaError::InvalidProperty {
                        detail: format!(
                            "the sidecar field {:?}.{:?} is not in the database — properties were                              not applied",
                            source.name, source_field.name
                        ),
                    });
                };
                // Physical index metadata is authoritative for `indexed`
                // (step 32.2, M15); the sidecar carries the other properties
                // and may describe an index for sources without a physical
                // catalog (or pre-M15 sidecars).
                let physical_indexed = target_field.properties.indexed;
                target_field.properties = source_field.properties.clone();
                if physical_indexed != IndexMode::None {
                    target_field.properties.indexed = physical_indexed;
                }
            }
            target.validate()?;
        }
        Ok(())
    }

    /// Lenient property copy for internal catalog refreshes (step 32.1,
    /// item H45): matching table/field properties are copied, everything
    /// else is skipped. Used after `alter_table` replaced the definitions
    /// from the port's physical catalog.
    pub fn merge_field_properties(target: &mut [TableDef], source: &[TableDef]) {
        for source_table in source {
            let Some(target_table) = target
                .iter_mut()
                .find(|table| table.name.eq_ignore_ascii_case(&source_table.name))
            else {
                continue;
            };
            for source_field in &source_table.fields {
                if let Some(target_field) = target_table
                    .fields
                    .iter_mut()
                    .find(|field| field.name.eq_ignore_ascii_case(&source_field.name))
                {
                    // Physical index metadata is authoritative for `indexed`
                    // (step 32.2, M15); the sidecar carries the other properties
                    // and may describe an index for sources without a physical
                    // catalog (or pre-M15 sidecars).
                    let physical_indexed = target_field.properties.indexed;
                    target_field.properties = source_field.properties.clone();
                    if physical_indexed != IndexMode::None {
                        target_field.properties.indexed = physical_indexed;
                    }
                }
            }
        }
    }

    /// Writes the document to `path` (the sidecar file) atomically.
    pub fn save(&self, path: &Path) -> Result<(), SchemaError> {
        crate::atomic_write::write_replacing(path, self.to_json().as_bytes())?;
        Ok(())
    }

    /// Reads and parses the document from `path`.
    pub fn load(path: &Path) -> Result<Self, SchemaError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

/// Applies `ladder` to walk a document from `found` up to `target`.
/// `ladder[i]` upgrades version `i + FIRST_VERSION` to the next one; a
/// missing step is a precise version error.
fn migrate_with_ladder(
    ladder: &[Migration],
    target: u32,
    value: &mut serde_json::Value,
    found: u64,
) -> Result<(), SchemaError> {
    let supported = u64::from(target);
    if found < u64::from(FIRST_VERSION) || found > supported {
        return Err(SchemaError::UnsupportedVersion { found, supported });
    }
    let mut version = found as u32;
    while version < target {
        let step = ladder
            .get((version - FIRST_VERSION) as usize)
            .copied()
            .ok_or(SchemaError::UnsupportedVersion {
                found: u64::from(version),
                supported,
            })?;
        step(value);
        version += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real ladder is a no-op for the initial version and rejects
    /// anything below or above it.
    #[test]
    fn ladder_is_identity_for_v1_and_strict_around_it() {
        let mut value = serde_json::json!({ "schema_version": 1 });
        migrate_with_ladder(MIGRATIONS, SCHEMA_VERSION, &mut value, 1).unwrap();
        assert_eq!(value["schema_version"], 1);

        for found in [0u64, 2, 99, u64::from(u32::MAX) + 1] {
            let mut value = serde_json::json!({ "schema_version": found });
            let err = migrate_with_ladder(MIGRATIONS, SCHEMA_VERSION, &mut value, found)
                .expect_err("ladder must reject versions without a step");
            assert!(
                err.to_string().contains(&found.to_string()),
                "error must name the found version {found}: {err}"
            );
        }
    }

    fn bump(value: &mut serde_json::Value) {
        let current = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64);
        if let Some(v) = current {
            value["schema_version"] = serde_json::json!(v + 1);
        }
    }

    /// A populated ladder applies every step in order until the target.
    #[test]
    fn ladder_applies_steps_in_order_up_to_the_target() {
        let ladder: &[Migration] = &[bump, bump];
        let mut value = serde_json::json!({ "schema_version": 1 });
        migrate_with_ladder(ladder, 3, &mut value, 1).unwrap();
        assert_eq!(value["schema_version"], 3);
    }

    /// A ladder shorter than the distance to the target is a version error
    /// naming the version where the step is missing.
    #[test]
    fn ladder_refuses_to_walk_past_its_last_step() {
        let ladder: &[Migration] = &[bump];
        let mut value = serde_json::json!({ "schema_version": 1 });
        let err = migrate_with_ladder(ladder, 3, &mut value, 1)
            .expect_err("missing step must be refused");
        let msg = err.to_string();
        assert!(msg.contains('2'), "must name the stuck version: {msg}");
        assert!(msg.contains("3"), "must name the supported target: {msg}");
    }
}
