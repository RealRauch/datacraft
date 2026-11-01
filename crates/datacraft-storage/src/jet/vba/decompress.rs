//! [MS-OVBA] §2.4.1 decompression.
//!
//! A `CompressedContainer` is a `0x01` signature followed by chunks. Each
//! chunk header carries the total chunk size (bits 0–11, minus three), the
//! `0b011` signature (bits 12–14) and the compressed flag (bit 15). Inside
//! a compressed chunk a flag byte selects literal bytes or copy tokens for
//! up to eight tokens; the copy offset/length bit split depends on the
//! position inside the current 4096-byte decompressed chunk.

use crate::error::StorageError;

/// Maximum decompressed bytes per chunk (spec constant).
const MAX_CHUNK: usize = 4096;

/// Decompresses one `CompressedContainer`.
pub fn decompress(container: &[u8]) -> Result<Vec<u8>, StorageError> {
    if container.first() != Some(&0x01) {
        return Err(corrupt("missing CompressedContainer signature 0x01"));
    }
    let mut out = Vec::new();
    let mut pos = 1usize;
    while pos < container.len() {
        if pos + 2 > container.len() {
            return Err(corrupt("truncated chunk header"));
        }
        let header = u16::from_le_bytes([container[pos], container[pos + 1]]);
        if (header >> 12) & 0x7 != 0b011 {
            return Err(corrupt(format!(
                "invalid chunk signature (header 0x{header:04X})"
            )));
        }
        let start = pos;
        let size = usize::from(header & 0x0FFF) + 3;
        let end = (start + size).min(container.len());
        let compressed = header & 0x8000 != 0;
        pos = start + 2;
        let chunk_start = out.len();
        if !compressed {
            let available = end.saturating_sub(pos).min(MAX_CHUNK);
            out.extend_from_slice(&container[pos..pos + available]);
            pos = end;
            continue;
        }
        while pos < end {
            let flags = container[pos];
            pos += 1;
            for bit in 0..8 {
                if pos >= end {
                    break;
                }
                if flags & (1 << bit) == 0 {
                    out.push(container[pos]);
                    pos += 1;
                    continue;
                }
                if pos + 2 > end {
                    return Err(corrupt("truncated copy token"));
                }
                let token = u16::from_le_bytes([container[pos], container[pos + 1]]);
                pos += 2;
                let position = out.len() - chunk_start;
                let bit_count = bit_count(position);
                let length_mask = 0xFFFFu16 >> bit_count;
                let length = usize::from(token & length_mask) + 3;
                let offset = usize::from((token & !length_mask) >> (16 - bit_count)) + 1;
                if offset > position {
                    return Err(corrupt(format!(
                        "copy offset {offset} reaches before the current chunk \
                         (position {position})"
                    )));
                }
                let source = out.len() - offset;
                for index in 0..length {
                    let byte = out[source + index];
                    out.push(byte);
                }
            }
        }
        let produced = out.len() - chunk_start;
        if produced == 0 || produced > MAX_CHUNK {
            return Err(corrupt(format!(
                "chunk produced {produced} bytes (expected 1..={MAX_CHUNK})"
            )));
        }
    }
    Ok(out)
}

/// Ceil(log2(position)), floored at the spec's minimum of four bits.
fn bit_count(position: usize) -> u16 {
    let bits = if position <= 1 {
        0
    } else {
        (usize::BITS - (position - 1).leading_zeros()) as u16
    };
    bits.max(4)
}

fn corrupt(detail: impl Into<String>) -> StorageError {
    StorageError::Corrupt {
        context: "VBA compressed container".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(size_minus_three: u16, compressed: bool) -> [u8; 2] {
        let flag = if compressed { 0x8000 } else { 0 };
        (size_minus_three | 0x3000 | flag).to_le_bytes()
    }

    #[test]
    fn a_raw_chunk_copies_its_uncompressed_bytes() {
        let mut container = vec![0x01];
        container.extend_from_slice(&header(4095, false));
        container.extend(std::iter::repeat_n(b'A', 4096));
        let out = decompress(&container).expect("raw chunk decodes");
        assert_eq!(out, vec![b'A'; 4096]);
    }

    #[test]
    fn literal_and_copy_tokens_decode_with_the_dynamic_bit_split() {
        // "abc" literals (flag bits 0..2) + one copy token (bit 3) that
        // repeats them: offset 3, length 3 -> token 0x2000 at position 3.
        let mut data = vec![0x01];
        let payload = [0x08, b'a', b'b', b'c', 0x00, 0x20];
        data.extend_from_slice(&header((payload.len() as u16) - 1, true));
        data.extend_from_slice(&payload);
        assert_eq!(decompress(&data).expect("decodes"), b"abcabc");
    }

    #[test]
    fn malformed_containers_report_precise_errors() {
        assert!(matches!(decompress(&[]), Err(StorageError::Corrupt { .. })));
        assert!(matches!(
            decompress(&[0x02]),
            Err(StorageError::Corrupt { .. })
        ));
        // Signature byte only, then a truncated header.
        assert!(matches!(
            decompress(&[0x01, 0x00]),
            Err(StorageError::Corrupt { .. })
        ));
        // Wrong chunk signature.
        let wrong_signature = [0x01, 0x00, 0x00, 0x00];
        assert!(matches!(
            decompress(&wrong_signature),
            Err(StorageError::Corrupt { .. })
        ));
        // Compressed chunk whose copy token points before the chunk.
        let mut data = vec![0x01];
        data.extend_from_slice(&header(3, true));
        data.extend_from_slice(&[0x01, 0x00, 0x00]);
        assert!(matches!(
            decompress(&data),
            Err(StorageError::Corrupt { detail, .. }) if detail.contains("copy offset")
        ));
        // A copy token that overruns the 4096-byte chunk limit.
        let mut data = vec![0x01];
        data.extend_from_slice(&header(5, true));
        data.extend_from_slice(&[0x01, 0x00, 0xFF]); // length mask all ones
        data.extend_from_slice(&[0x00, 0x00]); // 4098-byte copy? rejected
        assert!(matches!(
            decompress(&data),
            Err(StorageError::Corrupt { .. })
        ));
    }

    #[test]
    fn the_bit_split_grows_with_the_position() {
        assert_eq!(bit_count(0), 4);
        assert_eq!(bit_count(1), 4);
        assert_eq!(bit_count(16), 4);
        assert_eq!(bit_count(17), 5);
        assert_eq!(bit_count(33), 6);
        assert_eq!(bit_count(4096), 12);
    }
}
