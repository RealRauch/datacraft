//! Open/write refusals for the Excel driver.

use std::path::Path;

use datacraft_core::port::PortError;

/// The precise refusal of every write on a workbook.
pub(crate) fn read_only(what: &str) -> PortError {
    PortError::Unsupported {
        reason: format!("Excel workbooks are read-only — cannot {what}"),
    }
}

/// Maps calamine's open error: a password-protected workbook is a precise
/// `Unsupported` (roadmap §7); everything else is a locatable open failure.
pub(crate) fn open_error(path: &Path, error: &calamine::Error) -> PortError {
    let text = error.to_string();
    if text.to_lowercase().contains("password") {
        return PortError::Unsupported {
            reason: format!(
                "{}: the workbook is password-protected — DataCraft reads unprotected \
                 Excel workbooks only",
                path.display()
            ),
        };
    }
    PortError::OpenFailed {
        detail: format!("{}: {text}", path.display()),
    }
}
