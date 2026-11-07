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

/// The observed CP1252 sort weights, sorted by code point. One line per
/// entry — the generated table is excluded from rustfmt to stay diffable.
#[rustfmt::skip]
static WEIGHTS: &[CharWeight] = &[
    CharWeight { ch: '\u{0020}', primary: &[0x07], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0021}', primary: &[0x09], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0022}', primary: &[0x0a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0023}', primary: &[0x0c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0024}', primary: &[0x0e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0025}', primary: &[0x10], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0026}', primary: &[0x12], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0027}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x80) }, // ignorable
    CharWeight { ch: '\u{0028}', primary: &[0x14], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0029}', primary: &[0x16], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{002A}', primary: &[0x18], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{002B}', primary: &[0x2c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{002C}', primary: &[0x1a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{002D}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x82) }, // ignorable
    CharWeight { ch: '\u{002E}', primary: &[0x1c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{002F}', primary: &[0x1e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0030}', primary: &[0x36], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0031}', primary: &[0x38], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0032}', primary: &[0x3a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0033}', primary: &[0x3c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0034}', primary: &[0x3e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0035}', primary: &[0x40], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0036}', primary: &[0x42], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0037}', primary: &[0x44], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0038}', primary: &[0x46], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0039}', primary: &[0x48], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003A}', primary: &[0x20], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003B}', primary: &[0x22], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003C}', primary: &[0x2e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003D}', primary: &[0x30], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003E}', primary: &[0x32], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{003F}', primary: &[0x24], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0040}', primary: &[0x26], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0041}', primary: &[0x4a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0042}', primary: &[0x4c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0043}', primary: &[0x4d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0044}', primary: &[0x4f], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0045}', primary: &[0x51], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0046}', primary: &[0x53], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0047}', primary: &[0x55], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0048}', primary: &[0x57], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0049}', primary: &[0x59], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004A}', primary: &[0x5b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004B}', primary: &[0x5c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004C}', primary: &[0x5e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004D}', primary: &[0x60], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004E}', primary: &[0x62], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{004F}', primary: &[0x64], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0050}', primary: &[0x66], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0051}', primary: &[0x68], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0052}', primary: &[0x69], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0053}', primary: &[0x6b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0054}', primary: &[0x6d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0055}', primary: &[0x6f], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0056}', primary: &[0x71], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0057}', primary: &[0x73], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0058}', primary: &[0x75], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0059}', primary: &[0x76], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005A}', primary: &[0x78], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005B}', primary: &[0x27], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005C}', primary: &[0x29], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005D}', primary: &[0x2a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005E}', primary: &[0x2b, 0x02], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{005F}', primary: &[0x2b, 0x03], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0060}', primary: &[0x2b, 0x07], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0061}', primary: &[0x4a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0062}', primary: &[0x4c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0063}', primary: &[0x4d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0064}', primary: &[0x4f], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0065}', primary: &[0x51], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0066}', primary: &[0x53], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0067}', primary: &[0x55], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0068}', primary: &[0x57], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0069}', primary: &[0x59], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006A}', primary: &[0x5b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006B}', primary: &[0x5c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006C}', primary: &[0x5e], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006D}', primary: &[0x60], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006E}', primary: &[0x62], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{006F}', primary: &[0x64], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0070}', primary: &[0x66], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0071}', primary: &[0x68], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0072}', primary: &[0x69], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0073}', primary: &[0x6b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0074}', primary: &[0x6d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0075}', primary: &[0x6f], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0076}', primary: &[0x71], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0077}', primary: &[0x73], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0078}', primary: &[0x75], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0079}', primary: &[0x76], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007A}', primary: &[0x78], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007B}', primary: &[0x2b, 0x09], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007C}', primary: &[0x2b, 0x0b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007D}', primary: &[0x2b, 0x0d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007E}', primary: &[0x2b, 0x0f], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{007F}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x1D) }, // ignorable
    CharWeight { ch: '\u{0080}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x1E) }, // ignorable
    CharWeight { ch: '\u{0081}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x1F) }, // ignorable
    CharWeight { ch: '\u{0082}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x20) }, // ignorable
    CharWeight { ch: '\u{0083}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x21) }, // ignorable
    CharWeight { ch: '\u{0084}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x22) }, // ignorable
    CharWeight { ch: '\u{0085}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x23) }, // ignorable
    CharWeight { ch: '\u{0086}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x24) }, // ignorable
    CharWeight { ch: '\u{0087}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x25) }, // ignorable
    CharWeight { ch: '\u{0088}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x26) }, // ignorable
    CharWeight { ch: '\u{0089}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x27) }, // ignorable
    CharWeight { ch: '\u{008A}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x28) }, // ignorable
    CharWeight { ch: '\u{008B}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x29) }, // ignorable
    CharWeight { ch: '\u{008C}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2A) }, // ignorable
    CharWeight { ch: '\u{008D}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2B) }, // ignorable
    CharWeight { ch: '\u{008E}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2C) }, // ignorable
    CharWeight { ch: '\u{008F}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2D) }, // ignorable
    CharWeight { ch: '\u{0090}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2E) }, // ignorable
    CharWeight { ch: '\u{0091}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x2F) }, // ignorable
    CharWeight { ch: '\u{0092}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x30) }, // ignorable
    CharWeight { ch: '\u{0093}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x31) }, // ignorable
    CharWeight { ch: '\u{0094}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x32) }, // ignorable
    CharWeight { ch: '\u{0095}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x33) }, // ignorable
    CharWeight { ch: '\u{0096}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x34) }, // ignorable
    CharWeight { ch: '\u{0097}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x35) }, // ignorable
    CharWeight { ch: '\u{0098}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x36) }, // ignorable
    CharWeight { ch: '\u{0099}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x37) }, // ignorable
    CharWeight { ch: '\u{009A}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x38) }, // ignorable
    CharWeight { ch: '\u{009B}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x39) }, // ignorable
    CharWeight { ch: '\u{009C}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x3A) }, // ignorable
    CharWeight { ch: '\u{009D}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x3B) }, // ignorable
    CharWeight { ch: '\u{009E}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x3C) }, // ignorable
    CharWeight { ch: '\u{009F}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x3D) }, // ignorable
    CharWeight { ch: '\u{00A0}', primary: &[0x08, 0x02], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A1}', primary: &[0x2b, 0x10], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A2}', primary: &[0x34, 0xa6], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A3}', primary: &[0x34, 0xa7], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A4}', primary: &[0x34, 0xa8], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A5}', primary: &[0x34, 0xa9], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A6}', primary: &[0x2b, 0x11], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A7}', primary: &[0x34, 0xaa], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A8}', primary: &[0x2b, 0x12], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00A9}', primary: &[0x34, 0xab], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00AA}', primary: &[0x4a], elements: 1, secondary: 0x03, ignorable: None },
    CharWeight { ch: '\u{00AB}', primary: &[0x33, 0x05], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00AC}', primary: &[0x34, 0xac], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00AD}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x83) }, // ignorable
    CharWeight { ch: '\u{00AE}', primary: &[0x34, 0xad], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00AF}', primary: &[0x2b, 0x13], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B0}', primary: &[0x34, 0xae], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B1}', primary: &[0x33, 0x04], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B2}', primary: &[0x3a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B3}', primary: &[0x3c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B4}', primary: &[0x2b, 0x14], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B5}', primary: &[0x34, 0xaf], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B6}', primary: &[0x34, 0xb0], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B7}', primary: &[0x34, 0xb1], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B8}', primary: &[0x2b, 0x15], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00B9}', primary: &[0x38], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00BA}', primary: &[0x64], elements: 1, secondary: 0x03, ignorable: None },
    CharWeight { ch: '\u{00BB}', primary: &[0x33, 0x07], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00BC}', primary: &[0x37, 0x12], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00BD}', primary: &[0x37, 0x16], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00BE}', primary: &[0x37, 0x1a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00BF}', primary: &[0x2b, 0x16], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00C0}', primary: &[0x4a], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00C1}', primary: &[0x4a], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00C2}', primary: &[0x4a], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00C3}', primary: &[0x4a], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00C4}', primary: &[0x4a], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00C5}', primary: &[0x4a], elements: 1, secondary: 0x1A, ignorable: None },
    CharWeight { ch: '\u{00C6}', primary: &[0x4a, 0x51], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00C7}', primary: &[0x4d], elements: 1, secondary: 0x1C, ignorable: None },
    CharWeight { ch: '\u{00C8}', primary: &[0x51], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00C9}', primary: &[0x51], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00CA}', primary: &[0x51], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00CB}', primary: &[0x51], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00CC}', primary: &[0x59], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00CD}', primary: &[0x59], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00CE}', primary: &[0x59], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00CF}', primary: &[0x59], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00D0}', primary: &[0x4f], elements: 1, secondary: 0x68, ignorable: None },
    CharWeight { ch: '\u{00D1}', primary: &[0x62], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00D2}', primary: &[0x64], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00D3}', primary: &[0x64], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00D4}', primary: &[0x64], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00D5}', primary: &[0x64], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00D6}', primary: &[0x64], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00D7}', primary: &[0x33, 0x09], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00D8}', primary: &[0x64], elements: 1, secondary: 0x21, ignorable: None },
    CharWeight { ch: '\u{00D9}', primary: &[0x6f], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00DA}', primary: &[0x6f], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00DB}', primary: &[0x6f], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00DC}', primary: &[0x6f], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00DD}', primary: &[0x76], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00DE}', primary: &[0x6d, 0x57], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00DF}', primary: &[0x6b, 0x6b], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00E0}', primary: &[0x4a], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00E1}', primary: &[0x4a], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00E2}', primary: &[0x4a], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00E3}', primary: &[0x4a], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00E4}', primary: &[0x4a], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00E5}', primary: &[0x4a], elements: 1, secondary: 0x1A, ignorable: None },
    CharWeight { ch: '\u{00E6}', primary: &[0x4a, 0x51], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00E7}', primary: &[0x4d], elements: 1, secondary: 0x1C, ignorable: None },
    CharWeight { ch: '\u{00E8}', primary: &[0x51], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00E9}', primary: &[0x51], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00EA}', primary: &[0x51], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00EB}', primary: &[0x51], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00EC}', primary: &[0x59], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00ED}', primary: &[0x59], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00EE}', primary: &[0x59], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00EF}', primary: &[0x59], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00F0}', primary: &[0x4f], elements: 1, secondary: 0x68, ignorable: None },
    CharWeight { ch: '\u{00F1}', primary: &[0x62], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00F2}', primary: &[0x64], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00F3}', primary: &[0x64], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00F4}', primary: &[0x64], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00F5}', primary: &[0x64], elements: 1, secondary: 0x19, ignorable: None },
    CharWeight { ch: '\u{00F6}', primary: &[0x64], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00F7}', primary: &[0x33, 0x0a], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00F8}', primary: &[0x64], elements: 1, secondary: 0x21, ignorable: None },
    CharWeight { ch: '\u{00F9}', primary: &[0x6f], elements: 1, secondary: 0x0F, ignorable: None },
    CharWeight { ch: '\u{00FA}', primary: &[0x6f], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00FB}', primary: &[0x6f], elements: 1, secondary: 0x12, ignorable: None },
    CharWeight { ch: '\u{00FC}', primary: &[0x6f], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{00FD}', primary: &[0x76], elements: 1, secondary: 0x0E, ignorable: None },
    CharWeight { ch: '\u{00FE}', primary: &[0x6d, 0x57], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{00FF}', primary: &[0x76], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{0152}', primary: &[0x64, 0x51], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0153}', primary: &[0x64, 0x51], elements: 2, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{0160}', primary: &[0x6b], elements: 1, secondary: 0x14, ignorable: None },
    CharWeight { ch: '\u{0161}', primary: &[0x6b], elements: 1, secondary: 0x14, ignorable: None },
    CharWeight { ch: '\u{0178}', primary: &[0x76], elements: 1, secondary: 0x13, ignorable: None },
    CharWeight { ch: '\u{017D}', primary: &[0x78], elements: 1, secondary: 0x14, ignorable: None },
    CharWeight { ch: '\u{017E}', primary: &[0x78], elements: 1, secondary: 0x14, ignorable: None },
    CharWeight { ch: '\u{0192}', primary: &[0x53], elements: 1, secondary: 0x7B, ignorable: None },
    CharWeight { ch: '\u{02C6}', primary: &[0x2b, 0x02], elements: 1, secondary: 0x03, ignorable: None },
    CharWeight { ch: '\u{02DC}', primary: &[0x2b, 0x1c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2013}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x89) }, // ignorable
    CharWeight { ch: '\u{2014}', primary: &[], elements: 0, secondary: 0x02, ignorable: Some(0x8B) }, // ignorable
    CharWeight { ch: '\u{2018}', primary: &[0x2b, 0x37], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2019}', primary: &[0x2b, 0x38], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{201A}', primary: &[0x2b, 0x39], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{201C}', primary: &[0x2b, 0x3b], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{201D}', primary: &[0x2b, 0x3c], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{201E}', primary: &[0x2b, 0x3d], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2020}', primary: &[0x34, 0xb3], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2021}', primary: &[0x34, 0xb4], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2022}', primary: &[0x34, 0xb5], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2026}', primary: &[0x34, 0xb9], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2030}', primary: &[0x34, 0xba], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2039}', primary: &[0x2b, 0x46], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{203A}', primary: &[0x2b, 0x47], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{20AC}', primary: &[0x35, 0xa1], elements: 1, secondary: 0x02, ignorable: None },
    CharWeight { ch: '\u{2122}', primary: &[0x6e, 0x04], elements: 1, secondary: 0x02, ignorable: None },
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
