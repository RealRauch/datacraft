//! Object ACL rows for writer-created tables (step 49.2, item M63).
//!
//! The engine does not only resolve a table through the catalog — it also
//! checks the object's access-control entries before reading or writing it.
//! A catalogued table without ACE rows answers "Keine Leseberechtigung"
//! (acceptance finding 2026-11-07), so the writer grants the new object the
//! same default the engine's own `CREATE TABLE` emits: one full-control entry
//! for the database owner and one for the workgroup's user SID.
//!
//! Observed engine output (DAO probe batch, 2026-11-07):
//!
//! - The new catalog row carries the database object's owner SID (`MSysDb`).
//! - The ACE rows use the owner SID and the SID the engine keeps on `MSysDb`
//!   for the workgroup (`0xDA13` in the engine template), both with a
//!   full-control mask.
//! - The SIDs are per-file (template `0xDB13`/`0xDA13`, `testV2000.mdb`
//!   `0xE5A3`/`0xE4A3`), so the writer derives them from the file's own
//!   catalog and ACL instead of hardcoding them.

use crate::error::StorageError;
use crate::jet::catalog::CATALOG_TDEF_PAGE;
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::JetTableDef;
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

/// Owner of the database object (`MSysDb`) — the engine's owner SID source.
const DATABASE_OBJECT_ID: i64 = 0x1000_0000;
/// Full-control mask the engine writes for a new object's owner.
const OWNER_MASK: i64 = 983_294;
/// Full-control mask the engine writes for the workgroup SID.
const WORKGROUP_MASK: i64 = 1_048_319;

/// The ACL rows a new object receives, plus the owner SID its catalog row
/// must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjectAcl {
    /// SID of the database owner.
    pub owner_sid: Vec<u8>,
    /// `(SID, permissions mask)` rows to insert into `MSysACEs`.
    pub grants: Vec<(Vec<u8>, i64)>,
    /// TDEF page of `MSysACEs`.
    pub aces_page: u32,
}

impl Jet4Writer {
    /// Derives the new object's ACL from the file's catalog and ACL table.
    /// Files without a readable `MSysACEs` answer `None` (nothing to grant).
    pub(crate) fn object_acl(&self) -> Result<Option<ObjectAcl>, StorageError> {
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        let catalog = reader.catalog()?;
        let Some(aces_page) = catalog
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("MSysACEs"))
            .and_then(|entry| entry.tdef_page)
        else {
            return Ok(None);
        };
        let catalog_def = JetTableDef::decode(&self.file, CATALOG_TDEF_PAGE)?;
        let Some(owner_sid) = Self::database_owner(&reader, &catalog_def)? else {
            return Ok(None);
        };
        let workgroup = self.workgroup_sid(&reader, aces_page, &owner_sid)?;
        let mut grants = vec![(owner_sid.clone(), OWNER_MASK)];
        if let Some(sid) = workgroup {
            grants.push((sid, WORKGROUP_MASK));
        }
        Ok(Some(ObjectAcl {
            owner_sid,
            grants,
            aces_page,
        }))
    }

    /// The `Owner` binary of the `MSysDb` catalog row.
    fn database_owner(
        reader: &Jet4TableReader,
        catalog_def: &JetTableDef,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let rows = reader.read_table(CATALOG_TDEF_PAGE)?.rows;
        Ok(database_owner_in(catalog_def, &rows))
    }

    /// The workgroup SID the engine keeps on the database object: an ACE SID
    /// on `MSysDb` that differs from the owner.
    fn workgroup_sid(
        &self,
        reader: &Jet4TableReader,
        aces_page: u32,
        owner: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let def = JetTableDef::decode(&self.file, aces_page)?;
        let rows = reader.read_table(aces_page)?.rows;
        Ok(workgroup_sid_in(&def, &rows, owner))
    }

    /// Inserts the object's ACL rows into `MSysACEs`.
    pub(crate) fn grant_object_acl(
        &mut self,
        object_id: u32,
        acl: &ObjectAcl,
    ) -> Result<(), StorageError> {
        let def = JetTableDef::decode(&self.file, acl.aces_page)?;
        for (sid, mask) in &acl.grants {
            let values: Vec<StorageValue> = def
                .columns
                .iter()
                .map(|column| match column.name.as_str() {
                    "ObjectId" => StorageValue::Int(i64::from(object_id)),
                    "SID" => StorageValue::Binary(sid.clone()),
                    "ACM" => StorageValue::Int(*mask),
                    "FInheritable" => StorageValue::Bit(false),
                    _ => StorageValue::Null,
                })
                .collect();
            self.insert_row(acl.aces_page, &values)?;
        }
        Ok(())
    }
}

/// The `Owner` binary of the `MSysDb` catalog row: the first row whose `Id`
/// is the database object and whose `Owner` is a non-empty binary. `None`
/// when the columns or the row are missing (no ACL is derived then).
fn database_owner_in(catalog_def: &JetTableDef, rows: &[Vec<StorageValue>]) -> Option<Vec<u8>> {
    let column = |name: &str| {
        catalog_def
            .columns
            .iter()
            .position(|column| column.name == name)
    };
    let (Some(id), Some(owner)) = (column("Id"), column("Owner")) else {
        return None;
    };
    for row in rows {
        if row.get(id) != Some(&StorageValue::Int(DATABASE_OBJECT_ID)) {
            continue;
        }
        if let Some(StorageValue::Binary(sid)) = row.get(owner)
            && !sid.is_empty()
        {
            return Some(sid.clone());
        }
    }
    None
}

/// The workgroup SID the engine keeps on the database object: the first ACE
/// SID on `MSysDb` that differs from the owner. `None` when the columns are
/// missing or no differing SID exists (the grant list then carries the owner
/// entry only).
fn workgroup_sid_in(
    aces_def: &JetTableDef,
    rows: &[Vec<StorageValue>],
    owner: &[u8],
) -> Option<Vec<u8>> {
    let index = |name: &str| {
        aces_def
            .columns
            .iter()
            .position(|column| column.name == name)
    };
    let (Some(object), Some(sid)) = (index("ObjectId"), index("SID")) else {
        return None;
    };
    for row in rows {
        if row.get(object) != Some(&StorageValue::Int(DATABASE_OBJECT_ID)) {
            continue;
        }
        if let Some(StorageValue::Binary(candidate)) = row.get(sid)
            && !candidate.is_empty()
            && candidate != owner
        {
            return Some(candidate.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests;
