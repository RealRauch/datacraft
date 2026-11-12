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
        DisabledReason::NoPasswords => t("the open source has no database passwords"),
        DisabledReason::AlterTables => t("the open source cannot alter the schema"),
        DisabledReason::RenameTables => t("the open source cannot rename tables"),
        DisabledReason::CreateDatabase => t("the open source cannot create new databases"),
        // `DisabledReason` is non_exhaustive (M32): a reason from a newer
        // core still renders a localized, honest sentence.
        _ => t("the open source does not support this action"),
    }
}

/// `Some(reason)` when the affordance id is disabled for the open
/// source; absence means enabled.
pub fn restricted<'a>(snapshot: &'a UiStateSnapshot, id: &str) -> Option<&'a DisabledReason> {
    snapshot.database.as_ref()?.availability.get(id)
}
