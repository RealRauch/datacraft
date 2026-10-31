//! Jet4 table reader (step 2.3, item H6): TDEF + data-page scan + row decode.
//!
//! Data pages are discovered by scanning for type `0x01` pages owned by the
//! requested TDEF page (documented simplification, `docs/FORMAT-JET.md`
//! "Strategy notes"). Table names come from the catalog (step 2.4); until
//! then schemas carry a derived `tdef_NNNN` label.

use crate::error::StorageError;
use crate::jet::data_page::DataPage;
use crate::jet::header::FileHeader;
use crate::jet::page::{PAGE_TYPE_DATA, page_type};
use crate::jet::row::decode_row;
use crate::jet::tdef::JetTableDef;
use crate::types::{Row, TableSchema};

/// A decoded table: schema (catalog label) plus rows in page order.
#[derive(Debug)]
pub struct JetTableData {
    /// Table schema; the name is the derived `tdef_NNNN` label until the
    /// catalog supplies the real name (step 2.4).
    pub schema: TableSchema,
    /// Rows in page/record order.
    pub rows: Vec<Row>,
}

/// Read-only reader over an in-memory Jet4 file.
#[derive(Debug)]
pub struct Jet4TableReader {
    header: FileHeader,
    file: Vec<u8>,
}

impl Jet4TableReader {
    /// Takes ownership of the raw file bytes and validates the header.
    pub fn new(file: Vec<u8>) -> Result<Self, StorageError> {
        let header = FileHeader::decode(&file, file.len() as u64)?;
        if header.format == crate::types::FormatVersion::Jet3 {
            return Err(StorageError::Unsupported {
                reason: "Jet3 (Access 97) databases are not supported yet (BACKLOG L1)".into(),
            });
        }
        Ok(Self { header, file })
    }

    /// The decoded file header.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Reads the table rooted at `tdef_page`: schema plus all rows.
    pub fn read_table(&self, tdef_page: u32) -> Result<JetTableData, StorageError> {
        let def = JetTableDef::decode(&self.file, tdef_page)?;
        let mut rows = Vec::new();
        for page_idx in 1..self.header.page_count {
            let page = self.header.page_slice(&self.file, page_idx as u32)?;
            if page_type(page) != PAGE_TYPE_DATA {
                continue;
            }
            let decoded = DataPage::decode(page)?;
            if decoded.owner != tdef_page {
                continue;
            }
            for i in 0..decoded.records.len() {
                let record = decoded.record(page, i);
                rows.push(decode_row(record, &def, &self.file, self.header.page_size)?);
            }
        }
        Ok(JetTableData {
            schema: TableSchema::new(format!("tdef_{tdef_page:04}"), def.schema_columns()),
            rows,
        })
    }
}
