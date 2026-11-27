//! The VBA transpilation estimate behind the readiness report's VBA
//! section (phase 103, step 103.1; item M134; roadmap §14).
//!
//! Coarse by design: non-blank, non-comment lines per module; lines
//! touching a blocker marker are not transplicable candidates. The
//! share is a planning number, not a promise.

use std::collections::BTreeMap;

use crate::vba::{VbaModule, VbaModuleKind};

/// Blocker markers of the estimate: WinAPI declares, COM automation,
/// error control flow, host automation and the event model sit outside
/// the transplicable glue subset (roadmap §3).
const BLOCKERS: [&str; 7] = [
    "createobject",
    "declare",
    "getobject",
    "on error",
    "raiseevent",
    "sendkeys",
    "withevents",
];

/// One marker hit of the estimate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkerHit {
    /// The marker name (lower case).
    pub marker: String,
    /// Matching code lines in the module.
    pub count: usize,
}

/// The per-module transpilation estimate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaModuleEstimate {
    /// Module name.
    pub name: String,
    /// Module kind.
    pub kind: VbaModuleKind,
    /// Non-blank, non-comment source lines.
    pub code_lines: usize,
    /// Code lines without a blocker marker — the transplicable
    /// candidates.
    pub candidate_lines: usize,
    /// Markers with their line counts (alphabetical, hits only).
    pub markers: Vec<MarkerHit>,
    /// SQL string literals the module embeds (the 17.3 scan).
    pub sql_literals: usize,
}

/// Estimates one module.
pub(crate) fn estimate_module(module: &VbaModule) -> VbaModuleEstimate {
    let mut code_lines = 0;
    let mut candidates = 0;
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for line in module.source.lines() {
        if !is_code_line(line) {
            continue;
        }
        code_lines += 1;
        let markers = line_markers(line);
        if markers.is_empty() {
            candidates += 1;
        } else {
            for marker in markers {
                *counts.entry(marker).or_insert(0) += 1;
            }
        }
    }
    let sql_literals = crate::vba_analysis::string_literals(&module.source)
        .into_iter()
        .filter(|literal| crate::vba_analysis::looks_like_sql(literal))
        .count();
    VbaModuleEstimate {
        name: module.name.clone(),
        kind: module.kind,
        code_lines,
        candidate_lines: candidates,
        markers: counts
            .into_iter()
            .map(|(marker, count)| MarkerHit {
                marker: marker.to_owned(),
                count,
            })
            .collect(),
        sql_literals,
    }
}

/// The marker hits of one source line (lower case, word-boundary match).
fn line_markers(line: &str) -> Vec<&'static str> {
    let lower = line.to_ascii_lowercase();
    BLOCKERS
        .iter()
        .filter(|marker| contains_word(&lower, marker))
        .copied()
        .collect()
}

/// Whether `needle` occurs in the pre-lowered `text` on word boundaries.
fn contains_word(text: &str, needle: &str) -> bool {
    let mut rest = text;
    while let Some(position) = rest.find(needle) {
        let before = rest[..position].chars().next_back();
        let after = rest[position + needle.len()..].chars().next();
        let bounded = before.is_none_or(|c| !c.is_alphanumeric())
            && after.is_none_or(|c| !c.is_alphanumeric());
        if bounded {
            return true;
        }
        rest = &rest[position + needle.len()..];
    }
    false
}

/// Whether a line counts as code for the estimate (non-blank, no
/// `'`-comment, no `Rem`).
fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('\'')
        && !trimmed.to_ascii_lowercase().starts_with("rem ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_matching_respects_word_boundaries() {
        assert!(contains_word("on error resume next", "on error"));
        assert!(!contains_word("on errorsink", "on error"));
        assert!(contains_word("declare ptrsafe function", "declare"));
        assert!(!contains_word("declared", "declare"));
        assert!(contains_word("set s = createobject(\"x\")", "createobject"));
    }

    #[test]
    fn comment_and_blank_lines_are_not_code() {
        assert!(!is_code_line(""));
        assert!(!is_code_line("   "));
        assert!(!is_code_line("' a comment"));
        assert!(!is_code_line("REM uppercase"));
        assert!(is_code_line("x = 1 ' trailing comment"));
    }

    #[test]
    fn the_estimate_counts_markers_and_sql_literals() {
        let module = VbaModule {
            name: "M".to_owned(),
            kind: VbaModuleKind::Standard,
            source: "a = 1\nOn Error Resume Next\ns = \" SELECT * FROM [T] \"\n".to_owned(),
        };
        let estimate = estimate_module(&module);
        assert_eq!(estimate.code_lines, 3);
        assert_eq!(estimate.candidate_lines, 2);
        assert_eq!(
            estimate.markers,
            vec![MarkerHit {
                marker: "on error".to_owned(),
                count: 1,
            }]
        );
        assert_eq!(estimate.sql_literals, 1);
    }
}
