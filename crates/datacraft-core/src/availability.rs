//! Command affordance availability (phase 30, item M17; ADR-7).
//!
//! The core derives, per open source, which command affordances the UI
//! must render disabled — computed from the driver's declared
//! capabilities, never from format matches in UI code. The map lists
//! only *restricted* affordances: absence means enabled, so capable
//! sources change nothing and existing pins stay stable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::port::{DriverCapabilities, Support};

/// Stable key naming why an affordance is unavailable for the open
/// source. Locale-neutral on the wire (ADR-4): the UI resolves it
/// through its catalog; agents read the snake_case spelling.
///
/// `#[non_exhaustive]` (M32): every new capability affordance adds a
/// variant; clients match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DisabledReason {
    /// The driver cannot create tables for this source.
    CreateTables,
    /// The driver cannot drop tables for this source.
    DropTables,
    /// The driver cannot write rows for this source (read-only session).
    WriteRows,
    /// The source exposes no relationships.
    NoRelationships,
    /// The source has no database-password surface (save-as / password
    /// change are unavailable).
    NoPasswords,
    /// The driver cannot alter the schema in place (the design view's
    /// edit controls).
    AlterTables,
    /// The composition cannot create new database files.
    CreateDatabase,
}

/// Stable affordance ids shared by the ribbon spec, the navigation
/// pane's context menu and the datasheet's new-record row.
pub mod ids {
    /// Ribbon Create ▸ Tables (the new-table flow).
    pub const CREATE_TABLE: &str = "table";
    /// Ribbon External Data ▸ Import File (step 58.1, item M50). The
    /// chain creates the table and writes its rows.
    pub const IMPORT_FILE: &str = "import_file";
    /// Ribbon Database Tools ▸ Relationships (the mapper pane).
    pub const RELATIONSHIPS: &str = "relationships";
    /// Relationship-mapper create affordance (step 41.1, item M45).
    /// Gates on `Support::Yes` — the dispatch gate refuses anything less.
    pub const RELATIONSHIPS_CREATE: &str = "relationships.create";
    /// Relationship-mapper per-row delete affordance (step 41.1, item M45).
    pub const RELATIONSHIPS_DELETE: &str = "relationships.delete";
    /// Navigation-pane context menu Delete.
    pub const NAV_DELETE: &str = "nav.delete";
    /// Datasheet new-record row.
    pub const NEW_RECORD: &str = "datasheet.new_record";
    /// Datasheet record deletion (row context menu / Del key, step 40.3,
    /// item M44).
    pub const DELETE_RECORD: &str = "datasheet.delete_record";
    /// Datasheet in-place cell editing (step 40.4, item L23).
    pub const EDIT_RECORD: &str = "datasheet.edit_record";
    /// Backstage Save ▸ Save a copy (step 27.2, M14).
    pub const SAVE_AS: &str = "backstage.save_as";
    /// Backstage Save ▸ Change password (step 27.2, M14).
    pub const CHANGE_PASSWORD: &str = "backstage.change_password";
    /// Backstage New ▸ Create Database (step 42.2, item L18).
    pub const CREATE_DATABASE: &str = "backstage.create_database";
    /// Design view ▸ Add Field (step 42.2, item L18). The four design
    /// ids share the `alter_table` capability.
    pub const DESIGN_ADD_FIELD: &str = "design.add_field";
    /// Design view ▸ Rename (step 42.2, item L18).
    pub const DESIGN_RENAME_FIELD: &str = "design.rename_field";
    /// Design view ▸ Delete field (step 42.2, item L18).
    pub const DESIGN_DROP_FIELD: &str = "design.drop_field";
    /// Design view ▸ Required toggle (step 42.2, item L18).
    pub const DESIGN_SET_FIELD_PROPERTIES: &str = "design.set_field_properties";
}

/// The availability map for one open source: disabled affordance id →
/// reason. Only restricted affordances appear.
pub fn affordances(caps: &DriverCapabilities) -> BTreeMap<String, DisabledReason> {
    let mut map = BTreeMap::new();
    if !caps.create_table {
        map.insert(ids::CREATE_TABLE.to_owned(), DisabledReason::CreateTables);
    }
    if !caps.create_table {
        map.insert(ids::IMPORT_FILE.to_owned(), DisabledReason::CreateTables);
    } else if !caps.write_rows {
        map.insert(ids::IMPORT_FILE.to_owned(), DisabledReason::WriteRows);
    }
    if !caps.drop_table {
        map.insert(ids::NAV_DELETE.to_owned(), DisabledReason::DropTables);
    }
    if !caps.write_rows {
        map.insert(ids::NEW_RECORD.to_owned(), DisabledReason::WriteRows);
        map.insert(ids::DELETE_RECORD.to_owned(), DisabledReason::WriteRows);
        map.insert(ids::EDIT_RECORD.to_owned(), DisabledReason::WriteRows);
    }
    if caps.relationships == Support::No {
        map.insert(
            ids::RELATIONSHIPS.to_owned(),
            DisabledReason::NoRelationships,
        );
    }
    if caps.relationships != Support::Yes {
        map.insert(
            ids::RELATIONSHIPS_CREATE.to_owned(),
            DisabledReason::NoRelationships,
        );
        map.insert(
            ids::RELATIONSHIPS_DELETE.to_owned(),
            DisabledReason::NoRelationships,
        );
    }
    if !caps.passwords {
        map.insert(ids::SAVE_AS.to_owned(), DisabledReason::NoPasswords);
        map.insert(ids::CHANGE_PASSWORD.to_owned(), DisabledReason::NoPasswords);
    }
    if !caps.alter_table {
        map.insert(
            ids::DESIGN_ADD_FIELD.to_owned(),
            DisabledReason::AlterTables,
        );
        map.insert(
            ids::DESIGN_RENAME_FIELD.to_owned(),
            DisabledReason::AlterTables,
        );
        map.insert(
            ids::DESIGN_DROP_FIELD.to_owned(),
            DisabledReason::AlterTables,
        );
        map.insert(
            ids::DESIGN_SET_FIELD_PROPERTIES.to_owned(),
            DisabledReason::AlterTables,
        );
    }
    if !caps.create_database {
        map.insert(
            ids::CREATE_DATABASE.to_owned(),
            DisabledReason::CreateDatabase,
        );
    }
    map
}
