//! Nav-section entry lists (step 92.5's file-size split, item L30): the
//! category-tree sections and the flat details list, built from the
//! snapshot. Pure data — the paint and interaction stay in
//! `nav_pane.rs`.

use datacraft_core::ObjectKind;
use datacraft_core::snapshot::DatabaseSnapshot;

use crate::nav_pane::{
    SECTION_DETAILS, SECTION_FORMS, SECTION_MACROS, SECTION_MODULES, SECTION_QUERIES,
    SECTION_REPORTS, SECTION_TABLES, SECTION_VIEWS,
};

/// One section: its header key (an id-salt prefix and a catalog key) and
/// its entries in snapshot order (the pane sorts them).
pub(crate) type NavSection = (&'static str, Vec<(String, ObjectKind)>);

/// The category-tree sections in pane order.
pub(crate) fn tree_sections(db: &DatabaseSnapshot) -> Vec<NavSection> {
    vec![
        (
            SECTION_TABLES,
            db.tables
                .iter()
                .map(|t| (t.name.clone(), ObjectKind::Table))
                .collect(),
        ),
        (
            SECTION_QUERIES,
            db.queries
                .iter()
                .map(|q| (q.name.clone(), ObjectKind::Query))
                .collect(),
        ),
        (
            SECTION_VIEWS,
            db.views
                .iter()
                .map(|v| (v.name.clone(), ObjectKind::View))
                .chain(
                    db.native_views
                        .iter()
                        .map(|v| (v.name.clone(), ObjectKind::View)),
                )
                .collect(),
        ),
        (
            SECTION_FORMS,
            db.forms
                .iter()
                .map(|f| (f.name.clone(), ObjectKind::Form))
                .collect(),
        ),
        (
            SECTION_REPORTS,
            db.reports
                .iter()
                .map(|r| (r.name.clone(), ObjectKind::Report))
                .collect(),
        ),
        (
            SECTION_MACROS,
            db.macros
                .iter()
                .map(|m| (m.name.clone(), ObjectKind::Macro))
                .collect(),
        ),
        (
            SECTION_MODULES,
            db.vba
                .as_ref()
                .map(|project| {
                    project
                        .modules
                        .iter()
                        .map(|m| (m.name.clone(), ObjectKind::Module))
                        .collect()
                })
                .unwrap_or_default(),
        ),
    ]
}

/// The details view's single flat section: every object across the
/// categories (step 92.5, item L48).
pub(crate) fn details_sections(db: &DatabaseSnapshot) -> Vec<NavSection> {
    vec![(
        SECTION_DETAILS,
        db.tables
            .iter()
            .map(|t| (t.name.clone(), ObjectKind::Table))
            .chain(
                db.queries
                    .iter()
                    .map(|q| (q.name.clone(), ObjectKind::Query)),
            )
            .chain(db.views.iter().map(|v| (v.name.clone(), ObjectKind::View)))
            .chain(
                db.native_views
                    .iter()
                    .map(|v| (v.name.clone(), ObjectKind::View)),
            )
            .chain(db.forms.iter().map(|f| (f.name.clone(), ObjectKind::Form)))
            .chain(
                db.reports
                    .iter()
                    .map(|r| (r.name.clone(), ObjectKind::Report)),
            )
            .chain(
                db.macros
                    .iter()
                    .map(|m| (m.name.clone(), ObjectKind::Macro)),
            )
            .chain(
                db.vba
                    .as_ref()
                    .map(|project| {
                        project
                            .modules
                            .iter()
                            .map(|m| (m.name.clone(), ObjectKind::Module))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
            .collect(),
    )]
}
