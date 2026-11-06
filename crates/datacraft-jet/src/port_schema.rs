//! Jet column descriptors -> domain field definitions (split from
//! `port_adapter.rs` to keep every file under the size gate).
//!
//! Mapping decisions live in `docs/FORMAT-JET.md` §"Domain mapping": plain
//! GUID → `FieldKind::Guid`, GUID + AUTONUMBER → `AutoNumber(ReplicationId)`,
//! text length from the descriptor (compressed = byte-per-char), Numeric
//! precision modeled as 28, `required` = the inverted NULLABLE flag.

use datacraft_core::port::PortError;
use datacraft_core::schema::{AutoKind, BinaryKind, FieldDef, FieldKind, NumericKind, TableDef};

use crate::jet::JetType;
use crate::jet::complex::{ComplexCatalog, ComplexColumn};
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, flags};
use crate::port_errors::map_open;

/// Maps one table's descriptors onto a validated domain definition.
pub(crate) fn table_def(
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
pub(crate) fn field_def(
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
            scale: (column.misc & 0xFF) as u8,
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
    field.properties.indexed = column.indexed;
    Ok(field)
}
