//! VBA module source decoding ([MS-OVBA] §2.3.4.3).
//!
//! A module stream is `[PerformanceCache] [CompressedContainer]`; the
//! `MODULEOFFSET` from the `dir` stream is the start of the compressed
//! source. The decompressed bytes are text in the project code page.

use crate::error::StorageError;

use super::decompress;

/// Decodes one module stream at its `MODULEOFFSET`.
pub fn decode_source(stream: &[u8], offset: u32, code_page: u16) -> Result<String, StorageError> {
    let start = usize::try_from(offset).map_err(|_| corrupt("MODULEOFFSET overflows"))?;
    let compressed = stream.get(start..).ok_or_else(|| {
        corrupt(format!(
            "MODULEOFFSET {offset} is beyond the module stream ({} bytes)",
            stream.len()
        ))
    })?;
    let bytes = decompress::decompress(compressed)?;
    decode_text(&bytes, code_page)
}

/// Decodes source bytes with the project code page.
fn decode_text(bytes: &[u8], code_page: u16) -> Result<String, StorageError> {
    match code_page {
        1252 => Ok(crate::jet::jet3::cp1252(bytes)),
        65001 => Ok(String::from_utf8_lossy(bytes).into_owned()),
        1200 => {
            if !bytes.len().is_multiple_of(2) {
                return Err(corrupt("UTF-16 source has an odd byte count"));
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
        context: "VBA module stream".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_container(text: &[u8]) -> Vec<u8> {
        // One compressed chunk with literal-only token sequences (at most
        // eight literals per flag byte).
        let mut payload = Vec::new();
        for chunk in text.chunks(8) {
            payload.push(0x00);
            payload.extend_from_slice(chunk);
        }
        let mut data = vec![0x01];
        let size = payload.len() as u16;
        data.extend_from_slice(&(size | 0x3000 | 0x8000).to_le_bytes());
        data.extend_from_slice(&payload);
        data
    }

    #[test]
    fn sources_are_read_at_the_module_offset_in_the_project_code_page() {
        let mut stream = vec![0xFF; 7]; // performance cache prefix
        stream.extend(literal_container(b"Attribute VB_Name = \"X\"\n"));
        let source = decode_source(&stream, 7, 1252).expect("decodes");
        assert_eq!(source, "Attribute VB_Name = \"X\"\n");
    }

    #[test]
    fn offsets_beyond_the_stream_and_unsupported_code_pages_are_precise() {
        assert!(matches!(
            decode_source(b"short", 99, 1252),
            Err(StorageError::Corrupt { .. })
        ));
        let stream = literal_container(b"abc");
        assert!(matches!(
            decode_source(&stream, 0, 437),
            Err(StorageError::Unsupported { .. })
        ));
    }

    #[test]
    fn utf8_and_utf16_sources_decode() {
        assert_eq!(
            decode_source(&literal_container(b"abc"), 0, 65001).expect("utf-8"),
            "abc"
        );
        assert_eq!(
            decode_source(&literal_container(&[b'A', 0, b'B', 0]), 0, 1200).expect("utf-16"),
            "AB"
        );
        assert!(matches!(
            decode_source(&literal_container(b"A"), 0, 1200),
            Err(StorageError::Corrupt { .. })
        ));
    }
}
