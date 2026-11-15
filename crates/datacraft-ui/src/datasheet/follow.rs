//! Foreign-key follow (step 87.1, item M103): the datasheet's follow
//! affordance — per-column target labels for the grid's context menu
//! and the dispatch path behind a follow intent. Resolution and the
//! criteria encoding live in core `fk_nav` (pure input, deterministic
//! order); this module only maps snapshot data onto them and dispatches
//! the open/filter pair.

use datacraft_core::command::Command;
use datacraft_core::fk_nav::{self, FkDirection};
use datacraft_core::schema::{FieldDef, FieldValue, TableDef};
use datacraft_core::snapshot::TabSnapshot;

use crate::app::{App, ReferencedCount};
use crate::i18n::tr;

/// The per-column follow labels (step 87.1, item M103): forward targets
/// first, then the inferred reference, then reverse referenced-by.
/// Empty everywhere where the driver declares no relationship support —
/// the grid hides the follow affordance for those columns.
pub(crate) fn follow_labels(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
) -> Vec<Vec<String>> {
    if follow_restricted(app) {
        return Vec::new();
    }
    let snapshot = app.snapshot();
    let Some(database) = snapshot.database.as_ref() else {
        return Vec::new();
    };
    let relationships = database.relationships.clone();
    let groups = database.relationship_groups.clone();
    let tables = table_defs(database.tables.as_slice());
    columns
        .iter()
        .map(|column| {
            fk_nav::follow_targets(
                &tab.object_name,
                column,
                &relationships,
                groups.as_deref(),
                &tables,
            )
            .iter()
            .map(|target| match target.direction {
                FkDirection::Forward | FkDirection::Inferred => {
                    tr("Follow to {table}", &[("table", &target.table)])
                }
                FkDirection::Reverse => tr("Referenced by {table}", &[("table", &target.table)]),
            })
            .collect()
        })
        .collect()
}

/// Applies one follow intent (step 87.1, item M103): the target opens
/// (or activates) with the source row's key values as its filter — the
/// target's previous filters clear first, so the follow never lands in
/// a skewed view. An empty key or an unencodable value answers the
/// precise message instead of opening a tab.
pub(crate) fn follow_ui(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    row_values: Vec<FieldValue>,
    col: usize,
    index: usize,
) {
    if follow_restricted(app) {
        return;
    }
    let snapshot = app.snapshot();
    let Some(database) = snapshot.database.as_ref() else {
        return;
    };
    let relationships = database.relationships.clone();
    let groups = database.relationship_groups.clone();
    let tables = table_defs(database.tables.as_slice());
    let Some(column) = columns.get(col) else {
        return;
    };
    let target = fk_nav::follow_targets(
        &tab.object_name,
        column,
        &relationships,
        groups.as_deref(),
        &tables,
    )
    .into_iter()
    .nth(index);
    let Some(target) = target else {
        return;
    };
    // The row image travels in column order; the criteria encoder picks
    // the key pairs by name.
    let values: Vec<(String, Option<FieldValue>)> = columns
        .iter()
        .cloned()
        .zip(row_values)
        .map(|(name, value)| (name, Some(value)))
        .collect();
    let criteria = match fk_nav::follow_criteria(&target, &values, &tables) {
        Ok(criteria) => criteria,
        Err(reason) => {
            app.record_error(reason.to_string());
            return;
        }
    };
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        target.table.clone(),
    )));
    // The target's existing filters must not skew the follow: the
    // shared navigate path clears them and applies the key criteria.
    apply_filter_set(app, &target.table, &criteria_map(&criteria));
    // The navigation history (step 87.2, item M103): the source's
    // position rides onto the target tab's back stack.
    let snapshot = app.snapshot();
    if let Some(to_id) = snapshot
        .tabs
        .iter()
        .find(|candidate| candidate.object_name.eq_ignore_ascii_case(&target.table))
        .map(|candidate| candidate.id)
    {
        app.fk_follow_moved(tab, to_id);
    }
}

/// Clears the target's live filters and applies `filters` (the shared
/// navigate path of the follow and the history steps, step 87.2,
/// item M103). The filters are core tab state — the row load applies
/// them on every display pass.
pub(crate) fn apply_filter_set(
    app: &mut App,
    table: &str,
    filters: &std::collections::BTreeMap<String, String>,
) {
    let snapshot = app.snapshot();
    let existing: Vec<String> = snapshot
        .tabs
        .iter()
        .find(|candidate| candidate.object_name.eq_ignore_ascii_case(table))
        .map(|candidate| candidate.filters.keys().cloned().collect())
        .unwrap_or_default();
    for field in existing {
        app.dispatch(Command::SetTableFilter {
            table: table.to_owned(),
            field,
            criteria: None,
        });
    }
    for (field, criteria) in filters {
        app.dispatch(Command::SetTableFilter {
            table: table.to_owned(),
            field: field.clone(),
            criteria: Some(criteria.clone()),
        });
    }
}

/// The encoder's `(column, criteria)` pairs as the filter map the
/// navigate path applies.
fn criteria_map(criteria: &[(String, String)]) -> std::collections::BTreeMap<String, String> {
    criteria.iter().cloned().collect()
}

/// Queues the bounded count reads for a reverse follow target when the
/// cell context menu shows (step 87.2, item M103; duplicates and
/// answered keys are ignored, the reads run capped, one per frame).
pub(crate) fn request_counts(
    app: &mut App,
    tab: &TabSnapshot,
    columns: &[String],
    slots: &[usize],
    row: usize,
    col: usize,
) {
    if follow_restricted(app) {
        return;
    }
    let snapshot = app.snapshot();
    let Some(database) = snapshot.database.as_ref() else {
        return;
    };
    let relationships = database.relationships.clone();
    let groups = database.relationship_groups.clone();
    let tables = table_defs(database.tables.as_slice());
    let Some(column) = columns.get(col) else {
        return;
    };
    let reverse: Vec<_> = fk_nav::follow_targets(
        &tab.object_name,
        column,
        &relationships,
        groups.as_deref(),
        &tables,
    )
    .into_iter()
    .filter(|target| target.direction == FkDirection::Reverse)
    .collect();
    if reverse.is_empty() {
        return;
    }
    let slot = slots.get(row).copied().unwrap_or(row);
    let row_values = app.row_image(tab.id, slot).unwrap_or_default();
    let values: Vec<(String, Option<FieldValue>)> = columns
        .iter()
        .cloned()
        .zip(row_values)
        .map(|(name, value)| (name, Some(value)))
        .collect();
    for target in &reverse {
        if let Ok(criteria) = fk_nav::follow_criteria(target, &values, &tables) {
            app.queue_referenced_count(&target.table, &criteria);
        }
    }
    app.set_count_menu_tab(tab.id);
}

/// The frame-local count context (step 87.2, item M103): the reverse
/// targets per column, the row images and the answered counts, cloned
/// once per frame while a follow context menu is open — the grid's
/// suffix callback reads it without borrowing `App`.
pub(crate) struct FollowCounts {
    columns: Vec<String>,
    slots: Vec<usize>,
    tables: Vec<TableDef>,
    reverse: Vec<Vec<fk_nav::FkFollowTarget>>,
    rows: Vec<Vec<FieldValue>>,
    counts: std::collections::HashMap<String, ReferencedCount>,
}

impl FollowCounts {
    /// The menu entry's count suffix for one (row, col, index): exact
    /// and truncated counts render, pending reads and failures render
    /// without a suffix.
    pub(crate) fn suffix(&self, row: usize, col: usize, index: usize) -> Option<String> {
        let target = self.reverse.get(col)?.get(index)?;
        if target.direction != FkDirection::Reverse {
            return None;
        }
        let slot = self.slots.get(row).copied().unwrap_or(row);
        let row_values = self.rows.get(slot)?;
        let values: Vec<(String, Option<FieldValue>)> = self
            .columns
            .iter()
            .cloned()
            .zip(row_values.clone())
            .map(|(name, value)| (name, Some(value)))
            .collect();
        let criteria = fk_nav::follow_criteria(target, &values, &self.tables).ok()?;
        match self.counts.get(&crate::app::nav_history::count_key(
            &target.table,
            &criteria,
        )) {
            Some(ReferencedCount::Exact(count)) => Some(format!(" ({count})")),
            Some(ReferencedCount::Truncated(count)) => Some(format!(" (≥ {count})")),
            _ => None,
        }
    }
}

/// Builds the frame-local count context — only while this tab's follow
/// context menu is open (`None` otherwise: no clones in plain frames).
pub(crate) fn counts_context(
    app: &App,
    tab: &TabSnapshot,
    columns: &[String],
    slots: &[usize],
) -> Option<FollowCounts> {
    if app.count_menu_tab() != Some(tab.id) || follow_restricted(app) {
        return None;
    }
    let snapshot = app.snapshot();
    let database = snapshot.database.as_ref()?;
    let relationships = database.relationships.clone();
    let groups = database.relationship_groups.clone();
    let tables = table_defs(database.tables.as_slice());
    let reverse: Vec<Vec<_>> = columns
        .iter()
        .map(|column| {
            fk_nav::follow_targets(
                &tab.object_name,
                column,
                &relationships,
                groups.as_deref(),
                &tables,
            )
            .into_iter()
            .filter(|target| target.direction == FkDirection::Reverse)
            .collect()
        })
        .collect();
    // The row images with their staged overrides — the count keys ride
    // the same values the follow would encode.
    let mut rows: Vec<Vec<FieldValue>> = app.rows_for(tab.id).cloned().unwrap_or_default();
    if let Some(buffer) = app.staged_rows(tab.id) {
        for (slot, row) in rows.iter_mut().enumerate() {
            if let Some(image) = buffer.row_image(slot) {
                *row = image.to_vec();
            }
        }
    }
    Some(FollowCounts {
        columns: columns.to_vec(),
        slots: slots.to_vec(),
        tables,
        reverse,
        rows,
        counts: app.counts_snapshot(),
    })
}

/// The follow affordance hides where the driver declares no
/// relationship support (step 87.1, item M103).
fn follow_restricted(app: &App) -> bool {
    crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::RELATIONSHIPS,
    )
    .is_some()
}

/// The snapshot's table entries as schema defs (the projection the
/// datasheet's view order uses — names plus kinds).
fn table_defs(tables: &[datacraft_core::snapshot::TableEntrySnapshot]) -> Vec<TableDef> {
    tables
        .iter()
        .map(|entry| {
            TableDef::new(
                entry.name.clone(),
                entry
                    .fields
                    .iter()
                    .map(|field| FieldDef::new(field.name.clone(), field.kind))
                    .collect(),
            )
        })
        .collect()
}
