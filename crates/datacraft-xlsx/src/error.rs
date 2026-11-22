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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_protected_workbook_is_an_unsupported_boundary() {
        let error = calamine::Error::Xlsx(calamine::XlsxError::Password);
        let mapped = open_error(std::path::Path::new("book.xlsx"), &error);
        assert!(
            matches!(mapped, PortError::Unsupported { .. }),
            "{mapped:?}"
        );
        assert!(mapped.to_string().contains("password"));
    }

    #[test]
    fn every_other_open_error_names_the_file() {
        let error = calamine::Error::Msg("boom");
        let mapped = open_error(std::path::Path::new("book.xlsx"), &error);
        match mapped {
            PortError::OpenFailed { detail } => {
                assert!(detail.contains("book.xlsx"), "{detail}");
                assert!(detail.contains("boom"), "{detail}");
            }
            other => panic!("expected OpenFailed, got {other:?}"),
        }
    }

    #[test]
    fn the_write_refusal_names_the_surface() {
        assert!(read_only("insert rows").to_string().contains("insert rows"));
    }
}
