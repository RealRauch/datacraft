//! Navigation-pane ordering (step 21.5, item U9).
//!
//! A pure ordering function over the snapshot-derived object list: the
//! pane's group sort menu applies it, so the result is deterministic and
//! testable without a window.

use crate::ObjectKind;

/// Sort key of a navigation-pane group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavSortOrder {
    /// Case-insensitive object name (the Access default).
    #[default]
    Name,
    /// Canonical Access object-kind order, then name.
    Type,
}

/// Canonical Access object-kind sequence (tables, queries, forms, reports,
/// macros, modules).
fn kind_rank(kind: ObjectKind) -> u8 {
    match kind {
        ObjectKind::Table => 0,
        ObjectKind::Query => 1,
        ObjectKind::Form => 2,
        ObjectKind::Report => 3,
        ObjectKind::Macro => 4,
        ObjectKind::Module => 5,
    }
}

/// Orders a snapshot-derived `(name, kind)` list deterministically. Name
/// order is case-insensitive; ties break on the original casing and then
/// on the kind, so the ordering is total and idempotent.
pub fn sort_entries(entries: &mut [(String, ObjectKind)], order: NavSortOrder) {
    entries.sort_by(|left, right| {
        let by_name = || {
            left.0
                .to_lowercase()
                .cmp(&right.0.to_lowercase())
                .then_with(|| left.0.cmp(&right.0))
        };
        match order {
            NavSortOrder::Name => {
                by_name().then_with(|| kind_rank(left.1).cmp(&kind_rank(right.1)))
            }
            NavSortOrder::Type => kind_rank(left.1)
                .cmp(&kind_rank(right.1))
                .then_with(by_name),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The type rank orders every object kind, including the last two
    /// ranks (macros and modules) that the pane shows after reports.
    #[test]
    fn sort_entries_ranks_every_kind() {
        let mut entries = vec![
            ("zap".to_owned(), ObjectKind::Module),
            ("mac".to_owned(), ObjectKind::Macro),
            ("tab".to_owned(), ObjectKind::Table),
            ("frm".to_owned(), ObjectKind::Form),
            ("rpt".to_owned(), ObjectKind::Report),
            ("qry".to_owned(), ObjectKind::Query),
        ];
        sort_entries(&mut entries, NavSortOrder::Type);
        let kinds: Vec<ObjectKind> = entries.into_iter().map(|(_, kind)| kind).collect();
        assert_eq!(
            kinds,
            vec![
                ObjectKind::Table,
                ObjectKind::Query,
                ObjectKind::Form,
                ObjectKind::Report,
                ObjectKind::Macro,
                ObjectKind::Module,
            ]
        );
    }
}
