//! Storage → core mapping for extracted application objects (steps 17.1
//! and 17.2): named macros and the VBA project. No storage type crosses
//! the port boundary.

use datacraft_core::macro_def::{MacroAction, MacroDef};
use datacraft_core::port::{LinkKind, LinkedTable, NativeObject, NativeObjectKind};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};

use crate::jet::{
    CatalogEntry, OBJECT_TYPE_FORM, OBJECT_TYPE_LINK_ACCESS, OBJECT_TYPE_LINK_ODBC,
    OBJECT_TYPE_REPORT, StoredMacro, StoredModuleKind, StoredVbaProject,
};

/// Maps the macro inventory onto the core domain.
pub(crate) fn macros_to_core(stored: Vec<StoredMacro>) -> Vec<MacroDef> {
    stored
        .into_iter()
        .map(|macro_def| MacroDef {
            name: macro_def.name,
            actions: macro_def
                .arguments
                .into_iter()
                .map(|argument| MacroAction { argument })
                .collect(),
            definition_decoded: macro_def.definition_decoded,
        })
        .collect()
}

/// Maps the catalog's native form/report object types onto the core
/// inventory (phase 74, step 74.1; item M87). Names only — no definition
/// bytes are touched.
pub(crate) fn native_objects_to_core(entries: Vec<CatalogEntry>) -> Vec<NativeObject> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let kind = match entry.object_type {
                OBJECT_TYPE_FORM => NativeObjectKind::Form,
                OBJECT_TYPE_REPORT => NativeObjectKind::Report,
                _ => return None,
            };
            (!entry.name.starts_with("MSys")).then_some(NativeObject {
                name: entry.name,
                kind,
            })
        })
        .collect()
}

/// Maps the catalog's link records onto the core inventory (phase 100,
/// step 100.1; item M131): the kind, the external file or the connect
/// string, the foreign name and the flags. A connect string's credentials
/// are redacted here — the one place that touches them — so no caller can
/// leak a password into the UI, the wire or a log.
pub(crate) fn linked_tables_to_core(entries: Vec<CatalogEntry>) -> Vec<LinkedTable> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let kind = match entry.object_type {
                OBJECT_TYPE_LINK_ODBC => LinkKind::Odbc,
                OBJECT_TYPE_LINK_ACCESS => LinkKind::AccessFile,
                _ => return None,
            };
            Some(LinkedTable {
                name: entry.name,
                kind,
                source: entry.database,
                connect: entry.connect.as_deref().map(redact_connect),
                foreign_name: entry.foreign_name,
                flags: entry.flags,
            })
        })
        .collect()
}

/// Replaces the value of every credential fragment in an ODBC connect
/// string (`PWD=`, `UID=` stays readable — only the secret is wiped) with
/// `***`, keeping the rest of the string intact and its order stable.
fn redact_connect(connect: &str) -> String {
    const SECRET_KEYS: &[&str] = &["PWD", "PASSWORD"];
    let mut redacted = String::with_capacity(connect.len());
    for (index, fragment) in connect.split(';').enumerate() {
        if index > 0 {
            redacted.push(';');
        }
        let secret = fragment.split_once('=').is_some_and(|(key, _)| {
            SECRET_KEYS
                .iter()
                .any(|candidate| key.trim().eq_ignore_ascii_case(candidate))
        });
        if secret {
            let key = fragment.split_once('=').map_or("", |(key, _)| key);
            redacted.push_str(key);
            redacted.push_str("=***");
        } else {
            redacted.push_str(fragment);
        }
    }
    redacted
}

/// Maps the VBA project onto the core domain.
pub(crate) fn vba_to_core(project: StoredVbaProject) -> VbaProject {
    VbaProject {
        modules: project
            .modules
            .into_iter()
            .map(|module| VbaModule {
                name: module.name,
                kind: match module.kind {
                    StoredModuleKind::Standard => VbaModuleKind::Standard,
                    StoredModuleKind::Class => VbaModuleKind::Class,
                    StoredModuleKind::Form => VbaModuleKind::Form,
                    StoredModuleKind::Report => VbaModuleKind::Report,
                },
                source: module.source,
            })
            .collect(),
    }
}
