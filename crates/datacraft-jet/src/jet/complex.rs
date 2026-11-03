//! ACE complex-value decoding (step 16.1, item M4).
//!
//! `MSysComplexColumns` maps a conceptual table's complex column to the
//! `f_<guid>_*` flat table that stores its sub-records. The parent row
//! carries a row-scoped complex id in the complex column (the descriptor's
//! `misc` field holds the same id); the flat table's `_<name>` link column
//! carries that id per sub-record. Grouping the flat rows by the id yields
//! the ordered sub-records of each cell.
//!
//! Field-kind mapping (documented in `docs/FORMAT-JET.md` §"Complex values"):
//! the attachment complex type maps to `Binary(Attachment)`, a flat table
//! with exactly one user value column maps to `MultiValue(element kind)`,
//! everything else (e.g. version history) maps to `Complex`.

use std::collections::{HashMap, HashSet};

use datacraft_core::schema::{
    BinaryKind, ComplexField, ComplexRecord, FieldKind, FieldValue, MultiValueKind, NumericKind,
};

use crate::error::StorageError;
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, JetType, flags};
use crate::mapping::to_field_value;
use crate::types::StorageValue;

/// Catalog table holding the complex-column mapping.
const COMPLEX_CATALOG: &str = "MSysComplexColumns";
/// Complex type name of ACE attachment columns.
const ATTACHMENT_TYPE: &str = "MSysComplexType_Attachment";

/// One `MSysComplexColumns` row: a complex column of a conceptual table and
/// the flat table backing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexColumn {
    /// Column name in the conceptual table.
    pub column_name: String,
    /// TDEF page of the conceptual (owning) table.
    pub conceptual_tdef: u32,
    /// TDEF page of the flat backing table.
    pub flat_tdef: u32,
    /// Name of the complex type object (`MSysComplexType_Attachment`, …).
    pub type_name: String,
}

/// The complex-column mapping of one database (empty when the file has no
/// `MSysComplexColumns` table, e.g. Jet4).
#[derive(Debug, Default)]
pub struct ComplexCatalog {
    columns: Vec<ComplexColumn>,
    flat_pages: HashSet<u32>,
}

impl ComplexCatalog {
    /// Reads the mapping; a file without the catalog has no complex columns.
    pub fn read(reader: &Jet4TableReader) -> Result<Self, StorageError> {
        let data = match reader.open_table(COMPLEX_CATALOG) {
            Ok(data) => data,
            Err(StorageError::TableNotFound(_)) => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let index_of = |name: &str| -> Result<usize, StorageError> {
            data.schema
                .columns
                .iter()
                .position(|column| column.name == name)
                .ok_or_else(|| StorageError::Corrupt {
                    context: "complex catalog".into(),
                    detail: format!("{COMPLEX_CATALOG} lacks the {name} column"),
                })
        };
        let i_name = index_of("ColumnName")?;
        let i_type = index_of("ComplexTypeObjectID")?;
        let i_flat = index_of("FlatTableID")?;
        let i_conceptual = index_of("ConceptualTableID")?;
        let type_names: HashMap<i64, String> = reader
            .catalog()?
            .into_iter()
            .map(|entry| (entry.id, entry.name))
            .collect();

        let mut columns = Vec::with_capacity(data.rows.len());
        for row in &data.rows {
            let column_name = text(&row[i_name])?;
            let type_id = integer(&row[i_type])?;
            columns.push(ComplexColumn {
                column_name,
                conceptual_tdef: page_number(&row[i_conceptual])?,
                flat_tdef: page_number(&row[i_flat])?,
                type_name: type_names.get(&type_id).cloned().unwrap_or_default(),
            });
        }
        let flat_pages = columns.iter().map(|column| column.flat_tdef).collect();
        Ok(Self {
            columns,
            flat_pages,
        })
    }

    /// Whether `page` is a flat backing table (hidden from the user catalog).
    pub fn is_flat_page(&self, page: u32) -> bool {
        self.flat_pages.contains(&page)
    }

    /// The mapping of one complex column, matched by owning TDEF page and
    /// column name (case-insensitive, like every schema name comparison).
    pub fn column(&self, conceptual_tdef: u32, name: &str) -> Option<&ComplexColumn> {
        self.columns.iter().find(|column| {
            column.conceptual_tdef == conceptual_tdef
                && column.column_name.eq_ignore_ascii_case(name)
        })
    }

    /// Domain kind of a mapped complex column: attachment, multi-value with
    /// the flat table's scalar family, or a generic complex column.
    pub fn field_kind(
        &self,
        reader: &Jet4TableReader,
        mapping: &ComplexColumn,
    ) -> Result<FieldKind, StorageError> {
        if mapping.type_name == ATTACHMENT_TYPE {
            return Ok(FieldKind::Binary(BinaryKind::Attachment));
        }
        let flat = JetTableDef::decode(&reader.file, mapping.flat_tdef)?;
        let values: Vec<&JetColumn> = flat
            .columns
            .iter()
            .filter(|column| !is_link(column) && !is_autonumber(column))
            .collect();
        if values.len() == 1
            && let Some(kind) = multi_value_kind(values[0])
        {
            return Ok(FieldKind::MultiValue(kind));
        }
        Ok(FieldKind::Complex)
    }
}

/// The grouped sub-records of one flat table.
#[derive(Debug)]
pub struct FlatRows {
    records: HashMap<u32, Vec<ComplexRecord>>,
}

impl FlatRows {
    /// Reads a flat table and groups its rows by the `_<name>` link id.
    pub fn read(reader: &Jet4TableReader, mapping: &ComplexColumn) -> Result<Self, StorageError> {
        let def = JetTableDef::decode(&reader.file, mapping.flat_tdef)?;
        let link = def
            .columns
            .iter()
            .position(is_link)
            .ok_or_else(|| StorageError::Corrupt {
                context: "complex value".into(),
                detail: format!(
                    "flat table {} of column {:?} has no _<name> link column",
                    mapping.flat_tdef, mapping.column_name
                ),
            })?;
        let value_indices: Vec<usize> = def
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| !is_link(column) && !is_autonumber(column))
            .map(|(index, _)| index)
            .collect();
        let data = reader.read_table(mapping.flat_tdef)?;
        let mut records: HashMap<u32, Vec<ComplexRecord>> = HashMap::new();
        for row in &data.rows {
            let Some(id) = complex_id(&row[link])? else {
                continue;
            };
            let fields = value_indices
                .iter()
                .map(|&index| ComplexField {
                    name: def.columns[index].name.clone(),
                    value: to_field_value(&row[index]),
                })
                .collect();
            records
                .entry(id)
                .or_default()
                .push(ComplexRecord { fields });
        }
        Ok(Self { records })
    }

    /// Sub-records of one complex id, in flat-table row order.
    pub fn records(&self, id: u32) -> Vec<ComplexRecord> {
        self.records.get(&id).cloned().unwrap_or_default()
    }
}

/// Resolves one parent cell: `Null` stays `Null`, a complex id expands to
/// its grouped sub-records (an id without rows is an empty list).
pub fn resolve_cell(value: &StorageValue, flat: &FlatRows) -> Result<FieldValue, StorageError> {
    Ok(match complex_id(value)? {
        None => FieldValue::Null,
        Some(id) => FieldValue::Complex(flat.records(id)),
    })
}

/// The flat table's link column: an integer column named `_<name>`.
fn is_link(column: &JetColumn) -> bool {
    column.name.starts_with('_') && column.data_type == JetType::LongInt
}

/// The flat table's own AutoNumber identity column (never user data).
fn is_autonumber(column: &JetColumn) -> bool {
    column.flags & flags::AUTONUMBER != 0
}

/// Scalar family of a multi-value element column; `None` for shapes that are
/// not a scalar multi-value (memo/OLE/binary columns).
fn multi_value_kind(column: &JetColumn) -> Option<MultiValueKind> {
    Some(match column.data_type {
        JetType::Text => MultiValueKind::Text {
            max_len: column.text_max_len(),
        },
        JetType::Byte => MultiValueKind::Number(NumericKind::Byte),
        JetType::Int => MultiValueKind::Number(NumericKind::Integer),
        JetType::LongInt => MultiValueKind::Number(NumericKind::LongInteger),
        JetType::Float => MultiValueKind::Number(NumericKind::Single),
        JetType::Double => MultiValueKind::Number(NumericKind::Double),
        JetType::Numeric => MultiValueKind::Number(NumericKind::Decimal {
            precision: 28,
            scale: column.scale,
        }),
        JetType::DateTime => MultiValueKind::DateTime,
        JetType::Currency => MultiValueKind::Currency,
        JetType::Bool => MultiValueKind::Boolean {
            tri_state: column.is_nullable(),
        },
        JetType::Guid => MultiValueKind::Guid,
        JetType::Binary | JetType::Ole | JetType::LongBinary | JetType::Memo | JetType::Complex => {
            return None;
        }
    })
}

/// The row-scoped complex id stored in a complex cell.
fn complex_id(value: &StorageValue) -> Result<Option<u32>, StorageError> {
    match value {
        StorageValue::Null => Ok(None),
        StorageValue::Binary(bytes) if bytes.len() == 4 => Ok(Some(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        StorageValue::Int(id) if (0..=i64::from(u32::MAX)).contains(id) => Ok(Some(*id as u32)),
        other => Err(StorageError::Corrupt {
            context: "complex value".into(),
            detail: format!("complex cell is neither Null nor a 4-byte id: {other:?}"),
        }),
    }
}

fn text(value: &StorageValue) -> Result<String, StorageError> {
    match value {
        StorageValue::Text(text) => Ok(text.clone()),
        other => Err(corrupt_catalog(format!(
            "ColumnName is not text: {other:?}"
        ))),
    }
}

fn integer(value: &StorageValue) -> Result<i64, StorageError> {
    match value {
        StorageValue::Int(value) => Ok(*value),
        other => Err(corrupt_catalog(format!(
            "complex type/table id is not an integer: {other:?}"
        ))),
    }
}

fn page_number(value: &StorageValue) -> Result<u32, StorageError> {
    let id = integer(value)?;
    u32::try_from(id).map_err(|_| corrupt_catalog(format!("page id {id} is out of range")))
}

fn corrupt_catalog(detail: String) -> StorageError {
    StorageError::Corrupt {
        context: "complex catalog".into(),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, data_type: JetType, flags: u8) -> JetColumn {
        JetColumn::new(name, data_type, flags, 0, 0, 4)
    }

    #[test]
    fn complex_ids_decode_from_binary_and_integers() {
        assert_eq!(complex_id(&StorageValue::Null).unwrap(), None);
        assert_eq!(
            complex_id(&StorageValue::Binary(vec![7, 0, 0, 0])).unwrap(),
            Some(7)
        );
        assert_eq!(complex_id(&StorageValue::Int(9)).unwrap(), Some(9));
        assert!(complex_id(&StorageValue::Binary(vec![1, 2, 3])).is_err());
        assert!(complex_id(&StorageValue::Text("x".into())).is_err());
        assert!(complex_id(&StorageValue::Int(-1)).is_err());
    }

    #[test]
    fn link_and_autonumber_detection_follows_the_flat_table_shape() {
        assert!(is_link(&column("_Data", JetType::LongInt, 0x02)));
        assert!(!is_link(&column("Data", JetType::LongInt, 0x02)));
        assert!(!is_link(&column("_Data", JetType::Text, 0x02)));
        assert!(is_autonumber(&column(
            "Id",
            JetType::LongInt,
            flags::AUTONUMBER
        )));
        assert!(!is_autonumber(&column("Id", JetType::LongInt, 0)));
    }

    #[test]
    fn multi_value_kinds_cover_the_scalar_families() {
        let mut text = column("Value", JetType::Text, 0x02);
        text.len = 510;
        text.compressed_text = true;
        assert_eq!(
            multi_value_kind(&text),
            Some(MultiValueKind::Text { max_len: 510 })
        );
        assert_eq!(
            multi_value_kind(&column("Value", JetType::LongInt, 0x02)),
            Some(MultiValueKind::Number(NumericKind::LongInteger))
        );
        assert_eq!(
            multi_value_kind(&column("Value", JetType::DateTime, 0x02)),
            Some(MultiValueKind::DateTime)
        );
        assert_eq!(
            multi_value_kind(&column("Value", JetType::Guid, 0x02)),
            Some(MultiValueKind::Guid)
        );
        assert_eq!(multi_value_kind(&column("Value", JetType::Ole, 0x02)), None);
        assert_eq!(
            multi_value_kind(&column("Value", JetType::Memo, 0x02)),
            None
        );
    }

    #[test]
    fn empty_groups_resolve_to_an_empty_complex_value() {
        let flat = FlatRows {
            records: HashMap::new(),
        };
        assert_eq!(
            resolve_cell(&StorageValue::Binary(vec![1, 0, 0, 0]), &flat).unwrap(),
            FieldValue::Complex(Vec::new())
        );
        assert_eq!(
            resolve_cell(&StorageValue::Null, &flat).unwrap(),
            FieldValue::Null
        );
    }

    #[test]
    fn catalog_value_readers_are_precise() {
        assert_eq!(text(&StorageValue::Text("x".into())).unwrap(), "x");
        assert!(text(&StorageValue::Int(1)).is_err());
        assert_eq!(integer(&StorageValue::Int(3)).unwrap(), 3);
        assert!(integer(&StorageValue::Text("x".into())).is_err());
        assert_eq!(page_number(&StorageValue::Int(3)).unwrap(), 3);
        assert!(page_number(&StorageValue::Int(-1)).is_err());
    }
}
