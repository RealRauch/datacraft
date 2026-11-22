//! Import File reader (phase 96, step 96.4; item M127).

use std::path::Path;

use datacraft_core::schema::FieldValue;

/// The Import File reader (phase 96, step 96.4; item M127): the first
/// sheet with data becomes the import table, its typed cells rendered
/// into the canonical text the M50 chain coerces (ISO dates, `true`/
/// `false` booleans, empty cells as `Null`). A multi-sheet workbook
/// imports its first data sheet — the command names one target table.
pub struct ExcelImport;

impl datacraft_core::import::ImportReader for ExcelImport {
    fn supports(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "xlsx" | "xlsm" | "xls" | "xlsb"
                )
            })
    }

    fn read(&self, path: &Path) -> Result<datacraft_core::import::ImportTable, String> {
        let sheets = crate::read_workbook(path).map_err(|error| error.to_string())?;
        let sheet = sheets.into_iter().next().ok_or_else(|| {
            format!(
                "{}: the workbook has no sheet with data to import",
                path.display()
            )
        })?;
        let rows = sheet
            .rows
            .iter()
            .map(|row| row.iter().map(import_text).collect())
            .collect();
        Ok(datacraft_core::import::ImportTable {
            fields: sheet.definition.fields,
            rows,
        })
    }
}

/// Canonical import text of one typed cell (the coercion parser's
/// expected spellings).
fn import_text(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => String::new(),
        FieldValue::Integer(integer) => integer.to_string(),
        FieldValue::Float(float) => format!("{float}"),
        FieldValue::DateTime(serial) => ole_datetime_text(*serial),
        FieldValue::Boolean(boolean) => boolean.to_string(),
        FieldValue::Text(text) => text.clone(),
        other => datacraft_core::present::display_value(other),
    }
}

/// ISO 8601 text of an OLE automation day value (the parser's
/// `YYYY-MM-DD[ HH:MM:SS]` shape); sub-second parts round to seconds.
fn ole_datetime_text(serial: f64) -> String {
    let days = serial.floor() as i64;
    let (year, month, day) = civil_from_ole_days(days);
    let fraction = serial - days as f64;
    if fraction.abs() < 1e-9 {
        return format!("{year:04}-{month:02}-{day:02}");
    }
    let seconds = (fraction * 86_400.0).round() as i64;
    let (minute, second) = ((seconds / 60) % 60, seconds % 60);
    let hour = (seconds / 3600) % 24;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

/// Civil date of an OLE automation day count (days since 1899-12-30;
/// Howard Hinnant's public-domain `civil_from_days`, shifted from the
/// 1970 epoch).
fn civil_from_ole_days(days: i64) -> (i64, u32, u32) {
    let z = days - 25_569 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ole_days_render_as_iso_text() {
        assert_eq!(ole_datetime_text(45_306.0), "2024-01-15");
        assert_eq!(ole_datetime_text(45_342.0), "2024-02-20");
        assert_eq!(ole_datetime_text(45_306.5), "2024-01-15 12:00:00");
        assert_eq!(ole_datetime_text(0.0), "1899-12-30");
    }
}
