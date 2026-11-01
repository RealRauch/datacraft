//! VBA project extraction (step 17.2, item L5).
//!
//! The VBA project is extracted, stored and displayed — never executed.
//! DataCraft implements no VBA interpreter or runtime (ADR-2,
//! `docs/ROADMAP.md` §3); the source is migration material for readers,
//! viewers and static analysis.

use serde::{Deserialize, Serialize};

/// Kind of a VBA module as declared by the `dir` stream and the Access
/// naming convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VbaModuleKind {
    /// Procedural (standard) module.
    Standard,
    /// Class module (including `ThisDocument`-style designers).
    Class,
    /// Form class module (`Form_*`).
    Form,
    /// Report class module (`Report_*`).
    Report,
}

/// One decoded VBA module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaModule {
    /// Module name (`MODULENAME`).
    pub name: String,
    /// Module kind.
    pub kind: VbaModuleKind,
    /// Decoded VBA source (code page of the project applied).
    pub source: String,
}

impl VbaModule {
    /// Number of source lines (the snapshot summary).
    pub fn lines(&self) -> usize {
        self.source.lines().count()
    }
}

/// The extracted VBA project of the open database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaProject {
    /// Modules in `dir`-stream order.
    pub modules: Vec<VbaModule>,
}

impl VbaProject {
    /// Case-insensitive module lookup.
    pub fn module(&self, name: &str) -> Option<&VbaModule> {
        self.modules
            .iter()
            .find(|module| module.name.eq_ignore_ascii_case(name))
    }

    /// Whether the project carries no modules (an empty embedded project).
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, source: &str) -> VbaModule {
        VbaModule {
            name: name.to_owned(),
            kind: VbaModuleKind::Standard,
            source: source.to_owned(),
        }
    }

    #[test]
    fn module_lookup_is_case_insensitive_and_lines_count_is_exact() {
        let project = VbaProject {
            modules: vec![module("Utilities", "one\ntwo\n")],
        };
        assert_eq!(project.module("utilities").expect("found").lines(), 2);
        assert!(project.module("missing").is_none());
        assert!(!project.is_empty());
        assert!(VbaProject { modules: vec![] }.is_empty());
    }

    #[test]
    fn module_kinds_serialize_as_snake_case() {
        for (kind, json) in [
            (VbaModuleKind::Standard, "\"standard\""),
            (VbaModuleKind::Class, "\"class\""),
            (VbaModuleKind::Form, "\"form\""),
            (VbaModuleKind::Report, "\"report\""),
        ] {
            assert_eq!(serde_json::to_string(&kind).expect("serializes"), json);
        }
    }
}
