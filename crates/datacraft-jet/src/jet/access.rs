//! `MSysAccessStorage` tree and the named-macro inventory (step 17.1, L4).
//!
//! Access 2007+ keeps the compiled application objects in a tree table:
//! rows are storages (`Type` 1) or streams (`Type` 2), addressed by
//! `ParentId`. Named macros live under the `Scripts` storage: the `DirData`
//! stream lists `(index, name)` pairs and the child storage `<index>`
//! carries the definition stream `Blob`. The definition is decoded as a
//! **coarse** argument list — the readable texts in storage order; action
//! names and execution semantics stay out of scope (ADR-2).
//!
//! Jet 3 keeps the same objects inside `MSysAccessObjects`, whose
//! compound-file chunking is not decoded yet: Jet 3 macro names come from
//! the `MSysObjects` catalog with `definition_decoded = false`.

use crate::error::StorageError;
use crate::jet::reader::{Jet4TableReader, JetTableData};
use crate::types::StorageValue;

/// Object type value of a macro in `MSysObjects` (Jet 3 and ACE).
pub const OBJECT_TYPE_MACRO: i64 = -32766;

/// One row of `MSysAccessStorage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessStorageEntry {
    /// Row id (`Id`).
    pub id: i64,
    /// Parent storage id (`ParentId`).
    pub parent_id: i64,
    /// `Type`: 1 = storage, 2 = stream.
    pub entry_type: i64,
    /// Entry name (`Name`).
    pub name: String,
    /// Stream payload (`Lv`), empty for storages.
    pub data: Vec<u8>,
}

impl AccessStorageEntry {
    /// Whether the row is a stream (carries `Lv` bytes).
    pub fn is_stream(&self) -> bool {
        self.entry_type == AccessStorage::TYPE_STREAM
    }
}

/// One named macro decoded from the `Scripts` storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMacro {
    /// Macro name from the catalog.
    pub name: String,
    /// Readable definition arguments in storage order.
    pub arguments: Vec<String>,
    /// Whether the definition stream was decoded.
    pub definition_decoded: bool,
}

/// The read-only `MSysAccessStorage` tree.
#[derive(Debug, Clone, Default)]
pub struct AccessStorage {
    entries: Vec<AccessStorageEntry>,
}

impl AccessStorage {
    /// `Type` value of a storage node.
    pub const TYPE_STORAGE: i64 = 1;
    /// `Type` value of a stream node.
    pub const TYPE_STREAM: i64 = 2;

    /// Reads the tree; `Ok(None)` when the database has no
    /// `MSysAccessStorage` table (Jet 3, SQLite).
    pub fn read(reader: &Jet4TableReader) -> Result<Option<Self>, StorageError> {
        let Some(page) = reader
            .catalog()?
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("MSysAccessStorage"))
            .and_then(|entry| entry.tdef_page)
        else {
            return Ok(None);
        };
        let table = reader.read_table(page)?;
        Self::from_table(&table).map(Some)
    }

    /// Projects one decoded tree table; split from [`read`](Self::read) so
    /// the malformed-table branches are testable without a file.
    fn from_table(table: &JetTableData) -> Result<Self, StorageError> {
        let index = |name: &str| -> Result<usize, StorageError> {
            table
                .schema
                .columns
                .iter()
                .position(|column| column.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| StorageError::Corrupt {
                    context: "MSysAccessStorage".into(),
                    detail: format!("tree table lacks the {name} column"),
                })
        };
        let (i_id, i_parent, i_type, i_name, i_lv) = (
            index("Id")?,
            index("ParentId")?,
            index("Type")?,
            index("Name")?,
            index("Lv")?,
        );
        let mut entries = Vec::with_capacity(table.rows.len());
        for row in &table.rows {
            let (Some(id), Some(parent_id), Some(entry_type), Some(name)) = (
                int(&row[i_id]),
                int(&row[i_parent]),
                int(&row[i_type]),
                text(&row[i_name]),
            ) else {
                // A row without the tree key columns is not addressable —
                // skip it instead of inventing a node (same rule as the
                // catalog decoder).
                continue;
            };
            let data = match &row[i_lv] {
                StorageValue::Binary(bytes) => bytes.clone(),
                _ => Vec::new(),
            };
            entries.push(AccessStorageEntry {
                id,
                parent_id,
                entry_type,
                name,
                data,
            });
        }
        Ok(Self { entries })
    }

    /// Case-insensitive entry lookup by name.
    pub fn entry(&self, name: &str) -> Option<&AccessStorageEntry> {
        self.entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }

    /// Direct children of a storage node.
    pub fn children(&self, parent_id: i64) -> Vec<&AccessStorageEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.parent_id == parent_id)
            .collect()
    }

    /// Readable arguments of the named macro, or `None` when the definition
    /// stream is absent (the name still exists in the catalog).
    pub fn macro_arguments(&self, name: &str) -> Option<Vec<String>> {
        let scripts = self.entry("Scripts")?;
        if scripts.entry_type != Self::TYPE_STORAGE {
            return None;
        }
        let names = self.script_names(scripts.id);
        let index = names
            .iter()
            .find(|(_, script_name)| script_name.eq_ignore_ascii_case(name))
            .map(|(index, _)| *index)?;
        let storage = self.children(scripts.id).into_iter().find(|child| {
            child.entry_type == Self::TYPE_STORAGE && child.name.parse::<u32>().ok() == Some(index)
        })?;
        let blob = self.blob_of(storage)?;
        Some(decode_arguments(blob))
    }

    /// `(index, name)` pairs from the `Scripts` directory stream.
    fn script_names(&self, scripts_id: i64) -> Vec<(u32, String)> {
        let Some(dir) = self
            .children(scripts_id)
            .into_iter()
            .find(|entry| entry.is_stream() && entry.name.ends_with("DirData"))
        else {
            return Vec::new();
        };
        decode_dir_data(&dir.data)
    }

    /// The `Blob` stream below a script storage (direct child, then any
    /// descendant).
    fn blob_of(&self, storage: &AccessStorageEntry) -> Option<&[u8]> {
        let mut pending = vec![storage.id];
        while let Some(parent) = pending.pop() {
            for child in self.children(parent) {
                if child.is_stream() && child.name.eq_ignore_ascii_case("Blob") {
                    return Some(&child.data);
                }
                if child.entry_type == Self::TYPE_STORAGE {
                    pending.push(child.id);
                }
            }
        }
        None
    }
}

/// Decodes the `Scripts` directory stream: a 4-byte header followed by
/// entries of `tag(1) length(1) name(length-4) index(4)` — UTF-16LE names
/// without a null terminator.
pub fn decode_dir_data(data: &[u8]) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut pos = 4usize;
    while pos + 2 <= data.len() {
        if data[pos] != 0x04 {
            break;
        }
        let length = usize::from(data[pos + 1]);
        if length < 5 || pos + 2 + length > data.len() {
            break;
        }
        let name_end = pos + 2 + length - 4;
        let name_bytes = &data[pos + 2..name_end];
        let index_start = name_end;
        let index = u32::from_le_bytes([
            data[index_start],
            data[index_start + 1],
            data[index_start + 2],
            data[index_start + 3],
        ]);
        if let Some(name) = decode_utf16(name_bytes) {
            out.push((index, name));
        }
        pos += 2 + length;
    }
    out
}

/// Extracts the readable argument texts from a macro definition blob.
///
/// The blob interleaves binary action records with length-tagged UTF-16
/// argument texts. The scan keeps every printable text of at least three
/// characters that is terminated by a null unit, in storage order, and
/// strips the one-byte record tag that precedes some texts (e.g. `7=MsgBox`
/// and `8Delete * from …`). It is deliberately coarse: no action-name table
/// is claimed and nothing is executed.
pub fn decode_arguments(blob: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 1 < blob.len() {
        match read_utf16_run(blob, pos) {
            Some((raw, next)) => {
                if let Some(argument) = normalize_argument(&raw) {
                    out.push(argument);
                }
                pos = next;
            }
            None => pos += 2,
        }
    }
    out
}

/// One null-terminated printable UTF-16LE run starting at an even offset.
fn read_utf16_run(blob: &[u8], start: usize) -> Option<(String, usize)> {
    let mut pos = start;
    let mut text = String::new();
    while pos + 1 < blob.len() {
        let unit = u16::from_le_bytes([blob[pos], blob[pos + 1]]);
        if (0x20..=0x7e).contains(&unit) {
            text.push(char::from_u32(u32::from(unit))?);
            pos += 2;
        } else {
            break;
        }
    }
    if text.chars().count() < 3 {
        return None;
    }
    let terminated = pos + 1 < blob.len() && blob[pos] == 0 && blob[pos + 1] == 0;
    terminated.then_some((text, pos + 2))
}

/// Removes the record tag that precedes some argument texts and drops
/// pure flag values. The tag is a single ASCII alphanumeric byte followed
/// by an expression (`=`), a bracketed name (`[`) or an upper-case word
/// (`Delete`, `MsgBox`, `Not`, …) — normal capitalized words have a
/// lower-case second character and stay untouched.
fn normalize_argument(raw: &str) -> Option<String> {
    let mut text = raw;
    let mut chars = text.chars();
    if let (Some(first), Some(second)) = (chars.next(), chars.next()) {
        let tagged = first.is_ascii_alphanumeric()
            && (second.is_ascii_uppercase() || second == '[' || second == '=');
        if tagged {
            text = &text[first.len_utf8()..];
        }
    }
    // Expression-form arguments carry value-type markers (`<` string
    // literal, `$`, `&`, `"`, `,`) and may carry a record tag byte in front
    // of the marker. Strip up to two markers while the remainder begins
    // with a letter, a bracketed name or another marker; stored argument
    // text never begins with these markers themselves.
    const MARKERS: [char; 6] = ['\'', '"', '$', '&', ',', '<'];
    for _ in 0..2 {
        let mut chars = text.chars();
        if let (Some(first), Some(second)) = (chars.next(), chars.next()) {
            let continues =
                second.is_ascii_alphabetic() || second == '[' || MARKERS.contains(&second);
            if MARKERS.contains(&first) && continues {
                text = &text[first.len_utf8()..];
                continue;
            }
        }
        break;
    }
    let text = text.strip_prefix('=').unwrap_or(text).trim();
    if text.chars().count() < 3 {
        return None;
    }
    // Binary length/tag fields decode to short symbol runs like "*-1" —
    // only texts with letters are readable arguments.
    if !text.chars().any(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(text.to_owned())
}

fn decode_utf16(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    String::from_utf16(&units).ok()
}

fn int(value: &StorageValue) -> Option<i64> {
    match value {
        StorageValue::Int(n) => Some(*n),
        _ => None,
    }
}

fn text(value: &StorageValue) -> Option<String> {
    match value {
        StorageValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
impl AccessStorage {
    /// Test seam: assembles a tree without a database file.
    pub(crate) fn from_entries(entries: Vec<AccessStorageEntry>) -> Self {
        Self { entries }
    }
}

impl Jet4TableReader {
    /// Macros named in the `MSysObjects` catalog, in storage order.
    pub fn macro_names(&self) -> Result<Vec<String>, StorageError> {
        Ok(self
            .catalog()?
            .into_iter()
            .filter(|entry| entry.object_type == OBJECT_TYPE_MACRO)
            .map(|entry| entry.name)
            .collect())
    }

    /// Named macros with their coarse definition arguments (step 17.1).
    pub fn macros(&self) -> Result<Vec<StoredMacro>, StorageError> {
        let names = self.macro_names()?;
        let storage = AccessStorage::read(self)?;
        Ok(names
            .into_iter()
            .map(|name| {
                let arguments = storage
                    .as_ref()
                    .and_then(|tree| tree.macro_arguments(&name));
                stored_macro(name, arguments)
            })
            .collect())
    }
}

/// Pairs a catalog name with its optional decoded arguments.
fn stored_macro(name: String, arguments: Option<Vec<String>>) -> StoredMacro {
    match arguments {
        Some(arguments) => StoredMacro {
            name,
            arguments,
            definition_decoded: true,
        },
        None => StoredMacro {
            name,
            arguments: Vec::new(),
            definition_decoded: false,
        },
    }
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
