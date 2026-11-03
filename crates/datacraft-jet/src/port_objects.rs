//! Storage → core mapping for extracted application objects (steps 17.1
//! and 17.2): named macros and the VBA project. No storage type crosses
//! the port boundary.

use datacraft_core::macro_def::{MacroAction, MacroDef};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};

use crate::jet::{StoredMacro, StoredModuleKind, StoredVbaProject};

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
