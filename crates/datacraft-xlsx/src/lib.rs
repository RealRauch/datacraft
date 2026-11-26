//! Excel workbook read driver (phase 96, item M127; roadmap §7).
//!
//! A workbook is a database; each sheet is a table. The reader is
//! [calamine](https://crates.io/crates/calamine) (MIT, pure Rust, empty
//! default features) — the same precedent as the SQLite backend using
//! rusqlite for a publicly standardized format. The crate depends only on
//! `datacraft-core`, the ADR-7 out-of-tree driver rule (the CSV driver is
//! the other proof).
//!
//! ## Boundaries
//!
//! - **Read-only.** Every write affordance is declared unavailable; the
//!   workbook is never modified.
//! - **Values, not formulas.** A formula cell surfaces its saved cached
//!   value; formulas are neither evaluated nor rewritten.
//! - **No macros.** `.xlsm` is read like `.xlsx`; a VBA project is never
//!   read, executed or modified.
//! - **Families:** `.xlsx`, `.xlsm`, `.xls` (BIFF8) and `.xlsb`. ODF
//!   spreadsheets belong to their own later module (roadmap §8) and are
//!   refused precisely.
//! - **Password-protected workbooks** answer a precise `Unsupported`.
//!
//! ## Schema mapping
//!
//! The first row names the columns (empty headers become `Column<N>`).
//! Each column's kind is inferred from all its non-empty cells:
//! integers → Long Integer, integers/floats → Double, dates → Date/Time,
//! booleans → Yes/No, everything else (text, error cells, mixed
//! families) → Short Text (Memo when a value exceeds 255 characters).
//! Empty cells are `Null`. Error cells render their Excel label as text —
//! DataCraft has no error value, and hiding the cell would lie.

use std::path::{Path, PathBuf};

use calamine::{Reader, Sheets, open_workbook_auto};
use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, StoragePort, window_slice,
};
use datacraft_core::schema::{FieldValue, TableDef};

/// One sheet read at open: its definition and its rows in file order.
#[derive(Debug)]
pub(crate) struct Sheet {
    pub(crate) definition: TableDef,
    pub(crate) rows: Vec<Vec<FieldValue>>,
}

/// One Excel workbook as a database.
#[derive(Debug, Default)]
pub struct ExcelPort {
    path: Option<PathBuf>,
    sheets: Vec<Sheet>,
}

impl ExcelPort {
    /// Creates an unattached driver.
    pub fn new() -> Self {
        Self::default()
    }

    /// The open workbook's path (tests and diagnostics).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl StoragePort for ExcelPort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("xlsx")
    }

    fn source_format(&self) -> Option<DatabaseFormat> {
        Some(DatabaseFormat::Excel)
    }

    /// The honest set: reads only. Creation, writes, passwords,
    /// relationships, AutoNumbers, inventories and compact stay absent.
    fn capabilities(&self) -> DriverCapabilities {
        let mut caps = DriverCapabilities::none();
        caps.read_tables = true;
        caps
    }

    fn open(&mut self, path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        let sheets = read_workbook(path)?;
        self.path = Some(path.to_path_buf());
        self.sheets = sheets;
        let name = path.file_stem().map_or_else(
            || "workbook".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        Ok(OpenedDatabase {
            name,
            format: DatabaseFormat::Excel,
            lock_file: None,
        })
    }

    fn close(&mut self) {
        self.path = None;
        self.sheets.clear();
    }

    fn tables(&self) -> Vec<TableDef> {
        self.sheets
            .iter()
            .map(|sheet| sheet.definition.clone())
            .collect()
    }

    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(read_only("create tables"))
    }

    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(read_only("drop tables"))
    }

    fn insert_row(&mut self, _table: &str, _values: &[FieldValue]) -> Result<(), PortError> {
        Err(read_only("insert rows"))
    }

    fn update_row(
        &mut self,
        _table: &str,
        _row: usize,
        _values: &[FieldValue],
    ) -> Result<(), PortError> {
        Err(read_only("update rows"))
    }

    fn delete_row(&mut self, _table: &str, _row: usize) -> Result<(), PortError> {
        Err(read_only("delete rows"))
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        let sheet = self
            .sheets
            .iter()
            .find(|sheet| sheet.definition.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?;
        let (start, end) = window_slice(&sheet.definition.name, range, sheet.rows.len())?;
        Ok(sheet.rows[start..end].to_vec())
    }

    fn row_count(&self, table: &str) -> Result<usize, PortError> {
        Ok(self
            .sheets
            .iter()
            .find(|sheet| sheet.definition.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?
            .rows
            .len())
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(Vec::new())
    }

    fn indexes(&self) -> Result<Vec<datacraft_core::port::SourceIndex>, PortError> {
        Ok(Vec::new())
    }

    fn triggers(&self) -> Result<Vec<datacraft_core::port::SourceTrigger>, PortError> {
        Ok(Vec::new())
    }

    fn native_objects(&self) -> Result<Vec<datacraft_core::port::NativeObject>, PortError> {
        Ok(Vec::new())
    }

    fn native_views(&self) -> Result<Vec<datacraft_core::port::NativeView>, PortError> {
        Ok(Vec::new())
    }

    fn native_queries(&self) -> Result<Vec<datacraft_core::port::NativeQuery>, PortError> {
        Ok(Vec::new())
    }
}

/// Reads every non-empty sheet of one workbook (the port's open and the
/// import reader share this path).
pub(crate) fn read_workbook(path: &Path) -> Result<Vec<Sheet>, PortError> {
    let mut workbook = open_workbook_auto(path).map_err(|error| open_error(path, &error))?;
    if matches!(&workbook, Sheets::Ods(_)) {
        return Err(PortError::Unsupported {
            reason: format!(
                "{}: ODF spreadsheets are a separate driver module (roadmap §8) — \
                 Excel workbooks only",
                path.display()
            ),
        });
    }
    let names = workbook.sheet_names();
    let mut sheets = Vec::with_capacity(names.len());
    for name in names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|error| PortError::OpenFailed {
                detail: format!("{}: sheet {name:?}: {error}", path.display()),
            })?;
        if let Some(sheet) = read_sheet(&name, &range)? {
            sheets.push(sheet);
        }
    }
    Ok(sheets)
}

mod error;
mod import;
mod sheet;

use error::{open_error, read_only};
use sheet::read_sheet;

pub use import::ExcelImport;

#[cfg(test)]
mod tests {
    use super::*;
    use datacraft_core::port::Support;

    #[test]
    fn the_driver_declares_read_only_and_names_its_family() {
        let port = ExcelPort::new();
        assert_eq!(port.driver_id().as_str(), "xlsx");
        assert_eq!(port.source_format(), Some(DatabaseFormat::Excel));
        let caps = port.capabilities();
        assert!(caps.read_tables);
        assert!(!caps.write_rows);
        assert!(!caps.create_table);
        assert!(!caps.create_database);
        assert_eq!(caps.compact, Support::No);
        assert_eq!(caps.relationships, Support::No);
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use datacraft_core::schema::{FieldDef, FieldKind, NumericKind};

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("excelFixture.xlsx")
    }

    #[test]
    fn open_close_and_path_round_trip() {
        let mut port = ExcelPort::new();
        assert!(port.path().is_none());
        port.open(&fixture(), None).expect("opens");
        assert_eq!(port.path(), Some(fixture().as_path()));
        port.close();
        assert!(port.path().is_none());
        assert!(port.tables().is_empty());
        assert!(matches!(
            port.read_rows("Items", RowRange { start: 0, len: 1 }),
            Err(PortError::TableNotFound(_))
        ));
    }

    #[test]
    fn every_write_call_refuses_precisely() {
        let mut port = ExcelPort::new();
        port.open(&fixture(), None).expect("opens");
        let definition = TableDef::new(
            "New",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        );
        assert!(matches!(
            port.create_table(&definition),
            Err(PortError::Unsupported { .. })
        ));
        assert!(matches!(
            port.drop_table("Items"),
            Err(PortError::Unsupported { .. })
        ));
        assert!(matches!(
            port.update_row("Items", 0, &[]),
            Err(PortError::Unsupported { .. })
        ));
        assert!(matches!(
            port.delete_row("Items", 0),
            Err(PortError::Unsupported { .. })
        ));
    }

    #[test]
    fn the_inventories_are_empty() {
        let mut port = ExcelPort::new();
        port.open(&fixture(), None).expect("opens");
        assert!(port.relationships().expect("relationships").is_empty());
        assert!(port.indexes().expect("indexes").is_empty());
        assert!(port.triggers().expect("triggers").is_empty());
        assert!(port.native_objects().expect("objects").is_empty());
        assert!(port.native_views().expect("views").is_empty());
        assert!(port.native_queries().expect("queries").is_empty());
    }

    #[test]
    fn a_malformed_workbook_answers_an_open_failure() {
        let path = std::env::temp_dir().join(format!(
            "datacraft-xlsx-malformed-{}.xlsx",
            std::process::id()
        ));
        std::fs::write(&path, b"not a workbook").expect("write probe");
        let mut port = ExcelPort::new();
        let error = port.open(&path, None).expect_err("malformed");
        let _ = std::fs::remove_file(&path);
        assert!(matches!(error, PortError::OpenFailed { .. }), "{error:?}");
    }
}
