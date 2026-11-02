//! UI-side resolution of the core availability map (phase 30, item M17;
//! ADR-7): stable reasons resolve to localized text, and every surface
//! (ribbon, navigation pane, datasheet) asks the snapshot instead of
//! matching on formats.

use datacraft_core::availability::DisabledReason;
use datacraft_core::snapshot::UiStateSnapshot;

use crate::i18n::t;

/// Localized reason text for a disabled affordance.
pub fn reason_text(reason: &DisabledReason) -> &'static str {
    match reason {
        DisabledReason::CreateTables => t("the open source cannot create tables"),
        DisabledReason::DropTables => t("the open source cannot drop tables"),
        DisabledReason::WriteRows => t("the open source is read-only"),
        DisabledReason::NoRelationships => t("the open source has no relationships"),
    }
}

/// `Some(reason)` when the affordance id is disabled for the open
/// source; absence means enabled.
pub fn restricted<'a>(snapshot: &'a UiStateSnapshot, id: &str) -> Option<&'a DisabledReason> {
    snapshot.database.as_ref()?.availability.get(id)
}
