//! SQLite file creation (split from `lib.rs` for the file-size gate in
//! phase 73).

use std::path::Path;

use datacraft_core::port::{DatabaseFormat, PortError};
use rusqlite::Connection;

pub(super) fn create(path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
    if format != DatabaseFormat::Sqlite {
        return Err(PortError::Unsupported {
            reason: "the SQLite backend creates SQLite databases only — use the Jet \
                     backend for Jet4 files"
                .to_owned(),
        });
    }
    if path.exists() {
        return Err(PortError::WriteFailed {
            detail: format!(
                "{}: file already exists — creating would overwrite it",
                path.display()
            ),
        });
    }
    // `Connection::open` creates a zero-byte file; VACUUM materializes
    // the real database header so format detection sniffs the fresh
    // file. A failed create leaves no debris behind.
    let outcome = initialize(path);
    if outcome.is_err() {
        let _ = std::fs::remove_file(path);
    }
    outcome
}

/// Creates and initializes a brand-new SQLite database file.
fn initialize(path: &Path) -> Result<(), PortError> {
    let failed = |stage: &str, e: rusqlite::Error| PortError::WriteFailed {
        detail: format!("{}: {stage}: {e}", path.display()),
    };
    let conn = Connection::open(path).map_err(|e| failed("opening", e))?;
    conn.execute_batch("VACUUM")
        .map_err(|e| failed("initializing", e))?;
    conn.close().map_err(|(_, e)| failed("closing", e))?;
    Ok(())
}
