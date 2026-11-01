//! `StoragePort` adapter (item H16): the Jet4 read backend behind the
//! core-owned port trait (dependency direction `storage → core`).
//!
//! Mapping decisions (documented in `docs/FORMAT-JET.md` §"Domain mapping"):
//!
//! - System tables are excluded by the `MSys` name prefix — the observed
//!   writer-stable convention (their `Flags` values vary per writer:
//!   `0x80000000`, `0x2`, `0xa`).
//! - Plain GUID columns (Jet `0x0F`) map to `FieldKind::Guid` (H12
//!   decision); GUID AutoNumbers map to `AutoNumber(ReplicationId)`.
//! - Text length comes from the descriptor: uncompressed (UTF-16) columns
//!   carry byte length, so the character budget is `len / 2`; compressed
//!   columns are byte-per-char.
//! - Jet `Numeric` descriptors carry only the scale; precision is modeled
//!   as the 96-bit maximum (28).
//! - Writes are `Unsupported` until the writer workstream (BACKLOG H9).

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, OpenedDatabase, PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};

use crate::error::StorageError;
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, flags};
use crate::jet::writer::Jet4Writer;
use crate::jet::{JetType, OBJECT_TYPE_TABLE};
use crate::port_write;
use crate::types::FormatVersion;

/// The Jet4 read backend as a [`StoragePort`] (read-only).
#[derive(Debug, Default)]
pub struct Jet4Port {
    reader: Option<Jet4TableReader>,
    tables: Vec<TableDef>,
    /// TDEF page of every catalogued table (the writer's identity).
    pages: Vec<(String, u32)>,
    path: Option<PathBuf>,
}

impl Jet4Port {
    /// Creates an unattached adapter.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StoragePort for Jet4Port {
    fn open(&mut self, path: &Path) -> Result<OpenedDatabase, PortError> {
        let bytes = std::fs::read(path).map_err(|e| PortError::OpenFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let reader = Jet4TableReader::new(bytes).map_err(map_open)?;
        let format = match reader.header().format {
            FormatVersion::Jet3 => DatabaseFormat::Jet3,
            FormatVersion::Jet4 => DatabaseFormat::Jet4,
            FormatVersion::Ace => DatabaseFormat::Ace,
        };
        let entries = collect_tables(&reader)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.reader = Some(reader);
        self.path = Some(path.to_path_buf());
        Ok(OpenedDatabase { name, format })
    }

    fn close(&mut self) {
        self.reader = None;
        self.tables.clear();
        self.pages.clear();
        self.path = None;
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        let columns = port_write::table_to_columns(definition)?;
        let name = definition.name.clone();
        self.with_writer(|writer| writer.create_table(&name, &columns))?;
        Ok(())
    }

    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        Err(PortError::Unsupported {
            reason: "dropping Jet tables lands with the navigation-pane delete flow                      (step 11.4)"
                .to_owned(),
        })
    }

    fn relationships(&self) -> Result<Vec<datacraft_core::port::Relationship>, PortError> {
        let reader = self.reader.as_ref().ok_or_else(|| PortError::Unsupported {
            reason: "no database is open".to_owned(),
        })?;
        let data = match reader.open_table("MSysRelationships") {
            Ok(data) => data,
            // No relationships table: the database has none.
            Err(crate::error::StorageError::TableNotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(map_open(e)),
        };
        let index_of = |name: &str| {
            data.schema
                .columns
                .iter()
                .position(|c| c.name == name)
                .ok_or_else(|| PortError::Unsupported {
                    reason: format!("MSysRelationships lacks the {name} column"),
                })
        };
        let (i_obj, i_col, i_ref_obj, i_ref_col) = (
            index_of("szObject")?,
            index_of("szColumn")?,
            index_of("szReferencedObject")?,
            index_of("szReferencedColumn")?,
        );
        let text = |v: &crate::types::StorageValue| match v {
            crate::types::StorageValue::Text(s) => Some(s.clone()),
            _ => None,
        };
        Ok(data
            .rows
            .iter()
            .filter_map(|row| {
                Some(datacraft_core::port::Relationship {
                    table: text(&row[i_obj])?,
                    column: text(&row[i_col])?,
                    related_table: text(&row[i_ref_obj])?,
                    related_column: text(&row[i_ref_col])?,
                })
            })
            .collect())
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let Some(page) = self
            .pages
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(table))
            .map(|(_, page)| *page)
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
        let Some(def) = self
            .tables
            .iter()
            .find(|def| def.name.eq_ignore_ascii_case(table))
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
        let stored = port_write::to_storage_values(&def.fields, values)?;
        self.with_writer(|writer| writer.insert_row(page, &stored))?;
        Ok(())
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        let reader = self.reader.as_ref().ok_or_else(|| PortError::Unsupported {
            reason: "no database is open".to_owned(),
        })?;
        let data = reader.open_table(table).map_err(|e| match e {
            crate::error::StorageError::TableNotFound(name) => PortError::TableNotFound(name),
            other => map_open(other),
        })?;
        let (start, end) = datacraft_core::port::window_slice(table, range, data.rows.len())?;
        Ok(data.rows[start..end]
            .iter()
            .map(|row| row.iter().map(to_field_value).collect())
            .collect())
    }
}

/// Maps one stored value onto the domain vocabulary (1:1 — the variants
/// correspond, see docs/FORMAT-JET.md "Domain mapping").
fn to_field_value(value: &crate::types::StorageValue) -> FieldValue {
    match value {
        crate::types::StorageValue::Null => FieldValue::Null,
        crate::types::StorageValue::Bit(b) => FieldValue::Boolean(*b),
        crate::types::StorageValue::Int(i) => FieldValue::Integer(*i),
        crate::types::StorageValue::Float(f) => FieldValue::Float(*f),
        crate::types::StorageValue::Currency(c) => FieldValue::Currency(*c),
        crate::types::StorageValue::DateTime(d) => FieldValue::DateTime(*d),
        crate::types::StorageValue::Text(s) => FieldValue::Text(s.clone()),
        crate::types::StorageValue::Binary(b) => FieldValue::Binary(b.clone()),
        crate::types::StorageValue::Guid(g) => FieldValue::Guid(*g),
        crate::types::StorageValue::Decimal(d) => FieldValue::Decimal(*d),
    }
}

impl Jet4Port {
    /// Runs one writer operation on a private copy of the file, persists the
    /// result and swaps in the refreshed reader/catalog. A failed operation
    /// leaves the file and the in-memory state untouched.
    fn with_writer<T>(
        &mut self,
        op: impl FnOnce(&mut Jet4Writer) -> Result<T, StorageError>,
    ) -> Result<T, PortError> {
        let path = self.path.clone().ok_or_else(no_database)?;
        let bytes = self.reader.as_ref().ok_or_else(no_database)?.file.clone();
        let mut writer = Jet4Writer::new(bytes).map_err(map_write)?;
        let out = op(&mut writer).map_err(map_write)?;
        let bytes = writer.into_file();
        std::fs::write(&path, &bytes).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let reader = Jet4TableReader::new(bytes).map_err(map_open)?;
        let entries = collect_tables(&reader)?;
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.reader = Some(reader);
        Ok(out)
    }
}

/// Enumerates user tables through the catalog with their TDEF pages.
fn collect_tables(reader: &Jet4TableReader) -> Result<Vec<(TableDef, u32)>, PortError> {
    let mut tables = Vec::new();
    for entry in reader.catalog().map_err(map_open)? {
        if entry.object_type != OBJECT_TYPE_TABLE {
            continue;
        }
        let Some(tdef_page) = entry.tdef_page else {
            continue;
        };
        if entry.name.starts_with("MSys") {
            continue;
        }
        let def = JetTableDef::decode(&reader.file, tdef_page).map_err(map_open)?;
        tables.push((table_def(&entry.name, &def), tdef_page));
    }
    Ok(tables)
}

/// Maps one table's descriptors onto a validated domain definition.
fn table_def(name: &str, def: &JetTableDef) -> TableDef {
    TableDef::new(name, def.columns.iter().map(field_def).collect())
}

/// Maps one column descriptor onto a field definition.
fn field_def(column: &JetColumn) -> FieldDef {
    let nullable = column.flags & flags::NULLABLE != 0;
    let autonumber = column.flags & flags::AUTONUMBER != 0;
    let kind = match column.data_type {
        JetType::Bool => FieldKind::Boolean {
            tri_state: nullable,
        },
        JetType::Byte => FieldKind::Number(NumericKind::Byte),
        JetType::Int => FieldKind::Number(NumericKind::Integer),
        JetType::LongInt if autonumber => FieldKind::AutoNumber(AutoKind::LongInteger),
        JetType::LongInt => FieldKind::Number(NumericKind::LongInteger),
        JetType::Currency => FieldKind::Currency,
        JetType::Float => FieldKind::Number(NumericKind::Single),
        JetType::Double => FieldKind::Number(NumericKind::Double),
        JetType::Numeric => FieldKind::Number(NumericKind::Decimal {
            precision: 28,
            scale: column.scale,
        }),
        JetType::DateTime => FieldKind::DateTime,
        JetType::Binary | JetType::Ole | JetType::LongBinary => {
            FieldKind::Binary(BinaryKind::OleObject)
        }
        JetType::Text => FieldKind::Text {
            max_len: text_max_len(column),
        },
        JetType::Memo => FieldKind::Memo,
        JetType::Guid if autonumber => FieldKind::AutoNumber(AutoKind::ReplicationId),
        JetType::Guid => FieldKind::Guid,
        JetType::Complex => FieldKind::Binary(BinaryKind::Attachment),
    };
    let mut field = FieldDef::new(&column.name, kind);
    field.properties.required = !nullable;
    field
}

/// Descriptor length -> Access character budget (see module docs).
fn text_max_len(column: &JetColumn) -> u16 {
    if column.compressed_text {
        column.len.max(1)
    } else {
        (column.len / 2).max(1)
    }
}

fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}

fn map_write(error: StorageError) -> PortError {
    match error {
        StorageError::Unsupported { reason } => PortError::Unsupported { reason },
        other => PortError::WriteFailed {
            detail: other.to_string(),
        },
    }
}

fn map_open(error: StorageError) -> PortError {
    match error {
        StorageError::Unsupported { reason } => PortError::Unsupported { reason },
        other => PortError::OpenFailed {
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StorageValue;

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
}
