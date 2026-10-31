//! Long-value (memo/OLE) resolution (step 2.3, item H6).
//!
//! Reference: `docs/FORMAT-JET.md` ("Memo / OLE"). Inline values are handled
//! by [`crate::jet::row::decode_row`]; this module resolves single LVAL
//! records (`0x40000000`) and record chains (`0x00000000`).

use crate::error::StorageError;
use crate::jet::data_page::DataPage;
use crate::jet::page::PAGE_TYPE_DATA;
use crate::jet::page::page_type;

/// Resolves a memo/OLE value given its 12-byte header fields.
///
/// `length_and_flags` is the first u32 of the header (length in the low 30
/// bits, storage form in the two top bits), `record_pointer` the second u32
/// (`(page << 8) | row`).
pub fn resolve(
    file: &[u8],
    page_size: usize,
    length_and_flags: u32,
    record_pointer: u32,
) -> Result<Vec<u8>, StorageError> {
    let total_len = (length_and_flags & 0x3FFF_FFFF) as usize;
    match length_and_flags & 0xC000_0000 {
        0x8000_0000 => Err(StorageError::Corrupt {
            context: "LVAL".into(),
            detail: "inline memo reached the resolver — the row decoder handles inline data".into(),
        }),
        0x4000_0000 => {
            let (page, row) = split_pointer(record_pointer)?;
            let data = read_record(file, page_size, page, row)?;
            Ok(take_exact(data, total_len)?)
        }
        _ => {
            // Chain of type-2 records: [next pointer u32][data], 0 ends it.
            let mut out = Vec::with_capacity(total_len);
            let mut pointer = record_pointer;
            loop {
                let (page, row) = split_pointer(pointer)?;
                let record = read_record(file, page_size, page, row)?;
                if record.len() < 4 {
                    return Err(StorageError::Corrupt {
                        context: "LVAL chain".into(),
                        detail: format!(
                            "record at {pointer:#010x} is shorter than its next pointer"
                        ),
                    });
                }
                let next = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
                out.extend_from_slice(&record[4..]);
                if out.len() > total_len {
                    return Err(StorageError::Corrupt {
                        context: "LVAL chain".into(),
                        detail: format!("chain exceeds declared memo length {total_len}"),
                    });
                }
                if next == 0 || out.len() >= total_len {
                    break;
                }
                pointer = next;
            }
            if out.len() < total_len {
                return Err(StorageError::Corrupt {
                    context: "LVAL chain".into(),
                    detail: format!(
                        "chain ended after {} of declared memo length {total_len} bytes",
                        out.len()
                    ),
                });
            }
            Ok(take_exact(&out, total_len)?)
        }
    }
}

fn split_pointer(pointer: u32) -> Result<(u32, u16), StorageError> {
    let page = pointer >> 8;
    let row = (pointer & 0xFF) as u16;
    if page == 0 {
        return Err(StorageError::Corrupt {
            context: "LVAL chain".into(),
            detail: "record pointer names page 0 (the header page)".into(),
        });
    }
    Ok((page, row))
}

fn read_record(file: &[u8], page_size: usize, page: u32, row: u16) -> Result<&[u8], StorageError> {
    let start = page as usize * page_size;
    let page_bytes = file
        .get(start..start + page_size)
        .ok_or_else(|| StorageError::Corrupt {
            context: "LVAL chain".into(),
            detail: format!("LVAL page {page} beyond end of file"),
        })?;
    if page_type(page_bytes) != PAGE_TYPE_DATA {
        return Err(StorageError::Corrupt {
            context: "LVAL chain".into(),
            detail: format!("page {page} is not a data page"),
        });
    }
    let decoded = DataPage::decode(page_bytes)?;
    decoded
        .records
        .get(row as usize)
        .map(|&(start, end)| &page_bytes[start..end])
        .ok_or_else(|| StorageError::Corrupt {
            context: "LVAL chain".into(),
            detail: format!(
                "LVAL page {page} has no record {row} (holds {} records)",
                decoded.records.len()
            ),
        })
}

fn take_exact(data: &[u8], len: usize) -> Result<Vec<u8>, StorageError> {
    if data.len() < len {
        return Err(StorageError::Corrupt {
            context: "LVAL".into(),
            detail: format!("record holds {} bytes, memo declares {len}", data.len()),
        });
    }
    Ok(data[..len].to_vec())
}
