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

use crate::app::App;
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
    // The target's existing filters must not skew the follow: clear
    // them, then set the key criteria. The filters are core tab state —
    // the row load applies them on every display pass.
    let existing: Vec<String> = app
        .snapshot()
        .tabs
        .iter()
        .find(|candidate| candidate.object_name.eq_ignore_ascii_case(&target.table))
        .map(|candidate| candidate.filters.keys().cloned().collect())
        .unwrap_or_default();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        target.table.clone(),
    )));
    for field in existing {
        app.dispatch(Command::SetTableFilter {
            table: target.table.clone(),
            field,
            criteria: None,
        });
    }
    for (field, criteria) in criteria {
        app.dispatch(Command::SetTableFilter {
            table: target.table.clone(),
            field,
            criteria: Some(criteria),
        });
    }
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
