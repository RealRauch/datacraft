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
//! - Writes run through the clean-room Jet4 writer behind the port (step
//!   11.1); shapes the writer does not qualify yet (AutoNumber counters,
//!   attachments, table drops) answer a precise `Unsupported`
//!   (BACKLOG M4/M7/M8).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, OpenedDatabase, PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};

use crate::error::StorageError;
use crate::jet::complex::{self, ComplexCatalog, ComplexColumn, FlatRows};
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, flags};
use crate::jet::writer::Jet4Writer;
use crate::jet::{JetType, OBJECT_TYPE_TABLE};
use crate::mapping::to_field_value;
use crate::port_errors::{map_open, map_open_with_path, map_write, no_database};
use crate::port_write;
use crate::types::FormatVersion;

/// The Jet4 backend as a [`StoragePort`] (read/write).
#[derive(Debug, Default)]
pub struct Jet4Port {
    reader: Option<Jet4TableReader>,
    tables: Vec<TableDef>,
    /// TDEF page of every catalogued table (the writer's identity).
    pages: Vec<(String, u32)>,
    /// Complex-column mapping (`MSysComplexColumns`); empty for Jet4.
    complex: ComplexCatalog,
    /// Named macros read at open (step 17.1, L4; read-only).
    macros: Vec<datacraft_core::macro_def::MacroDef>,
    path: Option<PathBuf>,
}

impl Jet4Port {
    /// Creates an unattached adapter.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StoragePort for Jet4Port {
    fn open(&mut self, path: &Path, password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        let bytes = std::fs::read(path).map_err(|e| PortError::OpenFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let reader = Jet4TableReader::new_with_password(bytes, password)
            .map_err(|error| map_open_with_path(error, path))?;
        let format = match reader.header().format {
            FormatVersion::Jet3 => DatabaseFormat::Jet3,
            FormatVersion::Jet4 => DatabaseFormat::Jet4,
            FormatVersion::Ace => DatabaseFormat::Ace,
        };
        let complex = ComplexCatalog::read(&reader).map_err(map_open)?;
        let entries = collect_tables(&reader, &complex)?;
        let macros = reader
            .macros()
            .map_err(map_open)?
            .into_iter()
            .map(|stored| datacraft_core::macro_def::MacroDef {
                name: stored.name,
                actions: stored
                    .arguments
                    .into_iter()
                    .map(|argument| datacraft_core::macro_def::MacroAction { argument })
                    .collect(),
                definition_decoded: stored.definition_decoded,
            })
            .collect();
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.complex = complex;
        self.macros = macros;
        self.reader = Some(reader);
        self.path = Some(path.to_path_buf());
        Ok(OpenedDatabase { name, format })
    }

    fn close(&mut self) {
        self.reader = None;
        self.tables.clear();
        self.pages.clear();
        self.complex = ComplexCatalog::default();
        self.macros.clear();
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
            reason: "dropping Jet tables is not implemented yet (BACKLOG M7)".to_owned(),
        })
    }

    fn native_queries(&self) -> Result<Vec<datacraft_core::port::NativeQuery>, PortError> {
        let Some(reader) = self.reader.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(crate::port_native::to_core(
            reader.native_queries().map_err(map_open)?,
        ))
    }

    fn macros(&self) -> Result<Vec<datacraft_core::macro_def::MacroDef>, PortError> {
        Ok(self.macros.clone())
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
        let page = self.page_of(table)?;
        if self.complex.is_flat_page(page) {
            // Backing tables are an implementation detail, not user tables.
            return Err(PortError::TableNotFound(table.to_owned()));
        }
        let data = reader.open_table(table).map_err(|e| match e {
            crate::error::StorageError::TableNotFound(name) => PortError::TableNotFound(name),
            other => map_open(other),
        })?;
        let def = JetTableDef::decode(&reader.file, page).map_err(map_open)?;
        let (start, end) = datacraft_core::port::window_slice(table, range, data.rows.len())?;
        let mut flats: HashMap<u32, FlatRows> = HashMap::new();
        let mut rows = Vec::with_capacity(end - start);
        for row in &data.rows[start..end] {
            let mut values = Vec::with_capacity(row.len());
            for (column, value) in def.columns.iter().zip(row) {
                let Some(mapping) = self.complex.column(page, &column.name) else {
                    values.push(to_field_value(value));
                    continue;
                };
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    flats.entry(mapping.flat_tdef)
                {
                    entry.insert(FlatRows::read(reader, mapping).map_err(map_open)?);
                }
                let flat = flats.get(&mapping.flat_tdef).expect("inserted above");
                values.push(complex::resolve_cell(value, flat).map_err(map_open)?);
            }
            rows.push(values);
        }
        Ok(rows)
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let page = self.page_of(table)?;
        let stored = {
            let definition = self.definition_of(table)?.clone();
            port_write::to_storage_values_for_update(&definition.fields, values)?
        };
        self.with_writer(|writer| writer.update_row(page, row, &stored))?;
        Ok(())
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let page = self.page_of(table)?;
        self.with_writer(|writer| writer.delete_row(page, row))?;
        Ok(())
    }

    fn write_row_ops(
        &mut self,
        table: &str,
        ops: &[datacraft_core::port::RowWriteOp],
    ) -> Result<(), PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let page = self.page_of(table)?;
        let definition = self.definition_of(table)?.clone();
        // Validate and map every operation before a single byte is written.
        let mut prepared = Vec::with_capacity(ops.len());
        for op in ops {
            prepared.push(match op {
                datacraft_core::port::RowWriteOp::Insert(values) => {
                    PreparedOp::Insert(port_write::to_storage_values(&definition.fields, values)?)
                }
                datacraft_core::port::RowWriteOp::Update { row, values } => PreparedOp::Update {
                    row: *row,
                    values: port_write::to_storage_values_for_update(&definition.fields, values)?,
                },
                datacraft_core::port::RowWriteOp::Delete { row } => {
                    PreparedOp::Delete { row: *row }
                }
            });
        }
        // One writer session: the file is persisted only when every
        // operation succeeded — the batch is atomic on this backend.
        self.with_writer(|writer| {
            for op in &prepared {
                match op {
                    PreparedOp::Insert(values) => writer.insert_row(page, values)?,
                    PreparedOp::Update { row, values } => writer.update_row(page, *row, values)?,
                    PreparedOp::Delete { row } => writer.delete_row(page, *row)?,
                }
            }
            Ok(())
        })?;
        Ok(())
    }
}

/// One validated operation of a batch write (domain values mapped to the
/// storage vocabulary before any byte is written).
enum PreparedOp {
    Insert(Vec<crate::types::StorageValue>),
    Update {
        row: usize,
        values: Vec<crate::types::StorageValue>,
    },
    Delete {
        row: usize,
    },
}

impl Jet4Port {
    /// TDEF page of a catalogued table (the writer's identity).
    fn page_of(&self, table: &str) -> Result<u32, PortError> {
        self.pages
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(table))
            .map(|(_, page)| *page)
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Domain definition of a catalogued table.
    fn definition_of(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Runs one writer operation on a private copy of the file, persists the
    /// result and swaps in the refreshed reader/catalog. A failed operation
    /// leaves the file and the in-memory state untouched.
    fn with_writer<T>(
        &mut self,
        op: impl FnOnce(&mut Jet4Writer) -> Result<T, StorageError>,
    ) -> Result<T, PortError> {
        let path = self.path.clone().ok_or_else(no_database)?;
        let (bytes, encryption) = {
            let reader = self.reader.as_ref().ok_or_else(no_database)?;
            (reader.file.clone(), reader.encryption().copied())
        };
        let mut writer = Jet4Writer::new(bytes).map_err(map_write)?;
        let out = op(&mut writer).map_err(map_write)?;
        let decrypted = writer.into_file();
        // The writer works on decrypted bytes; the on-disk file keeps the
        // header region encrypted and, for protected databases, every page
        // after page 0 (step 16.2, M5).
        let reader = Jet4TableReader::from_decrypted(decrypted, encryption).map_err(map_open)?;
        let header = reader.header().clone();
        let mut disk = reader.file.clone();
        crate::jet::crypto::encrypt_file(&mut disk, &header, encryption.as_ref())
            .map_err(map_open)?;
        std::fs::write(&path, &disk).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let complex = ComplexCatalog::read(&reader).map_err(map_open)?;
        let entries = collect_tables(&reader, &complex)?;
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.complex = complex;
        self.reader = Some(reader);
        Ok(out)
    }
}

/// Enumerates user tables through the catalog with their TDEF pages.
/// Complex backing tables (`f_<guid>_*`) are hidden — they belong to their
/// owning column, not to the navigation pane.
fn collect_tables(
    reader: &Jet4TableReader,
    complex: &ComplexCatalog,
) -> Result<Vec<(TableDef, u32)>, PortError> {
    let mut tables = Vec::new();
    for entry in reader.catalog().map_err(map_open)? {
        if entry.object_type != OBJECT_TYPE_TABLE {
            continue;
        }
        let Some(tdef_page) = entry.tdef_page else {
            continue;
        };
        if entry.name.starts_with("MSys") || complex.is_flat_page(tdef_page) {
            continue;
        }
        let def = JetTableDef::decode(&reader.file, tdef_page).map_err(map_open)?;
        tables.push((
            table_def(&entry.name, tdef_page, &def, reader, complex)?,
            tdef_page,
        ));
    }
    Ok(tables)
}

/// Maps one table's descriptors onto a validated domain definition.
fn table_def(
    name: &str,
    tdef_page: u32,
    def: &JetTableDef,
    reader: &Jet4TableReader,
    complex: &ComplexCatalog,
) -> Result<TableDef, PortError> {
    let mut fields = Vec::with_capacity(def.columns.len());
    for column in &def.columns {
        let mapping = complex.column(tdef_page, &column.name);
        fields.push(field_def(column, mapping, complex, reader)?);
    }
    Ok(TableDef::new(name, fields))
}

/// Maps one column descriptor onto a field definition.
fn field_def(
    column: &JetColumn,
    mapping: Option<&ComplexColumn>,
    complex: &ComplexCatalog,
    reader: &Jet4TableReader,
) -> Result<FieldDef, PortError> {
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
            max_len: column.text_max_len(),
        },
        JetType::Memo => FieldKind::Memo,
        JetType::Guid if autonumber => FieldKind::AutoNumber(AutoKind::ReplicationId),
        JetType::Guid => FieldKind::Guid,
        // Without a mapping the column stays the documented binary fallback
        // (a file whose MSysComplexColumns is absent still reads).
        JetType::Complex => match mapping {
            Some(mapping) => complex.field_kind(reader, mapping).map_err(map_open)?,
            None => FieldKind::Binary(BinaryKind::Attachment),
        },
    };
    let mut field = FieldDef::new(&column.name, kind);
    field.properties.required = !nullable;
    Ok(field)
}

#[cfg(test)]
#[path = "port_adapter_tests.rs"]
mod tests;
