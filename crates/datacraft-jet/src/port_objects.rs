//! Storage → core mapping for extracted application objects (steps 17.1
//! and 17.2): named macros and the VBA project. No storage type crosses
//! the port boundary.

use datacraft_core::macro_def::{MacroAction, MacroDef};
use datacraft_core::port::{NativeObject, NativeObjectKind};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};

use crate::jet::{
    CatalogEntry, OBJECT_TYPE_FORM, OBJECT_TYPE_REPORT, StoredMacro, StoredModuleKind,
    StoredVbaProject,
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
