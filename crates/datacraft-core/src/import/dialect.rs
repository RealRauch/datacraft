//! CSV text decoding and delimiter sniffing (step 80.2, item H59): the
//! one shared path behind the Import File flow and the CSV driver, so a
//! file a human can open in a spreadsheet also imports here.
//!
//! The decode ladder is deterministic and refuses instead of guessing
//! lossily: valid UTF-8 first (with or without a BOM — the strict branch
//! every existing source takes, byte-identical), then a UTF-16 byte-order
//! mark, then Windows-1252 for the classic single-byte exports; a byte
//! Windows-1252 never defines or a broken UTF-16 stream is a precise,
//! locatable failure, never silent loss. The delimiter sniff counts the
//! candidates over the first 20 non-blank lines outside quotes and keeps
//! the one whose per-line count is consistent (and non-zero); ties and
//! no-candidate cases stay on the comma default. The sniff is a
//! heuristic: the csv crate stays the parsing truth, so a wrong guess
//! surfaces as a ragged-row failure, never as silently mis-split data.
//! The Arabic single-byte variant of the reference ladder (Windows-1256)
//! is a documented remainder: content in it still fails on the undefined
//! cp1252 bytes instead of decoding lossily.

use std::fmt;

/// The text encodings the decode ladder can name in the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CsvEncoding {
    /// Valid UTF-8, with or without a byte-order mark.
    Utf8,
    /// UTF-16 little-endian, declared by the `FF FE` byte-order mark.
    Utf16Le,
    /// UTF-16 big-endian, declared by the `FE FF` byte-order mark.
    Utf16Be,
    /// The classic single-byte Windows codepage for Western text.
    Windows1252,
}

impl fmt::Display for CsvEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            CsvEncoding::Utf8 => "UTF-8",
            CsvEncoding::Utf16Le => "UTF-16LE",
            CsvEncoding::Utf16Be => "UTF-16BE",
            CsvEncoding::Windows1252 => "Windows-1252",
        };
        f.write_str(name)
    }
}

/// The detected dialect of one CSV source: the encoding the text was
/// decoded from and the delimiter the sniff chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Dialect {
    /// The encoding the source text was decoded from.
    pub encoding: CsvEncoding,
    /// The sniffed delimiter as one ASCII byte (`b','` unless a
    /// candidate won over it).
    pub delimiter: u8,
}

impl Dialect {
    /// The delimiter as it reads in the preview (`Tab` renders as a
    /// word — a bare tab would be invisible).
    pub fn delimiter_label(&self) -> &'static str {
        delimiter_label(self.delimiter)
    }
}

/// The human-readable delimiter label (see [`Dialect::delimiter_label`]).
pub fn delimiter_label(delimiter: u8) -> &'static str {
    match delimiter {
        b',' => ",",
        b';' => ";",
        b'\t' => "Tab",
        _ => "custom",
    }
}

/// How many non-blank leading lines the delimiter sniff counts over.
const SNIFF_LINES: usize = 20;

/// The delimiter candidates, most common first: the tie-break order is
/// the iteration order, so the comma default wins every tie.
const DELIMITER_CANDIDATES: [u8; 3] = *b",;\t";

/// Decodes raw CSV bytes into text plus the detected [`Dialect`].
///
/// The reason of an error stands alone (no path) — the callers prefix
/// the locatable source path. The returned text never carries a
/// byte-order mark.
pub fn decode_csv_text(bytes: Vec<u8>) -> Result<(String, Dialect), String> {
    // The strict UTF-8 branch stays first so every existing source keeps
    // its exact result; `from_utf8` hands the bytes back on failure.
    match String::from_utf8(bytes) {
        Ok(text) => {
            let text = strip_bom(&text);
            let delimiter = detect_delimiter(&text);
            Ok((
                text,
                Dialect {
                    encoding: CsvEncoding::Utf8,
                    delimiter,
                },
            ))
        }
        Err(error) => {
            let bytes = error.into_bytes();
            let (encoding, text) = if bytes.starts_with(&[0xFF, 0xFE]) {
                (
                    CsvEncoding::Utf16Le,
                    decode_utf16(&bytes[2..], Endianness::Little)?,
                )
            } else if bytes.starts_with(&[0xFE, 0xFF]) {
                (
                    CsvEncoding::Utf16Be,
                    decode_utf16(&bytes[2..], Endianness::Big)?,
                )
            } else {
                (CsvEncoding::Windows1252, decode_windows1252(&bytes)?)
            };
            let text = strip_bom(&text);
            let delimiter = detect_delimiter(&text);
            Ok((
                text,
                Dialect {
                    encoding,
                    delimiter,
                },
            ))
        }
    }
}

enum Endianness {
    Little,
    Big,
}

/// Decodes a UTF-16 byte stream (without its byte-order mark) into text;
/// an odd byte count or a lone surrogate is a precise failure.
fn decode_utf16(bytes: &[u8], endianness: Endianness) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(
            "the UTF-16 byte stream has an odd length after the byte-order mark".to_owned(),
        );
    }
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| match endianness {
            Endianness::Little => u16::from_le_bytes(*pair),
            Endianness::Big => u16::from_be_bytes(*pair),
        })
        .collect();
    String::from_utf16(&units)
        .map_err(|error| format!("the UTF-16 byte stream is invalid: {error}"))
}

/// The Windows-1252 mappings of the 0x80–0x9F block (the rest of the
/// codepage matches Latin-1); the undefined slots are never read.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\0', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\0', '\u{017D}', '\0', '\0',
    '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}',
    '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\0', '\u{017E}', '\u{0178}',
];

/// One Windows-1252 byte as its character, or `None` for the undefined
/// bytes.
fn cp1252_char(b: u8) -> Option<char> {
    match b {
        0x80..=0x9F => Some(CP1252_HIGH[(b - 0x80) as usize]),
        _ => Some(b as char),
    }
    .filter(|c| *c != '\0')
}

/// Decodes a single-byte Windows-1252 stream into text; an undefined
/// byte is a precise failure naming the byte and its offset.
fn decode_windows1252(bytes: &[u8]) -> Result<String, String> {
    let mut text = String::with_capacity(bytes.len());
    for (offset, &b) in bytes.iter().enumerate() {
        let Some(c) = cp1252_char(b) else {
            return Err(format!(
                "byte 0x{b:02X} at offset {offset} is not defined in Windows-1252"
            ));
        };
        text.push(c);
    }
    Ok(text)
}

/// Strips a leading byte-order mark character from decoded text.
fn strip_bom(text: &str) -> String {
    match text.strip_prefix('\u{feff}') {
        Some(stripped) => stripped.to_owned(),
        None => text.to_owned(),
    }
}

/// Sniffs the delimiter of one decoded CSV text: the candidate whose
/// per-line count is consistent (every counted line carries the same
/// non-zero number of the candidate outside quotes) wins; ties keep the
/// earlier candidate, so the comma default survives; no consistent
/// candidate (single columns, ragged quoting) stays on the comma too.
pub fn detect_delimiter(text: &str) -> u8 {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(SNIFF_LINES)
        .collect();
    let mut best: Option<(u8, usize)> = None;
    for &candidate in &DELIMITER_CANDIDATES {
        let mut counts = lines
            .iter()
            .map(|line| count_outside_quotes(line, candidate));
        let Some(first) = counts.next() else {
            continue;
        };
        let consistent = first > 0 && counts.all(|count| count == first);
        if consistent && best.is_none_or(|(_, count)| first > count) {
            best = Some((candidate, first));
        }
    }
    best.map(|(candidate, _)| candidate).unwrap_or(b',')
}

/// Counts the occurrences of `delimiter` in one line outside quotes
/// (a `""` escape stays inside the quotes).
fn count_outside_quotes(line: &str, delimiter: u8) -> usize {
    let mut count = 0;
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            _ if !in_quotes && c as u32 == u32::from(delimiter) => count += 1,
            _ => {}
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_encoding_names_render_for_the_preview() {
        assert_eq!(CsvEncoding::Utf8.to_string(), "UTF-8");
        assert_eq!(CsvEncoding::Utf16Le.to_string(), "UTF-16LE");
        assert_eq!(CsvEncoding::Utf16Be.to_string(), "UTF-16BE");
        assert_eq!(CsvEncoding::Windows1252.to_string(), "Windows-1252");
    }

    #[test]
    fn the_delimiter_labels_render_for_the_preview() {
        assert_eq!(delimiter_label(b','), ",");
        assert_eq!(delimiter_label(b';'), ";");
        assert_eq!(delimiter_label(b'\t'), "Tab");
    }

    #[test]
    fn the_cp1252_high_block_maps_its_named_characters() {
        assert_eq!(cp1252_char(0x80), Some('\u{20AC}'), "the euro sign");
        assert_eq!(cp1252_char(0xE4), Some('ä'), "Latin-1 passthrough");
        assert_eq!(cp1252_char(0x81), None, "the C1 hole refuses");
        assert_eq!(cp1252_char(0x9D), None, "the C1 hole refuses");
    }

    #[test]
    fn the_sniff_counts_only_outside_quotes() {
        assert_eq!(count_outside_quotes("\"x,y\",z", b','), 1);
        assert_eq!(count_outside_quotes("\"x\"\"y\",z", b','), 1);
        assert_eq!(count_outside_quotes("plain", b','), 0);
    }

    #[test]
    fn an_empty_text_stays_on_the_comma_default() {
        assert_eq!(detect_delimiter(""), b',');
        assert_eq!(detect_delimiter("\n\n"), b',');
    }
}
