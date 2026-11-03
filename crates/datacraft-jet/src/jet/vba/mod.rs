//! VBA project assembly from `MSysAccessStorage` (step 17.2, item L5).
//!
//! Layout (observed on real Access files, [MS-OVBA] §2.2.2):
//! `VBA/VBAProject/VBA` holds the `dir` and `_VBA_PROJECT` streams plus one
//! stream per module. The `dir` stream names each module, names its stream
//! and carries the source offset and module type.
//!
//! Extraction is read-only by design: DataCraft never executes VBA
//! (ADR-2, `docs/ROADMAP.md` §3).

pub mod decompress;
pub mod dir;
pub mod source;

use crate::error::StorageError;
use crate::jet::access::{AccessStorage, AccessStorageEntry};

/// Kind of an extracted module (storage vocabulary; mapped onto the core
/// domain by the port adapter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredModuleKind {
    /// Procedural (standard) module.
    Standard,
    /// Class module.
    Class,
    /// Form class module (`Form_*`).
    Form,
    /// Report class module (`Report_*`).
    Report,
}

/// One extracted VBA module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredVbaModule {
    /// Module name from the `dir` stream.
    pub name: String,
    /// Stream that holds the module.
    pub stream_name: String,
    /// Module kind.
    pub kind: StoredModuleKind,
    /// Decoded source.
    pub source: String,
}

/// The extracted VBA project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredVbaProject {
    /// Modules in `dir`-stream order.
    pub modules: Vec<StoredVbaModule>,
}

impl crate::jet::reader::Jet4TableReader {
    /// Extracts the VBA project; `Ok(None)` for databases without a VBA
    /// storage (Jet 3 keeps its project in `MSysAccessObjects`, which is
    /// not decoded yet — BACKLOG remainder).
    pub fn vba_project(&self) -> Result<Option<StoredVbaProject>, StorageError> {
        let Some(storage) = AccessStorage::read(self)? else {
            return Ok(None);
        };
        read_project(&storage)
    }
}

/// Reads the project from an already decoded storage tree.
pub fn read_project(storage: &AccessStorage) -> Result<Option<StoredVbaProject>, StorageError> {
    let Some(root) = storage
        .entry("VBA")
        .filter(|entry| entry.entry_type == AccessStorage::TYPE_STORAGE)
    else {
        return Ok(None);
    };
    let project = find_storage(storage, root.id, "VBAProject").unwrap_or(root);
    let streams_parent = child_storage(storage, project.id, "VBA").unwrap_or(project);
    let streams: Vec<&AccessStorageEntry> = storage
        .children(streams_parent.id)
        .into_iter()
        .filter(|entry| entry.is_stream())
        .collect();
    let Some(dir_entry) = streams
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case("dir"))
    else {
        return Err(corrupt("VBA storage lacks the dir stream"));
    };
    let decompressed = decompress::decompress(&dir_entry.data)?;
    let info = dir::parse_dir(&decompressed)?;
    let mut modules = Vec::with_capacity(info.modules.len());
    for module in info.modules {
        let Some(stream) = streams
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(&module.stream_name))
        else {
            return Err(corrupt(format!(
                "module {:?} names stream {:?}, which is absent",
                module.name, module.stream_name
            )));
        };
        let source = source::decode_source(&stream.data, module.offset, info.code_page)?;
        let kind = module_kind(&module.name, module.is_procedural());
        modules.push(StoredVbaModule {
            name: module.name,
            stream_name: module.stream_name,
            kind,
            source,
        });
    }
    Ok(Some(StoredVbaProject { modules }))
}

/// First storage named `name` at or below `parent`.
fn find_storage<'a>(
    storage: &'a AccessStorage,
    parent: i64,
    name: &str,
) -> Option<&'a AccessStorageEntry> {
    let mut pending = vec![parent];
    while let Some(id) = pending.pop() {
        for child in storage.children(id) {
            if child.entry_type != AccessStorage::TYPE_STORAGE {
                continue;
            }
            if child.name.eq_ignore_ascii_case(name) {
                return Some(child);
            }
            pending.push(child.id);
        }
    }
    None
}

/// Direct storage child named `name`.
fn child_storage<'a>(
    storage: &'a AccessStorage,
    parent: i64,
    name: &str,
) -> Option<&'a AccessStorageEntry> {
    storage.children(parent).into_iter().find(|entry| {
        entry.entry_type == AccessStorage::TYPE_STORAGE && entry.name.eq_ignore_ascii_case(name)
    })
}

/// Maps the `dir` type and the Access naming convention onto the kind.
fn module_kind(name: &str, procedural: bool) -> StoredModuleKind {
    if procedural {
        return StoredModuleKind::Standard;
    }
    if name.starts_with("Form_") {
        StoredModuleKind::Form
    } else if name.starts_with("Report_") {
        StoredModuleKind::Report
    } else {
        StoredModuleKind::Class
    }
}

fn corrupt(detail: impl Into<String>) -> StorageError {
    StorageError::Corrupt {
        context: "VBA project".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jet::access::AccessStorageEntry;

    fn entry(
        id: i64,
        parent_id: i64,
        entry_type: i64,
        name: &str,
        data: &[u8],
    ) -> AccessStorageEntry {
        AccessStorageEntry {
            id,
            parent_id,
            entry_type,
            name: name.to_owned(),
            data: data.to_vec(),
        }
    }

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

    fn dir_record(id: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn project_dir(modules: &[(&str, &str, u16)]) -> Vec<u8> {
        let mut dir = dir_record(0x0003, &1252u16.to_le_bytes());
        dir.extend(dir_record(0x000F, &(modules.len() as u16).to_le_bytes()));
        for (name, stream, kind) in modules {
            dir.extend(dir_record(0x0019, name.as_bytes()));
            dir.extend(dir_record(0x001A, stream.as_bytes()));
            dir.extend(dir_record(0x0031, &0u32.to_le_bytes()));
            dir.extend(dir_record(*kind, &[]));
            dir.extend(dir_record(0x002B, &[]));
        }
        dir
    }

    fn tree(dir: &[u8], streams: &[(&str, &[u8])]) -> AccessStorage {
        let mut entries = vec![
            entry(3, 1, AccessStorage::TYPE_STORAGE, "VBA", &[]),
            entry(4, 3, AccessStorage::TYPE_STORAGE, "VBAProject", &[]),
            entry(5, 4, AccessStorage::TYPE_STORAGE, "VBA", &[]),
            entry(
                6,
                5,
                AccessStorage::TYPE_STREAM,
                "dir",
                &literal_container(dir),
            ),
        ];
        for (id, (name, data)) in (7i64..).zip(streams.iter()) {
            entries.push(entry(id, 5, AccessStorage::TYPE_STREAM, name, data));
        }
        AccessStorage::from_entries(entries)
    }

    #[test]
    fn a_project_with_standard_and_designer_modules_decodes() {
        let dir = project_dir(&[
            ("Utilities", "S1", 0x0021),
            ("Form_Sales", "S2", 0x0022),
            ("Report_Sales", "S3", 0x0022),
            ("ErrorHandling", "S4", 0x0022),
        ]);
        let storage = tree(
            &dir,
            &[
                (
                    "S1",
                    &literal_container(b"Attribute VB_Name = \"Utilities\"\n"),
                ),
                (
                    "S2",
                    &literal_container(b"Attribute VB_Name = \"Form_Sales\"\n"),
                ),
                (
                    "S3",
                    &literal_container(b"Attribute VB_Name = \"Report_Sales\"\n"),
                ),
                (
                    "S4",
                    &literal_container(b"Attribute VB_Name = \"ErrorHandling\"\n"),
                ),
            ],
        );
        let project = read_project(&storage).expect("decodes").expect("project");
        let kinds: Vec<StoredModuleKind> = project.modules.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            vec![
                StoredModuleKind::Standard,
                StoredModuleKind::Form,
                StoredModuleKind::Report,
                StoredModuleKind::Class,
            ]
        );
        assert!(project.modules[0].source.contains("Utilities"));
    }

    #[test]
    fn missing_project_parts_report_precise_errors() {
        assert!(
            read_project(&AccessStorage::from_entries(Vec::new()))
                .expect("no VBA root is not an error")
                .is_none()
        );

        let no_dir = AccessStorage::from_entries(vec![
            entry(3, 1, AccessStorage::TYPE_STORAGE, "VBA", &[]),
            entry(4, 3, AccessStorage::TYPE_STORAGE, "VBAProject", &[]),
        ]);
        assert!(
            matches!(read_project(&no_dir), Err(StorageError::Corrupt { detail, .. }) if detail.contains("dir"))
        );

        let dir = project_dir(&[("Utilities", "Missing", 0x0021)]);
        let missing_stream = tree(&dir, &[]);
        assert!(
            matches!(read_project(&missing_stream), Err(StorageError::Corrupt { detail, .. }) if detail.contains("Missing"))
        );

        // A VBA node that is a stream is not a project.
        let stream_root =
            AccessStorage::from_entries(vec![entry(3, 1, AccessStorage::TYPE_STREAM, "VBA", &[])]);
        assert!(read_project(&stream_root).expect("not a project").is_none());
    }

    #[test]
    fn module_kind_falls_back_to_the_access_name_convention() {
        assert_eq!(module_kind("X", true), StoredModuleKind::Standard);
        assert_eq!(module_kind("Form_X", false), StoredModuleKind::Form);
        assert_eq!(module_kind("Report_X", false), StoredModuleKind::Report);
        assert_eq!(module_kind("ClassX", false), StoredModuleKind::Class);
    }
}
