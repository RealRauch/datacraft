//! Save-As and password-change implementation (step 27.2, item M14) —
//! split from `port_adapter.rs` to keep every file under the size gate.

use std::path::Path;

use datacraft_core::port::PortError;

use crate::jet::reader::Jet4TableReader;
use crate::port_errors::{map_open, map_write, no_database};
use crate::types::FormatVersion;

use super::Jet4Port;

/// Writes a copy of the open database (step 27.2, item M14). The copy
/// is derived from the decrypted writer handoff: the header region is
/// rewritten with the requested password state and page-encrypted
/// accordingly. The source file is never touched.
pub(crate) fn save_as(
    port: &mut Jet4Port,
    path: &Path,
    password: Option<&str>,
) -> Result<(), PortError> {
    let reader = port.reader.as_ref().ok_or_else(no_database)?;
    if reader.header.format == FormatVersion::Jet3 {
        return Err(PortError::Unsupported {
            reason: "Jet3 (Access 97) sources are read-only — save-as needs a writable \
                     format"
                .to_owned(),
        });
    }
    let header = reader.header.clone();
    let mut copy = reader.file.clone();
    let encryption =
        crate::jet::crypto::set_password(&mut copy, &header, password).map_err(map_write)?;
    crate::jet::crypto::encrypt_file(&mut copy, &header, encryption.as_ref()).map_err(map_write)?;
    write_new_file(path, &copy)
}

/// Rewrites the open file with a new password (step 27.2, item M14).
/// The reader swaps to the newly encrypted state, so subsequent writes
/// re-encrypt with the new key.
pub(crate) fn change_password(
    port: &mut Jet4Port,
    new_password: Option<&str>,
) -> Result<(), PortError> {
    let path = port.path.clone().ok_or_else(no_database)?;
    let (header, decrypted) = {
        let reader = port.reader.as_ref().ok_or_else(no_database)?;
        (reader.header.clone(), reader.file.clone())
    };
    let mut decrypted = decrypted;
    let encryption = crate::jet::crypto::set_password(&mut decrypted, &header, new_password)
        .map_err(map_write)?;
    let mut disk = decrypted.clone();
    crate::jet::crypto::encrypt_file(&mut disk, &header, encryption.as_ref()).map_err(map_write)?;
    write_replacing(&path, &disk)?;
    let reader = Jet4TableReader::from_decrypted(decrypted, encryption).map_err(map_open)?;
    port.reader = Some(reader);
    Ok(())
}

/// Writes a new file, refusing an existing target; a failed write removes
/// the partial file so no half-copy survives (step 27.2, M14).
fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), PortError> {
    match datacraft_core::atomic_write::write_new(path, bytes) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(PortError::WriteFailed {
            detail: format!("{}: the target file already exists", path.display()),
        }),
        Err(e) => Err(PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        }),
    }
}

/// Atomically replaces `path` through a sibling temp file: an interrupted
/// write can never truncate the existing database.
fn write_replacing(path: &Path, bytes: &[u8]) -> Result<(), PortError> {
    datacraft_core::atomic_write::write_replacing(path, bytes).map_err(|e| PortError::WriteFailed {
        detail: format!("{}: {e}", path.display()),
    })
}
