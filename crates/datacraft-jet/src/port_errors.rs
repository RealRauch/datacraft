//! Storage-error -> port-error mapping shared by the Jet4 adapter
//! (split from `port_adapter.rs` to keep every file under the size gate).

use std::path::Path;

use datacraft_core::port::PortError;

use crate::error::StorageError;

/// The "no database is open" answer of every port operation.
pub(crate) fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}

/// Write-path mapping.
pub(crate) fn map_write(error: StorageError) -> PortError {
    match error {
        StorageError::Unsupported { reason } => PortError::Unsupported { reason },
        StorageError::OutOfRange {
            table,
            start,
            len,
            row_count,
        } => PortError::OutOfRange {
            table,
            start,
            len,
            row_count,
        },
        other => PortError::WriteFailed {
            detail: other.to_string(),
        },
    }
}

/// Open-path mapping.
pub(crate) fn map_open(error: StorageError) -> PortError {
    match error {
        StorageError::Unsupported { reason } => PortError::Unsupported { reason },
        other => PortError::OpenFailed {
            detail: other.to_string(),
        },
    }
}

/// Open-path mapping that keeps the password errors typed and names the
/// file (step 16.2, M5).
pub(crate) fn map_open_with_path(error: StorageError, path: &Path) -> PortError {
    match error {
        StorageError::PasswordRequired => PortError::PasswordRequired {
            path: path.display().to_string(),
        },
        StorageError::WrongPassword => PortError::WrongPassword {
            path: path.display().to_string(),
        },
        other => map_open(other),
    }
}
