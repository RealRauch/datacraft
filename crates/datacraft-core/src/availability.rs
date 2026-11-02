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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

/// Stable affordance ids shared by the ribbon spec, the navigation
/// pane's context menu and the datasheet's new-record row.
pub mod ids {
    /// Ribbon Create ▸ Tables (the new-table flow).
    pub const CREATE_TABLE: &str = "table";
    /// Ribbon Database Tools ▸ Relationships (the mapper pane).
    pub const RELATIONSHIPS: &str = "relationships";
    /// Navigation-pane context menu Delete.
    pub const NAV_DELETE: &str = "nav.delete";
    /// Datasheet new-record row.
    pub const NEW_RECORD: &str = "datasheet.new_record";
    /// Backstage Save ▸ Save a copy (step 27.2, M14).
    pub const SAVE_AS: &str = "backstage.save_as";
    /// Backstage Save ▸ Change password (step 27.2, M14).
    pub const CHANGE_PASSWORD: &str = "backstage.change_password";
}

/// The availability map for one open source: disabled affordance id →
/// reason. Only restricted affordances appear.
pub fn affordances(caps: &DriverCapabilities) -> BTreeMap<String, DisabledReason> {
    let mut map = BTreeMap::new();
    if !caps.create_table {
        map.insert(ids::CREATE_TABLE.to_owned(), DisabledReason::CreateTables);
    }
    if !caps.drop_table {
        map.insert(ids::NAV_DELETE.to_owned(), DisabledReason::DropTables);
    }
    if !caps.write_rows {
        map.insert(ids::NEW_RECORD.to_owned(), DisabledReason::WriteRows);
    }
    if caps.relationships == Support::No {
        map.insert(
            ids::RELATIONSHIPS.to_owned(),
            DisabledReason::NoRelationships,
        );
    }
    if !caps.passwords {
        map.insert(ids::SAVE_AS.to_owned(), DisabledReason::NoPasswords);
        map.insert(ids::CHANGE_PASSWORD.to_owned(), DisabledReason::NoPasswords);
    }
    map
}
