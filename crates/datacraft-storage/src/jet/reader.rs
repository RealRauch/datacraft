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
use crate::jet::row::{decode_row, decode_row_jet3};
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
    pub(crate) header: FileHeader,
    pub(crate) file: Vec<u8>,
}

impl Jet4TableReader {
    /// Takes ownership of the raw file bytes and validates the header.
    pub fn new(file: Vec<u8>) -> Result<Self, StorageError> {
        let header = FileHeader::decode(&file, file.len() as u64)?;
        Ok(Self { header, file })
    }

    /// The decoded file header.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Consumes the reader and returns the raw file bytes (writer handoff,
    /// step 10.1).
    pub fn into_raw_file(self) -> Vec<u8> {
        self.file
    }

    /// Resolves a record slot, following overflow pointers (bounded depth).
    fn resolve_record<'a>(
        &'a self,
        page: &'a [u8],
        decoded: &DataPage,
        i: usize,
        depth: u8,
    ) -> Result<&'a [u8], StorageError> {
        if !decoded.is_overflow(i) {
            let (start, end) = decoded.record(i);
            return Ok(&page[start..end]);
        }
        if depth >= 4 {
            return Err(StorageError::Unsupported {
                reason: "overflow chain deeper than 4 levels".into(),
            });
        }
        let (pos, _, _) = decoded.records[i];
        let bytes = page
            .get(pos..pos + 4)
            .ok_or_else(|| StorageError::Corrupt {
                context: "data page".into(),
                detail: format!("overflow slot {i} points past the page"),
            })?;
        let pointer = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let target_page = pointer >> 8;
        let row = (pointer & 0xFF) as usize;
        let target = self.header.page_slice(&self.file, target_page)?;
        if page_type(target) != PAGE_TYPE_DATA {
            return Err(StorageError::Corrupt {
                context: "data page".into(),
                detail: format!("overflow target page {target_page} is not a data page"),
            });
        }
        let target_decoded = DataPage::decode(target)?;
        if target_decoded.records.get(row).is_none() {
            return Err(StorageError::Corrupt {
                context: "data page".into(),
                detail: format!(
                    "overflow target page {target_page} has no record {row} (holds {} slots)",
                    target_decoded.records.len()
                ),
            });
        }
        self.resolve_record(target, &target_decoded, row, depth + 1)
    }

    /// Reads the table rooted at `tdef_page`: schema plus all rows.
    /// Jet3 files dispatch to the 1-byte layout paths (step 10.6, L1).
    pub fn read_table(&self, tdef_page: u32) -> Result<JetTableData, StorageError> {
        let def = JetTableDef::decode(&self.file, tdef_page)?;
        let jet3 = self.header.format == crate::types::FormatVersion::Jet3;
        let mut rows = Vec::new();
        for page_idx in 1..self.header.page_count {
            let page = self.header.page_slice(&self.file, page_idx as u32)?;
            if page_type(page) != PAGE_TYPE_DATA {
                continue;
            }
            let decoded = if jet3 {
                DataPage::decode_jet3(page)?
            } else {
                DataPage::decode(page)?
            };
            if decoded.owner != tdef_page {
                continue;
            }
            for i in 0..decoded.records.len() {
                // Skipped: deleted rows and overflow targets (both flagged
                // 0x8000); overflow sources (0x4000) resolve via pointer.
                if decoded.records[i].2 & 0x8000 != 0 {
                    continue;
                }
                let record = self.resolve_record(page, &decoded, i, 0)?;
                rows.push(if jet3 {
                    decode_row_jet3(record, &def, &self.file, self.header.page_size)?
                } else {
                    decode_row(record, &def, &self.file, self.header.page_size)?
                });
            }
        }
        Ok(JetTableData {
            schema: TableSchema::new(format!("tdef_{tdef_page:04}"), def.schema_columns()),
            rows,
        })
    }
}
