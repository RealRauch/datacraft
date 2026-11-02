//! Sibling `.datacraft` sidecar loading (phase 29, step 29.4; step 32.1,
//! item H45) — split from `database.rs` to keep every file under the size
//! gate.

use crate::schema::SchemaDocument;

/// Reads the sibling `.datacraft` sidecar (ADR-1) for the database at
/// `path`. A missing sidecar is `Ok(None)` — it is not an error; a broken
/// document or a foreign database name is `Err(reason)`, surfaced as
/// `sidecar_error` (phase 29, step 29.4).
pub(super) fn read_sidecar(
    path: &std::path::Path,
    expected_name: &str,
) -> Result<Option<SchemaDocument>, String> {
    let sidecar = path.with_extension("datacraft");
    let json = match std::fs::read_to_string(&sidecar) {
        Ok(json) => json,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("sidecar read failed: {e}")),
    };
    let document =
        SchemaDocument::from_json(&json).map_err(|e| format!("sidecar decode failed: {e}"))?;
    document
        .validate()
        .map_err(|e| format!("sidecar validation failed: {e}"))?;
    if document.database_name != expected_name {
        return Err(format!(
            "the sidecar belongs to {:?}, not to {:?}",
            document.database_name, expected_name
        ));
    }
    Ok(Some(document))
}
