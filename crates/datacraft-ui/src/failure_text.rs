//! Localized failure text (step 62.2, item I6): known failure codes render
//! localized text from the UI catalog, every other code keeps the precise
//! English wire text unchanged (ADR-4: language stays UI-owned — core
//! carries no localized text).

use datacraft_core::failure::FailureCode;

use crate::i18n::t;

/// The UI text for one failure: known codes answer their localized
/// sentence, unknown codes (and data-bearing reasons) keep the precise
/// English text.
pub fn failure_text(code: FailureCode, reason: &str) -> String {
    match code {
        FailureCode::NoDatabase => t("no database is open — open a database first").to_owned(),
        // The capability refusals map onto the affordance vocabulary —
        // the same localized sentences the disabled affordances carry
        // (ADR-7, M17).
        FailureCode::Capability(reason) => crate::availability::reason_text(&reason).to_owned(),
        _ => reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datacraft_core::availability::DisabledReason;

    #[test]
    fn unknown_codes_keep_the_english_reason() {
        let reason = "table \"missing\" not found";
        assert_eq!(
            failure_text(FailureCode::NotFound, reason),
            reason,
            "a data-bearing reason stays the precise English text"
        );
    }

    #[test]
    fn the_no_database_code_resolves_the_canonical_sentence() {
        assert_eq!(
            failure_text(FailureCode::NoDatabase, "ignored"),
            t("no database is open — open a database first")
        );
    }

    #[test]
    fn capability_codes_resolve_the_affordance_vocabulary() {
        let reason = DisabledReason::WriteRows;
        assert_eq!(
            failure_text(FailureCode::Capability(reason), "unsupported: …"),
            crate::availability::reason_text(&reason)
        );
    }
}
