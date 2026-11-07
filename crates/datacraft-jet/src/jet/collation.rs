//! Jet4 text index sort keys (step 49.2, item M63).
//!
//! The engine does not store plain characters in index leaves: text keys are
//! collation sort keys of the LANGID `0x0409`/CP1252 "General" sort order.
//! The grammar and the weight table are **observed engine output** (DAO oracle
//! probe batch, 2026-11-07: single-character tables enumerated over the CP1252
//! repertoire, plus position and accent alignment probes) and documented in
//! `docs/FORMAT-JET.md` ("Fresh-create template and the engine's open gates").
//! DataCraft embeds the observed table as a data constant; characters outside
//! it answer a precise `Unsupported` instead of being guessed.
//!
//! Key grammar (all facts observed on engine-written leaves):
//!
//! ```text
//! key = level1 [0x01 level2 [0x01 level3 [0x01 level4 [0x01 level5]]]] 0x00
//! ```
//!
//! - level 1: the primary weights, one or two bytes per sort element; a
//!   character can contribute two elements (the ligatures `Æ æ Œ œ ß Þ þ`).
//! - level 2: one secondary byte per sort element up to the last element whose
//!   secondary is not the default `0x02` (accents; trailing defaults drop).
//! - levels 3 and 4 are always empty in this collation (case folds into the
//!   primary weights, no width/kana variants).
//! - level 5: ignorable characters, one block each:
//!   `0x80 | (code >> 8), code & 0xFF, 0x06, weight` with `code = 4*k + 7`
//!   and `k` = sort elements emitted before the ignorable.
//! - Separators are emitted up to the last non-empty level; a key with no
//!   lower-level data carries exactly one `0x01` before the `0x00`.
//! - Trailing spaces (`0x20`) are trimmed before encoding; an empty or
//!   all-space value encodes as `01 00`.
//!
//! The three constants `0x06` (ignorable block), `0x07` (position base) and
//! the `0x80`-marked two-byte position form are observed values, not derived
//! from the public `HACKING` sketch (whose simple `A-Z -> 0x60-0x79` ranges do
//! not describe this sort order).

use crate::error::StorageError;

/// Secondary weight of an element without a diacritic.
const DEFAULT_SECONDARY: u8 = 0x02;
/// Level separator.
const LEVEL_SEPARATOR: u8 = 0x01;
/// Key terminator.
const KEY_TERMINATOR: u8 = 0x00;
/// Constant byte of an ignorable block (observed on every ignorable entry).
const IGNORABLE_MARKER: u8 = 0x06;
/// Largest encodable ignorable position code (`0x80`-marked two-byte form).
const MAX_POSITION_CODE: u32 = 0x7FFF;

/// One observed character weight.
struct CharWeight {
    /// Code point the entry describes.
    ch: char,
    /// Primary sort elements, already in their stored byte form.
    primary: &'static [u8],
    /// Number of sort elements the character emits.
    elements: u8,
    /// Secondary (accent) weight per element.
    secondary: u8,
    /// Weight of an ignorable character, `None` for sortable characters.
    ignorable: Option<u8>,
}

/// Const constructor for the table literal.
const fn w(
    ch: char,
    primary: &'static [u8],
    elements: u8,
    secondary: u8,
    ignorable: Option<u8>,
) -> CharWeight {
    CharWeight {
        ch,
        primary,
        elements,
        secondary,
        ignorable,
    }
}

/// The observed CP1252 sort weights, sorted by code point.
static WEIGHTS: &[CharWeight] = &[
    w('\u{0020}', &[0x07], 1, 0x02, None),
    w('\u{0021}', &[0x09], 1, 0x02, None),
    w('\u{0022}', &[0x0a], 1, 0x02, None),
    w('\u{0023}', &[0x0c], 1, 0x02, None),
    w('\u{0024}', &[0x0e], 1, 0x02, None),
    w('\u{0025}', &[0x10], 1, 0x02, None),
    w('\u{0026}', &[0x12], 1, 0x02, None),
    w('\u{0027}', &[], 0, 0x02, Some(0x80)), // ignorable
    w('\u{0028}', &[0x14], 1, 0x02, None),
    w('\u{0029}', &[0x16], 1, 0x02, None),
    w('\u{002A}', &[0x18], 1, 0x02, None),
    w('\u{002B}', &[0x2c], 1, 0x02, None),
    w('\u{002C}', &[0x1a], 1, 0x02, None),
    w('\u{002D}', &[], 0, 0x02, Some(0x82)), // ignorable
    w('\u{002E}', &[0x1c], 1, 0x02, None),
    w('\u{002F}', &[0x1e], 1, 0x02, None),
    w('\u{0030}', &[0x36], 1, 0x02, None),
    w('\u{0031}', &[0x38], 1, 0x02, None),
    w('\u{0032}', &[0x3a], 1, 0x02, None),
    w('\u{0033}', &[0x3c], 1, 0x02, None),
    w('\u{0034}', &[0x3e], 1, 0x02, None),
    w('\u{0035}', &[0x40], 1, 0x02, None),
    w('\u{0036}', &[0x42], 1, 0x02, None),
    w('\u{0037}', &[0x44], 1, 0x02, None),
    w('\u{0038}', &[0x46], 1, 0x02, None),
    w('\u{0039}', &[0x48], 1, 0x02, None),
    w('\u{003A}', &[0x20], 1, 0x02, None),
    w('\u{003B}', &[0x22], 1, 0x02, None),
    w('\u{003C}', &[0x2e], 1, 0x02, None),
    w('\u{003D}', &[0x30], 1, 0x02, None),
    w('\u{003E}', &[0x32], 1, 0x02, None),
    w('\u{003F}', &[0x24], 1, 0x02, None),
    w('\u{0040}', &[0x26], 1, 0x02, None),
    w('\u{0041}', &[0x4a], 1, 0x02, None),
    w('\u{0042}', &[0x4c], 1, 0x02, None),
    w('\u{0043}', &[0x4d], 1, 0x02, None),
    w('\u{0044}', &[0x4f], 1, 0x02, None),
    w('\u{0045}', &[0x51], 1, 0x02, None),
    w('\u{0046}', &[0x53], 1, 0x02, None),
    w('\u{0047}', &[0x55], 1, 0x02, None),
    w('\u{0048}', &[0x57], 1, 0x02, None),
    w('\u{0049}', &[0x59], 1, 0x02, None),
    w('\u{004A}', &[0x5b], 1, 0x02, None),
    w('\u{004B}', &[0x5c], 1, 0x02, None),
    w('\u{004C}', &[0x5e], 1, 0x02, None),
    w('\u{004D}', &[0x60], 1, 0x02, None),
    w('\u{004E}', &[0x62], 1, 0x02, None),
    w('\u{004F}', &[0x64], 1, 0x02, None),
    w('\u{0050}', &[0x66], 1, 0x02, None),
    w('\u{0051}', &[0x68], 1, 0x02, None),
    w('\u{0052}', &[0x69], 1, 0x02, None),
    w('\u{0053}', &[0x6b], 1, 0x02, None),
    w('\u{0054}', &[0x6d], 1, 0x02, None),
    w('\u{0055}', &[0x6f], 1, 0x02, None),
    w('\u{0056}', &[0x71], 1, 0x02, None),
    w('\u{0057}', &[0x73], 1, 0x02, None),
    w('\u{0058}', &[0x75], 1, 0x02, None),
    w('\u{0059}', &[0x76], 1, 0x02, None),
    w('\u{005A}', &[0x78], 1, 0x02, None),
    w('\u{005B}', &[0x27], 1, 0x02, None),
    w('\u{005C}', &[0x29], 1, 0x02, None),
    w('\u{005D}', &[0x2a], 1, 0x02, None),
    w('\u{005E}', &[0x2b, 0x02], 1, 0x02, None),
    w('\u{005F}', &[0x2b, 0x03], 1, 0x02, None),
    w('\u{0060}', &[0x2b, 0x07], 1, 0x02, None),
    w('\u{0061}', &[0x4a], 1, 0x02, None),
    w('\u{0062}', &[0x4c], 1, 0x02, None),
    w('\u{0063}', &[0x4d], 1, 0x02, None),
    w('\u{0064}', &[0x4f], 1, 0x02, None),
    w('\u{0065}', &[0x51], 1, 0x02, None),
    w('\u{0066}', &[0x53], 1, 0x02, None),
    w('\u{0067}', &[0x55], 1, 0x02, None),
    w('\u{0068}', &[0x57], 1, 0x02, None),
    w('\u{0069}', &[0x59], 1, 0x02, None),
    w('\u{006A}', &[0x5b], 1, 0x02, None),
    w('\u{006B}', &[0x5c], 1, 0x02, None),
    w('\u{006C}', &[0x5e], 1, 0x02, None),
    w('\u{006D}', &[0x60], 1, 0x02, None),
    w('\u{006E}', &[0x62], 1, 0x02, None),
    w('\u{006F}', &[0x64], 1, 0x02, None),
    w('\u{0070}', &[0x66], 1, 0x02, None),
    w('\u{0071}', &[0x68], 1, 0x02, None),
    w('\u{0072}', &[0x69], 1, 0x02, None),
    w('\u{0073}', &[0x6b], 1, 0x02, None),
    w('\u{0074}', &[0x6d], 1, 0x02, None),
    w('\u{0075}', &[0x6f], 1, 0x02, None),
    w('\u{0076}', &[0x71], 1, 0x02, None),
    w('\u{0077}', &[0x73], 1, 0x02, None),
    w('\u{0078}', &[0x75], 1, 0x02, None),
    w('\u{0079}', &[0x76], 1, 0x02, None),
    w('\u{007A}', &[0x78], 1, 0x02, None),
    w('\u{007B}', &[0x2b, 0x09], 1, 0x02, None),
    w('\u{007C}', &[0x2b, 0x0b], 1, 0x02, None),
    w('\u{007D}', &[0x2b, 0x0d], 1, 0x02, None),
    w('\u{007E}', &[0x2b, 0x0f], 1, 0x02, None),
    w('\u{007F}', &[], 0, 0x02, Some(0x1D)), // ignorable
    w('\u{0080}', &[], 0, 0x02, Some(0x1E)), // ignorable
    w('\u{0081}', &[], 0, 0x02, Some(0x1F)), // ignorable
    w('\u{0082}', &[], 0, 0x02, Some(0x20)), // ignorable
    w('\u{0083}', &[], 0, 0x02, Some(0x21)), // ignorable
    w('\u{0084}', &[], 0, 0x02, Some(0x22)), // ignorable
    w('\u{0085}', &[], 0, 0x02, Some(0x23)), // ignorable
    w('\u{0086}', &[], 0, 0x02, Some(0x24)), // ignorable
    w('\u{0087}', &[], 0, 0x02, Some(0x25)), // ignorable
    w('\u{0088}', &[], 0, 0x02, Some(0x26)), // ignorable
    w('\u{0089}', &[], 0, 0x02, Some(0x27)), // ignorable
    w('\u{008A}', &[], 0, 0x02, Some(0x28)), // ignorable
    w('\u{008B}', &[], 0, 0x02, Some(0x29)), // ignorable
    w('\u{008C}', &[], 0, 0x02, Some(0x2A)), // ignorable
    w('\u{008D}', &[], 0, 0x02, Some(0x2B)), // ignorable
    w('\u{008E}', &[], 0, 0x02, Some(0x2C)), // ignorable
    w('\u{008F}', &[], 0, 0x02, Some(0x2D)), // ignorable
    w('\u{0090}', &[], 0, 0x02, Some(0x2E)), // ignorable
    w('\u{0091}', &[], 0, 0x02, Some(0x2F)), // ignorable
    w('\u{0092}', &[], 0, 0x02, Some(0x30)), // ignorable
    w('\u{0093}', &[], 0, 0x02, Some(0x31)), // ignorable
    w('\u{0094}', &[], 0, 0x02, Some(0x32)), // ignorable
    w('\u{0095}', &[], 0, 0x02, Some(0x33)), // ignorable
    w('\u{0096}', &[], 0, 0x02, Some(0x34)), // ignorable
    w('\u{0097}', &[], 0, 0x02, Some(0x35)), // ignorable
    w('\u{0098}', &[], 0, 0x02, Some(0x36)), // ignorable
    w('\u{0099}', &[], 0, 0x02, Some(0x37)), // ignorable
    w('\u{009A}', &[], 0, 0x02, Some(0x38)), // ignorable
    w('\u{009B}', &[], 0, 0x02, Some(0x39)), // ignorable
    w('\u{009C}', &[], 0, 0x02, Some(0x3A)), // ignorable
    w('\u{009D}', &[], 0, 0x02, Some(0x3B)), // ignorable
    w('\u{009E}', &[], 0, 0x02, Some(0x3C)), // ignorable
    w('\u{009F}', &[], 0, 0x02, Some(0x3D)), // ignorable
    w('\u{00A0}', &[0x08, 0x02], 1, 0x02, None),
    w('\u{00A1}', &[0x2b, 0x10], 1, 0x02, None),
    w('\u{00A2}', &[0x34, 0xa6], 1, 0x02, None),
    w('\u{00A3}', &[0x34, 0xa7], 1, 0x02, None),
    w('\u{00A4}', &[0x34, 0xa8], 1, 0x02, None),
    w('\u{00A5}', &[0x34, 0xa9], 1, 0x02, None),
    w('\u{00A6}', &[0x2b, 0x11], 1, 0x02, None),
    w('\u{00A7}', &[0x34, 0xaa], 1, 0x02, None),
    w('\u{00A8}', &[0x2b, 0x12], 1, 0x02, None),
    w('\u{00A9}', &[0x34, 0xab], 1, 0x02, None),
    w('\u{00AA}', &[0x4a], 1, 0x03, None),
    w('\u{00AB}', &[0x33, 0x05], 1, 0x02, None),
    w('\u{00AC}', &[0x34, 0xac], 1, 0x02, None),
    w('\u{00AD}', &[], 0, 0x02, Some(0x83)), // ignorable
    w('\u{00AE}', &[0x34, 0xad], 1, 0x02, None),
    w('\u{00AF}', &[0x2b, 0x13], 1, 0x02, None),
    w('\u{00B0}', &[0x34, 0xae], 1, 0x02, None),
    w('\u{00B1}', &[0x33, 0x04], 1, 0x02, None),
    w('\u{00B2}', &[0x3a], 1, 0x02, None),
    w('\u{00B3}', &[0x3c], 1, 0x02, None),
    w('\u{00B4}', &[0x2b, 0x14], 1, 0x02, None),
    w('\u{00B5}', &[0x34, 0xaf], 1, 0x02, None),
    w('\u{00B6}', &[0x34, 0xb0], 1, 0x02, None),
    w('\u{00B7}', &[0x34, 0xb1], 1, 0x02, None),
    w('\u{00B8}', &[0x2b, 0x15], 1, 0x02, None),
    w('\u{00B9}', &[0x38], 1, 0x02, None),
    w('\u{00BA}', &[0x64], 1, 0x03, None),
    w('\u{00BB}', &[0x33, 0x07], 1, 0x02, None),
    w('\u{00BC}', &[0x37, 0x12], 1, 0x02, None),
    w('\u{00BD}', &[0x37, 0x16], 1, 0x02, None),
    w('\u{00BE}', &[0x37, 0x1a], 1, 0x02, None),
    w('\u{00BF}', &[0x2b, 0x16], 1, 0x02, None),
    w('\u{00C0}', &[0x4a], 1, 0x0F, None),
    w('\u{00C1}', &[0x4a], 1, 0x0E, None),
    w('\u{00C2}', &[0x4a], 1, 0x12, None),
    w('\u{00C3}', &[0x4a], 1, 0x19, None),
    w('\u{00C4}', &[0x4a], 1, 0x13, None),
    w('\u{00C5}', &[0x4a], 1, 0x1A, None),
    w('\u{00C6}', &[0x4a, 0x51], 2, 0x02, None),
    w('\u{00C7}', &[0x4d], 1, 0x1C, None),
    w('\u{00C8}', &[0x51], 1, 0x0F, None),
    w('\u{00C9}', &[0x51], 1, 0x0E, None),
    w('\u{00CA}', &[0x51], 1, 0x12, None),
    w('\u{00CB}', &[0x51], 1, 0x13, None),
    w('\u{00CC}', &[0x59], 1, 0x0F, None),
    w('\u{00CD}', &[0x59], 1, 0x0E, None),
    w('\u{00CE}', &[0x59], 1, 0x12, None),
    w('\u{00CF}', &[0x59], 1, 0x13, None),
    w('\u{00D0}', &[0x4f], 1, 0x68, None),
    w('\u{00D1}', &[0x62], 1, 0x19, None),
    w('\u{00D2}', &[0x64], 1, 0x0F, None),
    w('\u{00D3}', &[0x64], 1, 0x0E, None),
    w('\u{00D4}', &[0x64], 1, 0x12, None),
    w('\u{00D5}', &[0x64], 1, 0x19, None),
    w('\u{00D6}', &[0x64], 1, 0x13, None),
    w('\u{00D7}', &[0x33, 0x09], 1, 0x02, None),
    w('\u{00D8}', &[0x64], 1, 0x21, None),
    w('\u{00D9}', &[0x6f], 1, 0x0F, None),
    w('\u{00DA}', &[0x6f], 1, 0x0E, None),
    w('\u{00DB}', &[0x6f], 1, 0x12, None),
    w('\u{00DC}', &[0x6f], 1, 0x13, None),
    w('\u{00DD}', &[0x76], 1, 0x0E, None),
    w('\u{00DE}', &[0x6d, 0x57], 2, 0x02, None),
    w('\u{00DF}', &[0x6b, 0x6b], 2, 0x02, None),
    w('\u{00E0}', &[0x4a], 1, 0x0F, None),
    w('\u{00E1}', &[0x4a], 1, 0x0E, None),
    w('\u{00E2}', &[0x4a], 1, 0x12, None),
    w('\u{00E3}', &[0x4a], 1, 0x19, None),
    w('\u{00E4}', &[0x4a], 1, 0x13, None),
    w('\u{00E5}', &[0x4a], 1, 0x1A, None),
    w('\u{00E6}', &[0x4a, 0x51], 2, 0x02, None),
    w('\u{00E7}', &[0x4d], 1, 0x1C, None),
    w('\u{00E8}', &[0x51], 1, 0x0F, None),
    w('\u{00E9}', &[0x51], 1, 0x0E, None),
    w('\u{00EA}', &[0x51], 1, 0x12, None),
    w('\u{00EB}', &[0x51], 1, 0x13, None),
    w('\u{00EC}', &[0x59], 1, 0x0F, None),
    w('\u{00ED}', &[0x59], 1, 0x0E, None),
    w('\u{00EE}', &[0x59], 1, 0x12, None),
    w('\u{00EF}', &[0x59], 1, 0x13, None),
    w('\u{00F0}', &[0x4f], 1, 0x68, None),
    w('\u{00F1}', &[0x62], 1, 0x19, None),
    w('\u{00F2}', &[0x64], 1, 0x0F, None),
    w('\u{00F3}', &[0x64], 1, 0x0E, None),
    w('\u{00F4}', &[0x64], 1, 0x12, None),
    w('\u{00F5}', &[0x64], 1, 0x19, None),
    w('\u{00F6}', &[0x64], 1, 0x13, None),
    w('\u{00F7}', &[0x33, 0x0a], 1, 0x02, None),
    w('\u{00F8}', &[0x64], 1, 0x21, None),
    w('\u{00F9}', &[0x6f], 1, 0x0F, None),
    w('\u{00FA}', &[0x6f], 1, 0x0E, None),
    w('\u{00FB}', &[0x6f], 1, 0x12, None),
    w('\u{00FC}', &[0x6f], 1, 0x13, None),
    w('\u{00FD}', &[0x76], 1, 0x0E, None),
    w('\u{00FE}', &[0x6d, 0x57], 2, 0x02, None),
    w('\u{00FF}', &[0x76], 1, 0x13, None),
    w('\u{0152}', &[0x64, 0x51], 2, 0x02, None),
    w('\u{0153}', &[0x64, 0x51], 2, 0x02, None),
    w('\u{0160}', &[0x6b], 1, 0x14, None),
    w('\u{0161}', &[0x6b], 1, 0x14, None),
    w('\u{0178}', &[0x76], 1, 0x13, None),
    w('\u{017D}', &[0x78], 1, 0x14, None),
    w('\u{017E}', &[0x78], 1, 0x14, None),
    w('\u{0192}', &[0x53], 1, 0x7B, None),
    w('\u{02C6}', &[0x2b, 0x02], 1, 0x03, None),
    w('\u{02DC}', &[0x2b, 0x1c], 1, 0x02, None),
    w('\u{2013}', &[], 0, 0x02, Some(0x89)), // ignorable
    w('\u{2014}', &[], 0, 0x02, Some(0x8B)), // ignorable
    w('\u{2018}', &[0x2b, 0x37], 1, 0x02, None),
    w('\u{2019}', &[0x2b, 0x38], 1, 0x02, None),
    w('\u{201A}', &[0x2b, 0x39], 1, 0x02, None),
    w('\u{201C}', &[0x2b, 0x3b], 1, 0x02, None),
    w('\u{201D}', &[0x2b, 0x3c], 1, 0x02, None),
    w('\u{201E}', &[0x2b, 0x3d], 1, 0x02, None),
    w('\u{2020}', &[0x34, 0xb3], 1, 0x02, None),
    w('\u{2021}', &[0x34, 0xb4], 1, 0x02, None),
    w('\u{2022}', &[0x34, 0xb5], 1, 0x02, None),
    w('\u{2026}', &[0x34, 0xb9], 1, 0x02, None),
    w('\u{2030}', &[0x34, 0xba], 1, 0x02, None),
    w('\u{2039}', &[0x2b, 0x46], 1, 0x02, None),
    w('\u{203A}', &[0x2b, 0x47], 1, 0x02, None),
    w('\u{20AC}', &[0x35, 0xa1], 1, 0x02, None),
    w('\u{2122}', &[0x6e, 0x04], 1, 0x02, None),
];

/// Looks one character up in the observed table.
fn lookup(ch: char) -> Option<&'static CharWeight> {
    WEIGHTS
        .binary_search_by_key(&ch, |entry| entry.ch)
        .ok()
        .map(|index| &WEIGHTS[index])
}

/// Builds the engine's sort key for a text value.
///
/// Characters outside the observed CP1252 repertoire answer a precise
/// `Unsupported` naming the code point — a guessed weight would corrupt the
/// engine's B-tree navigation silently.
pub fn text_sort_key(text: &str) -> Result<Vec<u8>, StorageError> {
    let trimmed = text.trim_end_matches(' ');
    let mut primary: Vec<u8> = Vec::with_capacity(trimmed.len() + 8);
    let mut secondary: Vec<u8> = Vec::new();
    let mut ignorables: Vec<(u32, u8)> = Vec::new();
    let mut elements_so_far: u32 = 0;
    for ch in trimmed.chars() {
        let weight = lookup(ch).ok_or_else(|| outside_table(ch))?;
        if let Some(ignorable) = weight.ignorable {
            ignorables.push((elements_so_far, ignorable));
            continue;
        }
        primary.extend_from_slice(weight.primary);
        elements_so_far += u32::from(weight.elements);
        for _ in 0..weight.elements {
            secondary.push(weight.secondary);
        }
    }
    while secondary.last() == Some(&DEFAULT_SECONDARY) {
        secondary.pop();
    }
    let mut level_five: Vec<u8> = Vec::with_capacity(ignorables.len() * 4);
    for (elements, weight) in ignorables {
        let code = elements
            .checked_mul(4)
            .and_then(|value| value.checked_add(7))
            .filter(|value| *value <= MAX_POSITION_CODE)
            .ok_or_else(|| StorageError::Unsupported {
                reason: format!("text sort key: ignorable position {elements} is out of range"),
            })?;
        level_five.push(0x80 | ((code >> 8) as u8));
        level_five.push((code & 0xFF) as u8);
        level_five.push(IGNORABLE_MARKER);
        level_five.push(weight);
    }
    let levels: [&[u8]; 4] = [&secondary, &[], &[], &level_five];
    let mut key = primary;
    match levels.iter().rposition(|level| !level.is_empty()) {
        None => key.extend_from_slice(&[LEVEL_SEPARATOR, KEY_TERMINATOR]),
        Some(last) => {
            for level in &levels[..=last] {
                key.push(LEVEL_SEPARATOR);
                key.extend_from_slice(level);
            }
            key.push(KEY_TERMINATOR);
        }
    }
    Ok(key)
}

/// Precise refusal for a character the observed table does not cover.
fn outside_table(ch: char) -> StorageError {
    StorageError::Unsupported {
        reason: format!(
            "text sort key: character U+{:04X} is outside the observed CP1252              (LANGID 0x0409) collation table",
            ch as u32
        ),
    }
}

#[cfg(test)]
mod tests;
