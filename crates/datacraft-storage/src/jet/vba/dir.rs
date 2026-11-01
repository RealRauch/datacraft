//! [MS-OVBA] §2.3.4.2 `dir` stream parser.
//!
//! The decompressed stream is a flat sequence of `id (2) size (4) payload`
//! records. The only exception is `PROJECTVERSION` (0x0009), a fixed
//! 12-byte record without a size field. Module records are delimited by the
//! `MODULETERMINATOR` (0x002B); the fields used here are the module name,
//! stream name, offset and type.

use crate::error::StorageError;

const ID_PROJECTCODEPAGE: u16 = 0x0003;
const ID_PROJECTVERSION: u16 = 0x0009;
const ID_PROJECTMODULES: u16 = 0x000F;
const ID_MODULENAME: u16 = 0x0019;
const ID_MODULESTREAMNAME: u16 = 0x001A;
const ID_MODULEOFFSET: u16 = 0x0031;
const ID_MODULETYPE_PROCEDURAL: u16 = 0x0021;
const ID_MODULETYPE_NONPROCEDURAL: u16 = 0x0022;
const ID_MODULETERMINATOR: u16 = 0x002B;

/// One module record of the `dir` stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirModule {
    /// `MODULENAME` (decoded from the project code page).
    pub name: String,
    /// `MODULESTREAMNAME` — the storage stream holding the module.
    pub stream_name: String,
    /// `MODULEOFFSET` — where the compressed source starts in the stream.
    pub offset: u32,
    /// `MODULETYPE` id (`0x0021`/`0x0022`), when present.
    pub kind: Option<u16>,
}

/// Parsed project information needed to extract sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirInfo {
    /// `PROJECTCODEPAGE` (defaults to 1252 when absent).
    pub code_page: u16,
    /// Modules in stream order.
    pub modules: Vec<DirModule>,
}

impl DirModule {
    /// Whether the module is procedural (standard) according to `MODULETYPE`.
    pub fn is_procedural(&self) -> bool {
        self.kind == Some(ID_MODULETYPE_PROCEDURAL)
    }

    /// Whether the module is non-procedural according to `MODULETYPE`.
    pub fn is_non_procedural(&self) -> bool {
        self.kind == Some(ID_MODULETYPE_NONPROCEDURAL)
    }
}

/// Parses the decompressed `dir` stream.
pub fn parse_dir(bytes: &[u8]) -> Result<DirInfo, StorageError> {
    let mut code_page = 1252u16;
    let mut modules = Vec::new();
    let mut partial = PartialModule::default();
    let mut pos = 0usize;
    while pos + 2 <= bytes.len() {
        let id = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        if id == ID_PROJECTVERSION {
            // Fixed 12-byte record: id, reserved (4), major (4), minor (2).
            if pos + 12 > bytes.len() {
                return Err(corrupt("truncated PROJECTVERSION record"));
            }
            pos += 12;
            continue;
        }
        if pos + 6 > bytes.len() {
            return Err(corrupt("truncated record header"));
        }
        let size = usize::try_from(u32::from_le_bytes([
            bytes[pos + 2],
            bytes[pos + 3],
            bytes[pos + 4],
            bytes[pos + 5],
        ]))
        .map_err(|_| corrupt("record size does not fit the address space"))?;
        let start = pos + 6;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| corrupt(format!("record 0x{id:04X} exceeds the stream")))?;
        let payload = &bytes[start..end];
        pos = end;
        match id {
            ID_PROJECTCODEPAGE if payload.len() >= 2 => {
                code_page = u16::from_le_bytes([payload[0], payload[1]]);
            }
            ID_PROJECTMODULES => {}
            ID_MODULENAME => partial.name = Some(payload.to_vec()),
            ID_MODULESTREAMNAME => partial.stream_name = Some(payload.to_vec()),
            ID_MODULEOFFSET if payload.len() == 4 => {
                partial.offset = Some(u32::from_le_bytes([
                    payload[0], payload[1], payload[2], payload[3],
                ]));
            }
            ID_MODULETYPE_PROCEDURAL | ID_MODULETYPE_NONPROCEDURAL => partial.kind = Some(id),
            ID_MODULETERMINATOR => {
                if let Some(module) = partial.finish(code_page)? {
                    modules.push(module);
                }
                partial = PartialModule::default();
            }
            _ => {}
        }
    }
    if !partial.is_empty() {
        return Err(corrupt(
            "unterminated module record at the end of the stream",
        ));
    }
    Ok(DirInfo { code_page, modules })
}

/// Module fields accumulated between terminators.
#[derive(Debug, Default)]
struct PartialModule {
    name: Option<Vec<u8>>,
    stream_name: Option<Vec<u8>>,
    offset: Option<u32>,
    kind: Option<u16>,
}

impl PartialModule {
    fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.stream_name.is_none()
            && self.offset.is_none()
            && self.kind.is_none()
    }

    fn finish(self, code_page: u16) -> Result<Option<DirModule>, StorageError> {
        let empty = self.is_empty();
        let PartialModule {
            name,
            stream_name,
            offset,
            kind,
        } = self;
        let (Some(name), Some(stream_name), Some(offset)) = (name, stream_name, offset) else {
            if empty {
                return Ok(None);
            }
            return Err(corrupt(
                "module record lacks MODULENAME, MODULESTREAMNAME or MODULEOFFSET",
            ));
        };
        Ok(Some(DirModule {
            name: decode_mbcs(&name, code_page)?,
            stream_name: decode_mbcs(&stream_name, code_page)?,
            offset,
            kind,
        }))
    }
}

/// Decodes MBCS text with the project code page.
pub(crate) fn decode_mbcs(bytes: &[u8], code_page: u16) -> Result<String, StorageError> {
    match code_page {
        1252 => Ok(crate::jet::jet3::cp1252(bytes)),
        65001 => Ok(String::from_utf8_lossy(bytes).into_owned()),
        1200 => {
            if !bytes.len().is_multiple_of(2) {
                return Err(corrupt("UTF-16 project name has an odd byte count"));
            }
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
        other => Err(StorageError::Unsupported {
            reason: format!(
                "VBA project code page {other} is not decoded (supported: 1252, 1200, 65001)"
            ),
        }),
    }
}

fn corrupt(detail: impl Into<String>) -> StorageError {
    StorageError::Corrupt {
        context: "VBA dir stream".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn project_version() -> Vec<u8> {
        let mut out = vec![0x09, 0x00];
        out.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // reserved
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // major
        out.extend_from_slice(&[0x02, 0x00]); // minor
        out
    }

    #[test]
    fn a_minimal_project_yields_one_module_and_its_code_page() {
        let mut dir = project_version();
        dir.extend(record(ID_PROJECTCODEPAGE, &1252u16.to_le_bytes()));
        dir.extend(record(ID_PROJECTMODULES, &1u16.to_le_bytes()));
        dir.extend(record(ID_MODULENAME, b"Utilities"));
        dir.extend(record(ID_MODULESTREAMNAME, b"ABCDEF"));
        dir.extend(record(ID_MODULEOFFSET, &0x1234u32.to_le_bytes()));
        dir.extend(record(ID_MODULETYPE_PROCEDURAL, &[]));
        dir.extend(record(ID_MODULETERMINATOR, &[]));
        let info = parse_dir(&dir).expect("parses");
        assert_eq!(info.code_page, 1252);
        assert_eq!(
            info.modules,
            vec![DirModule {
                name: "Utilities".to_owned(),
                stream_name: "ABCDEF".to_owned(),
                offset: 0x1234,
                kind: Some(ID_MODULETYPE_PROCEDURAL),
            }]
        );
        assert!(info.modules[0].is_procedural());
        assert!(!info.modules[0].is_non_procedural());
    }

    #[test]
    fn an_empty_project_has_no_modules() {
        let mut dir = record(ID_PROJECTCODEPAGE, &1252u16.to_le_bytes());
        dir.extend(record(ID_PROJECTMODULES, &0u16.to_le_bytes()));
        let info = parse_dir(&dir).expect("parses");
        assert!(info.modules.is_empty());
    }

    #[test]
    fn malformed_records_report_precise_errors() {
        assert!(matches!(
            parse_dir(&[0x09, 0x00, 0x00]),
            Err(StorageError::Corrupt { .. })
        ));
        assert!(matches!(
            parse_dir(&[0x19, 0x00]),
            Err(StorageError::Corrupt { .. })
        ));
        assert!(matches!(
            parse_dir(&[0x19, 0x00, 0x10, 0x00, 0x00, 0x00]),
            Err(StorageError::Corrupt { .. })
        ));
        // Unterminated module record.
        let mut dir = record(ID_MODULENAME, b"Utilities");
        dir.extend(record(ID_MODULESTREAMNAME, b"ABCDEF"));
        assert!(matches!(parse_dir(&dir), Err(StorageError::Corrupt { .. })));
        // Terminator without the required fields.
        let mut dir = record(ID_MODULENAME, b"Utilities");
        dir.extend(record(ID_MODULETERMINATOR, &[]));
        assert!(matches!(parse_dir(&dir), Err(StorageError::Corrupt { .. })));
    }

    #[test]
    fn mbcs_decoding_supports_the_documented_code_pages() {
        assert_eq!(decode_mbcs(&[0x80], 1252).expect("cp1252"), "\u{20AC}");
        assert_eq!(decode_mbcs(b"abc", 65001).expect("utf-8"), "abc");
        assert_eq!(
            decode_mbcs(&[b'A', 0, b'B', 0], 1200).expect("utf-16"),
            "AB"
        );
        assert!(matches!(
            decode_mbcs(b"A", 1200),
            Err(StorageError::Corrupt { .. })
        ));
        assert!(matches!(
            decode_mbcs(b"abc", 437),
            Err(StorageError::Unsupported { .. })
        ));
    }
}
