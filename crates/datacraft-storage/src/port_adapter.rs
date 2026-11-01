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

use datacraft_core::port::{DatabaseFormat, OpenedDatabase, PortError, StoragePort};
use datacraft_core::schema::{AutoKind, BinaryKind, FieldDef, FieldKind, NumericKind, TableDef};

use crate::error::StorageError;
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, flags};
use crate::jet::{JetType, OBJECT_TYPE_TABLE};
use crate::types::FormatVersion;

/// The Jet4 read backend as a [`StoragePort`] (read-only).
#[derive(Debug, Default)]
pub struct Jet4Port {
    reader: Option<Jet4TableReader>,
    tables: Vec<TableDef>,
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
            FormatVersion::Jet4 => DatabaseFormat::Jet4,
            FormatVersion::Ace => DatabaseFormat::Ace,
            // The reader rejects Jet3 itself (BACKLOG L1).
            FormatVersion::Jet3 => {
                return Err(PortError::Unsupported {
                    reason: "Jet3 (Access 97) databases are not supported yet".to_owned(),
                });
            }
        };
        let tables = self.collect_tables(&reader)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.reader = Some(reader);
        self.tables = tables;
        self.path = Some(path.to_path_buf());
        Ok(OpenedDatabase { name, format })
    }

    fn close(&mut self) {
        self.reader = None;
        self.tables.clear();
        self.path = None;
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(read_only())
    }

    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(read_only())
    }
}

impl Jet4Port {
    /// Enumerates user tables through the catalog and maps their TDEF
    /// descriptors onto domain definitions.
    fn collect_tables(&self, reader: &Jet4TableReader) -> Result<Vec<TableDef>, PortError> {
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
            tables.push(table_def(&entry.name, &def));
        }
        Ok(tables)
    }
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
        JetType::Binary | JetType::Ole => FieldKind::Binary(BinaryKind::OleObject),
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

fn read_only() -> PortError {
    PortError::Unsupported {
        reason: "the Jet4 backend is read-only — the writer workstream is BACKLOG H9".to_owned(),
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
