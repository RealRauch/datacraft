//! Compact for Jet4/ACE (phase 95, step 95.2; item M126).
//!
//! Compact reads everything, writes a fresh file of the same family from
//! the embedded engine-written template, verifies the fresh file against
//! the collected data, then swaps it atomically at the original path and
//! re-opens the session. Nothing is rewritten in place, so an interrupted
//! compact leaves the original byte-identical.
//!
//! Boundaries stated plainly: Jet3 sources are read-only, a password- or
//! page-protected source cannot be re-encrypted without its plaintext
//! password (refused precisely), and a locked session never writes (M59).
//! The deterministic-write rule holds: identical input bytes produce
//! identical compacted bytes (the template is fixed and the catalog rows
//! carry the source's creation date).

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;

use crate::jet::writer::Jet4Writer;
use crate::port_errors::{map_open, map_write, no_database};
use crate::types::FormatVersion;

use super::Jet4Port;

/// One table's collected content in storage order.
struct Collected {
    definition: datacraft_core::schema::TableDef,
    rows: Vec<Vec<FieldValue>>,
}

pub(crate) fn compact(port: &mut Jet4Port) -> Result<(), PortError> {
    let path = port.path.clone().ok_or_else(no_database)?;
    if port.lock_file.is_some() {
        return Err(port.locked_error());
    }
    let reader = port.reader.as_ref().ok_or_else(no_database)?;
    let format = reader.header.format;
    if format == FormatVersion::Jet3 {
        return Err(PortError::Unsupported {
            reason: "Jet3 (Access 97) sources are read-only — compact needs a writable \
                     format"
                .to_owned(),
        });
    }
    let created = match reader.header_info() {
        Some(info) if !info.password_protected && info.db_key == 0 => info.creation_date,
        Some(_) => {
            return Err(PortError::Unsupported {
                reason: "compacting a password-protected database needs its plaintext \
                         password, which the session does not retain — save an unprotected \
                         copy and compact that"
                    .to_owned(),
            });
        }
        None => {
            return Err(PortError::Unsupported {
                reason: "the source's protection state could not be read — compact refuses \
                         rather than guessing"
                    .to_owned(),
            });
        }
    };

    // Everything the fresh file must carry, read once through the session.
    let definitions = port.tables.clone();
    let mut collected = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let rows = port.read_rows(
            &definition.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )?;
        collected.push(Collected { definition, rows });
    }
    let mut relationships: Vec<datacraft_core::port::Relationship> = port
        .relationships()?
        .into_iter()
        .filter(|relationship| {
            !relationship.table.starts_with("MSys")
                && !relationship.related_table.starts_with("MSys")
        })
        .collect();
    relationships.sort_by(|a, b| {
        (&a.table, &a.column, &a.related_table, &a.related_column).cmp(&(
            &b.table,
            &b.column,
            &b.related_table,
            &b.related_column,
        ))
    });

    // Build the fresh file in memory from the embedded engine template.
    let template = match format {
        FormatVersion::Jet4 => crate::jet::create::fresh_bytes(),
        FormatVersion::Ace => crate::jet::create::ace_template_bytes(),
        FormatVersion::Jet3 => unreachable!("rejected above"),
    };
    let mut writer = Jet4Writer::new(template).map_err(map_open)?;
    writer.set_catalog_date(created);
    for table in &collected {
        let columns = crate::port_write::table_to_columns(&table.definition)?;
        let page = writer
            .create_table(&table.definition.name, &columns)
            .map_err(map_write)?;
        for row in &table.rows {
            let stored =
                crate::port_write::to_storage_values_for_restore(&table.definition.fields, row)?;
            writer.insert_row(page, &stored).map_err(map_write)?;
        }
    }

    // Stage, add relationships through the ordinary port path, verify,
    // then swap. The staging file never replaces the original on failure.
    let stage = datacraft_core::atomic_write::staging_path(&path);
    if let Err(error) = datacraft_core::atomic_write::write_new(&stage, &writer.into_file()) {
        return Err(PortError::WriteFailed {
            detail: format!("{}: {error}", stage.display()),
        });
    }
    let verified = verify(&stage, &collected, &relationships);
    let bytes = match verified {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = std::fs::remove_file(&stage);
            return Err(error);
        }
    };
    let _ = std::fs::remove_file(&stage);
    datacraft_core::atomic_write::write_replacing(&path, &bytes).map_err(|error| {
        PortError::WriteFailed {
            detail: format!("{}: {error}", path.display()),
        }
    })?;
    // Refresh the session from the swapped file (unprotected by the guard).
    port.open(&path, None)?;
    Ok(())
}

/// Adds the relationships to the staged file, then proves the fresh file
/// carries exactly the collected tables, rows and relationships before the
/// swap. Returns the staged bytes.
fn verify(
    stage: &std::path::Path,
    collected: &[Collected],
    relationships: &[datacraft_core::port::Relationship],
) -> Result<Vec<u8>, PortError> {
    let mut fresh = Jet4Port::new();
    fresh.open(stage, None)?;
    for relationship in relationships {
        fresh.create_relationship(relationship)?;
    }
    let tables = fresh.tables();
    if tables.len() != collected.len() {
        return Err(mismatch(format!(
            "the fresh file carries {} tables, the source {}",
            tables.len(),
            collected.len()
        )));
    }
    for expected in collected {
        let Some(actual) = tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(&expected.definition.name))
        else {
            return Err(mismatch(format!(
                "table {:?} is missing from the fresh file",
                expected.definition.name
            )));
        };
        if actual.fields.len() != expected.definition.fields.len() {
            return Err(mismatch(format!(
                "table {:?} carries {} fields, the source {}",
                expected.definition.name,
                actual.fields.len(),
                expected.definition.fields.len()
            )));
        }
        let rows = fresh.read_rows(
            &expected.definition.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )?;
        if rows != expected.rows {
            return Err(mismatch(format!(
                "table {:?} carries {} rows, the source {} (values differ)",
                expected.definition.name,
                rows.len(),
                expected.rows.len()
            )));
        }
    }
    let mut written = fresh.relationships()?;
    written.retain(|relationship| {
        !relationship.table.starts_with("MSys") && !relationship.related_table.starts_with("MSys")
    });
    written.sort_by(|a, b| {
        (&a.table, &a.column, &a.related_table, &a.related_column).cmp(&(
            &b.table,
            &b.column,
            &b.related_table,
            &b.related_column,
        ))
    });
    if written != relationships {
        return Err(mismatch(
            "the fresh file's relationships differ from the source".to_owned(),
        ));
    }
    fresh.close();
    std::fs::read(stage).map_err(|error| PortError::WriteFailed {
        detail: format!("{}: {error}", stage.display()),
    })
}

fn mismatch(detail: String) -> PortError {
    PortError::WriteFailed {
        detail: format!("compact verification failed: {detail}"),
    }
}
