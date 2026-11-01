//! Text decoding for Jet/ACE rows (split from `row.rs` to keep every file
//! under the size gate).
//!
//! Reference: `docs/FORMAT-JET.md` ("Text compression").

/// Text decoding: UCS-2LE or compressed unicode on Jet4, CP1252 on Jet3
/// (see `docs/FORMAT-JET.md`).
pub(crate) fn decode_text(bytes: &[u8], compressed: bool, jet3: bool) -> String {
    if jet3 {
        return crate::jet::jet3::cp1252(bytes);
    }
    if compressed {
        decode_compressed_text(bytes)
    } else {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .collect();
        String::from_utf16_lossy(&units)
    }
}

/// Compressed unicode (Jet4). Two evidence-backed shapes:
///
/// - Without the `FF FE` header (compressed text columns): single-byte
///   characters until a `0x00` switches to UCS-2 pairs for the remainder;
///   a pair with high byte `0` reads back as its low byte (identical to
///   the compressed reading).
/// - With the `FF FE` header (Access memos, and values that mix ASCII and
///   non-ASCII): the same mode-switch reading applies after the header —
///   the header announces that compression is in use, not the first byte.
pub(crate) fn decode_compressed_text(bytes: &[u8]) -> String {
    let body = bytes.strip_prefix(&[0xFF, 0xFE][..]).unwrap_or(bytes);
    let mut text = String::new();
    let mut at = 0usize;
    while at < body.len() {
        let b = body[at];
        at += 1;
        if b == 0x00 {
            let units: Vec<u16> = body[at..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect();
            text.push_str(&String::from_utf16_lossy(&units));
            return text;
        }
        text.push(char::from(b));
    }
    text
}
