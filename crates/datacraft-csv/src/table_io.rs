//! Table-file IO for the CSV directory driver (moved out of `lib.rs`
//! for the file-size gate, L30): the catalog, the read path through the
//! one shared core decode (step 80.2, item H59 — encoding detection and
//! delimiter sniffing) and the one-shot atomic write.

use std::path::Path;

use datacraft_core::import::{decode_csv_text, infer_field_kinds, quote_mask};
use datacraft_core::port::PortError;
use datacraft_core::schema::TableDef;

/// Table names become file names: alphanumeric plus `_`/`-`, so a
/// hostile name can never escape the directory (LESSONS #1).
pub(crate) fn table_file_name(name: &str) -> Result<String, PortError> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(PortError::WriteFailed {
            detail: format!(
                "table name {name:?} is not usable as a CSV file name — use letters, \
                 digits, '_' and '-'"
            ),
        });
    }
    Ok(format!("{name}.csv"))
}

/// Catalogues the directory's `*.csv` files in sorted stem order.
pub(crate) fn collect_tables(dir: &Path) -> Result<Vec<TableDef>, PortError> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| PortError::OpenFailed {
            detail: format!("{}: {e}", dir.display()),
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            let extension = path.extension()?.to_string_lossy().to_lowercase();
            (extension == "csv").then_some(stem)
        })
        .collect();
    names.sort();
    let mut tables = Vec::with_capacity(names.len());
    for stem in names {
        let file = dir.join(format!("{stem}.csv"));
        let (text, delimiter) = read_table_text(&file)?;
        let records = parse_records(&file, &text, delimiter)?;
        let header = records.first().ok_or_else(|| PortError::OpenFailed {
            detail: format!("{}: the CSV file has no header row", file.display()),
        })?;
        let mask = quote_mask(&text, delimiter);
        let kinds = infer_field_kinds(&records, &mask);
        let mut fields = Vec::with_capacity(header.len());
        for (index, column) in header.iter().enumerate() {
            if column.is_empty() {
                return Err(PortError::OpenFailed {
                    detail: format!("{}: header column {index} has no name", file.display()),
                });
            }
            fields.push(datacraft_core::schema::FieldDef::new(
                column.clone(),
                kinds[index],
            ));
        }
        tables.push(TableDef::new(stem, fields));
    }
    Ok(tables)
}

/// Reads one table file through the shared decode path (step 80.2, item
/// H59): valid UTF-8 first (a leading BOM stripped — byte-identical for
/// every existing file), then UTF-16 BOM streams, then Windows-1252; the
/// delimiter is sniffed and returned beside the text. Undecodable bytes
/// fail with a locatable reason (LESSONS #4). The file is only read.
pub(crate) fn read_table_text(file: &Path) -> Result<(String, u8), PortError> {
    let bytes = std::fs::read(file).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", file.display()),
    })?;
    let (text, dialect) = decode_csv_text(bytes).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", file.display()),
    })?;
    Ok((text, dialect.delimiter))
}

/// Parses one CSV text into uniform records; ragged rows fail with a
/// locatable reason naming the file (LESSONS #4). The caller reads the
/// file once (the same text feeds the L7 quote mask).
pub(crate) fn parse_records(
    file: &Path,
    text: &str,
    delimiter: u8,
) -> Result<Vec<Vec<String>>, PortError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());
    let mut records: Vec<Vec<String>> = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| PortError::OpenFailed {
            detail: format!("{}: row {}: {e}", file.display(), index + 1),
        })?;
        // `flexible(false)` makes a ragged row a reader *error*, so every
        // pushed record has the header's arity by construction.
        records.push(record.into_iter().map(String::from).collect());
    }
    Ok(records)
}

/// Writes one CSV file through a sibling temp file renamed over the
/// target: an interrupted write can never truncate the table, and the
/// rename makes the call atomic.
pub(crate) fn write_rows(file: &Path, rows: &[Vec<String>]) -> Result<(), PortError> {
    let temp = file.with_extension("csv.tmp");
    let write = || -> Result<(), PortError> {
        let mut writer = csv::Writer::from_path(&temp).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", temp.display()),
        })?;
        for row in rows {
            writer
                .write_record(row)
                .map_err(|e| PortError::WriteFailed {
                    detail: format!("{}: {e}", temp.display()),
                })?;
        }
        writer.flush().map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", temp.display()),
        })?;
        std::fs::rename(&temp, file).map_err(|e| PortError::WriteFailed {
            detail: format!("{} -> {}: {e}", temp.display(), file.display()),
        })?;
        Ok(())
    };
    let outcome = write();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    outcome
}
