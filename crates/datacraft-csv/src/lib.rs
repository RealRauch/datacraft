//! CSV directory driver for the DataCraft engine (phase 30, item M18;
//! ADR-7): the out-of-tree proof that a capability-poor data source docks
//! through the core-owned [`StoragePort`] alone — this crate depends on
//! nothing but `datacraft-core`.
//!
//! A *database* is a directory; its tables are the `*.csv` files inside.
//! The header row names the columns, every column is Short Text, and any
//! write rewrites the whole file once (sibling temp file + rename), so a
//! call is atomic — either every operation of the call lands or none.
//! Rows are addressed positionally in file order (data rows after the
//! header). Non-text domain values render deterministically through the
//! core's cell display (`Null` becomes an empty cell).
//!
//! ## Column type inference (L7)
//!
//! The schema kind of each column is inferred at open, deterministically,
//! from a full scan: all-integer columns become `LongInteger`, all-float
//! columns `Double`, all-ISO-date columns (`YYYY-MM-DD`) `DateTime`;
//! anything else — and every ambiguous column — stays Short Text. Quoting
//! is the user's escape hatch: one quoted cell pins the whole column to
//! Short Text, and empty cells don't vote. Cell **values** still read as
//! text; only the schema kind is inferred (the datasheet formats and
//! parses by the declared kind). The inference and the quote mask live in
//! `datacraft_core::import` — shared with the `ImportFile` chain (M50).

use std::path::{Path, PathBuf};

use datacraft_core::import::{infer_field_kinds, quote_mask};

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, StoragePort, Support, window_slice,
};
use datacraft_core::schema::{FieldDef, FieldValue, TableDef};

/// One CSV directory as a database.
#[derive(Debug, Default)]
pub struct CsvPort {
    dir: Option<PathBuf>,
    tables: Vec<TableDef>,
}

impl CsvPort {
    /// Creates an unattached driver.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StoragePort for CsvPort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("csv")
    }

    /// The driver's fixed family (M23): the inventory reports it so a
    /// composed binary can advertise the CSV driver offline.
    fn source_format(&self) -> Option<DatabaseFormat> {
        Some(DatabaseFormat::Csv)
    }

    /// The honest set (phase 30, M18): table/row/file creation served
    /// with one atomic rewrite per call; no schema alteration, no
    /// passwords, no engine-assigned AutoNumbers, and none of the
    /// Jet-style inventories.
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            read_tables: true,
            write_rows: true,
            create_table: true,
            drop_table: true,
            create_database: true,
            alter_table: false,
            atomic_batch: true,
            passwords: false,
            autonumber: Support::No,
            relationships: Support::No,
            native_queries: Support::No,
            macros: Support::No,
            vba: Support::No,
            triggers: Support::No,
            stored_procedures: Support::No,
        }
    }

    fn open(&mut self, path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        if !path.is_dir() {
            return Err(PortError::OpenFailed {
                detail: format!(
                    "{}: not a CSV directory — this driver opens directories of *.csv tables",
                    path.display()
                ),
            });
        }
        let tables = collect_tables(path)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.tables = tables;
        self.dir = Some(path.to_path_buf());
        Ok(OpenedDatabase {
            name,
            format: DatabaseFormat::Csv,
            lock_file: None,
        })
    }

    fn close(&mut self) {
        self.dir = None;
        self.tables.clear();
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        if format != DatabaseFormat::Csv {
            return Err(PortError::Unsupported {
                reason: "the CSV driver creates CSV directories only — use the matching \
                         backend for Jet4 or SQLite files"
                    .to_owned(),
            });
        }
        if path.exists() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "{}: already exists — creating would overwrite it",
                    path.display()
                ),
            });
        }
        std::fs::create_dir_all(path).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        })
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        let dir = self.dir.as_ref().ok_or_else(no_database)?;
        // The Jet and SQLite drivers validate; a CSV header cannot carry an
        // empty or duplicate name either (the open path would reject it).
        definition.validate().map_err(|e| PortError::WriteFailed {
            detail: format!("table {:?}: {e}", definition.name),
        })?;
        let file_name = table_file_name(&definition.name)?;
        let file = dir.join(file_name);
        if file.exists() {
            return Err(PortError::WriteFailed {
                detail: format!("{}: table already exists", definition.name),
            });
        }
        let header = definition
            .fields
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<String>>();
        write_rows(&file, &[header])?;
        // The in-memory catalog grows with the directory (sorted, like
        // the listing at open).
        self.tables.push(definition.clone());
        self.tables.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        let dir = self.dir.as_ref().ok_or_else(no_database)?;
        let (canonical, _) = self
            .tables
            .iter()
            .find(|def| def.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| PortError::TableNotFound(name.to_owned()))
            .map(|def| (def.name.clone(), ()))?;
        let file = dir.join(table_file_name(&canonical)?);
        std::fs::remove_file(&file).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", file.display()),
        })?;
        self.tables.retain(|def| def.name != canonical);
        Ok(canonical)
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        let rows = self.rows_of(table)?;
        let (start, end) = window_slice(table, range, rows.len())?;
        Ok(rows[start..end]
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| FieldValue::Text(cell.clone()))
                    .collect()
            })
            .collect())
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        let arity = self.definition_of(table)?.fields.len();
        let canonical = self.definition_of(table)?.name.clone();
        if values.len() != arity {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "insert into {:?}: expected {} values, found {}",
                    canonical,
                    arity,
                    values.len()
                ),
            });
        }
        let mut rows = self.rows_of(&canonical)?;
        rows.push(values.iter().map(to_cell).collect::<Vec<String>>());
        self.write_table(&canonical, rows)
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        let arity = self.definition_of(table)?.fields.len();
        let canonical = self.definition_of(table)?.name.clone();
        if values.len() != arity {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "update {:?}: expected {} values, found {}",
                    canonical,
                    arity,
                    values.len()
                ),
            });
        }
        let mut rows = self.rows_of(&canonical)?;
        if row >= rows.len() {
            return Err(out_of_range(&canonical, row, rows.len()));
        }
        let target = &mut rows[row];
        *target = values.iter().map(to_cell).collect::<Vec<String>>();
        self.write_table(&canonical, rows)
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        let canonical = self.definition_of(table)?.name.clone();
        let mut rows = self.rows_of(&canonical)?;
        if row >= rows.len() {
            return Err(out_of_range(&canonical, row, rows.len()));
        }
        rows.remove(row);
        self.write_table(&canonical, rows)
    }

    /// One read-modify-write per batch: every operation applies to the
    /// in-memory row set and the file is rewritten once — the batch is
    /// atomic (ADR-7 `atomic_batch`).
    fn write_row_ops(
        &mut self,
        table: &str,
        ops: &[datacraft_core::port::RowWriteOp],
    ) -> Result<(), PortError> {
        let arity = self.definition_of(table)?.fields.len();
        let canonical = self.definition_of(table)?.name.clone();
        let mut rows = self.rows_of(&canonical)?;
        for op in ops {
            match op {
                datacraft_core::port::RowWriteOp::Insert(values) => {
                    if values.len() != arity {
                        return Err(PortError::WriteFailed {
                            detail: format!(
                                "batch insert into {:?}: expected {} values, found {}",
                                canonical,
                                arity,
                                values.len()
                            ),
                        });
                    }
                    rows.push(values.iter().map(to_cell).collect::<Vec<String>>());
                }
                datacraft_core::port::RowWriteOp::Update { row, values } => {
                    if values.len() != arity {
                        return Err(PortError::WriteFailed {
                            detail: format!(
                                "batch update of {:?}: expected {} values, found {}",
                                canonical,
                                arity,
                                values.len()
                            ),
                        });
                    }
                    if *row >= rows.len() {
                        return Err(out_of_range(&canonical, *row, rows.len()));
                    }
                    let target = &mut rows[*row];
                    *target = values.iter().map(to_cell).collect::<Vec<String>>();
                }
                datacraft_core::port::RowWriteOp::Delete { row } => {
                    if *row >= rows.len() {
                        return Err(out_of_range(&canonical, *row, rows.len()));
                    }
                    rows.remove(*row);
                }
            }
        }
        self.write_table(&canonical, rows)
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(Vec::new())
    }
}

impl CsvPort {
    /// The canonical definition of a catalogued table.
    fn definition_of(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .find(|def| def.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Data rows of one table, parsed fresh per call (stateless,
    /// deterministic).
    fn rows_of(&self, table: &str) -> Result<Vec<Vec<String>>, PortError> {
        let dir = self.dir.as_ref().ok_or_else(no_database)?;
        let def = self.definition_of(table)?;
        let file = dir.join(table_file_name(&def.name)?);
        let records = parse_records(&file, &read_table_text(&file)?)?;
        Ok(records.into_iter().skip(1).collect())
    }

    /// Rewrites one table file in a single atomic replace.
    fn write_table(&mut self, table: &str, rows: Vec<Vec<String>>) -> Result<(), PortError> {
        let dir = self.dir.as_ref().ok_or_else(no_database)?;
        let def = self.definition_of(table)?.clone();
        let header: Vec<String> = def.fields.iter().map(|f| f.name.clone()).collect();
        let file = dir.join(table_file_name(&def.name)?);
        let mut all = Vec::with_capacity(rows.len() + 1);
        all.push(header);
        all.extend(rows);
        write_rows(&file, &all)
    }
}

/// Maps a domain value onto a CSV cell deterministically.
fn to_cell(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => String::new(),
        FieldValue::Text(text) => text.clone(),
        other => datacraft_core::present::display_value(other),
    }
}

/// Table names become file names: alphanumeric plus `_`/`-`, so a
/// hostile name can never escape the directory (LESSONS #1).
fn table_file_name(name: &str) -> Result<String, PortError> {
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
fn collect_tables(dir: &Path) -> Result<Vec<TableDef>, PortError> {
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
        let text = read_table_text(&file)?;
        let records = parse_records(&file, &text)?;
        let header = records.first().ok_or_else(|| PortError::OpenFailed {
            detail: format!("{}: the CSV file has no header row", file.display()),
        })?;
        let mask = quote_mask(&text);
        let kinds = infer_field_kinds(&records, &mask);
        let mut fields = Vec::with_capacity(header.len());
        for (index, column) in header.iter().enumerate() {
            if column.is_empty() {
                return Err(PortError::OpenFailed {
                    detail: format!("{}: header column {index} has no name", file.display()),
                });
            }
            fields.push(FieldDef::new(column.clone(), kinds[index]));
        }
        tables.push(TableDef::new(stem, fields));
    }
    Ok(tables)
}

/// Reads one table file as UTF-8 text with a leading BOM stripped — the
/// single read that feeds parsing and the L7 quote mask alike;
/// non-UTF-8 content fails with a locatable reason (LESSONS #4).
fn read_table_text(file: &Path) -> Result<String, PortError> {
    let bytes = std::fs::read(file).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", file.display()),
    })?;
    let text = String::from_utf8(bytes).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: not valid UTF-8: {e}", file.display()),
    })?;
    match text.strip_prefix('\u{feff}') {
        Some(stripped) => Ok(stripped.to_owned()),
        None => Ok(text),
    }
}

/// Parses one CSV text into uniform records; ragged rows fail with a
/// locatable reason naming the file (LESSONS #4). The caller reads the
/// file once (the same text feeds the L7 quote mask).
fn parse_records(file: &Path, text: &str) -> Result<Vec<Vec<String>>, PortError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
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
fn write_rows(file: &Path, rows: &[Vec<String>]) -> Result<(), PortError> {
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

fn out_of_range(table: &str, row: usize, row_count: usize) -> PortError {
    PortError::OutOfRange {
        table: table.to_owned(),
        start: row,
        len: 1,
        row_count,
    }
}

fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}
