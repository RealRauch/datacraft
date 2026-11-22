//! Sheet → table mapping (phase 96, step 96.2; item M127): header names,
//! conservative kind inference and typed cell conversion.

use calamine::Data;
use datacraft_core::port::PortError;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

use crate::Sheet;


/// Reads one sheet into a table definition plus domain rows; a sheet
/// without any used cell is not a table (`None`).
pub(crate) fn read_sheet(
    name: &str,
    range: &calamine::Range<Data>,
) -> Result<Option<Sheet>, PortError> {
    let mut rows: Vec<&[Data]> = range.rows().collect();
    // Trailing all-empty rows (formatting often extends the used range)
    // carry no data and would become thousands of Null rows.
    while rows
        .last()
        .is_some_and(|row| row.iter().all(|cell| matches!(cell, Data::Empty)))
    {
        rows.pop();
    }
    if rows.is_empty() {
        return Ok(None);
    }
    let header = rows[0];
    let width = header
        .iter()
        .rposition(|cell| !matches!(cell, Data::Empty))
        .map_or(0, |index| index + 1);
    if width == 0 {
        return Ok(None);
    }
    let data = &rows[1..];
    let names = column_names(header, width, name)?;
    let kinds = (0..width)
        .map(|column| infer_kind(data, column))
        .collect::<Vec<_>>();
    let fields = names
        .into_iter()
        .zip(kinds)
        .map(|(name, kind)| FieldDef::new(name, kind))
        .collect::<Vec<_>>();
    let definition = TableDef::new(name, fields);
    definition
        .validate()
        .map_err(|error| PortError::Unsupported {
            reason: format!("sheet {name:?}: {error}"),
        })?;
    let mut values = Vec::with_capacity(data.len());
    for (row_index, row) in data.iter().enumerate() {
        let mut converted = Vec::with_capacity(definition.fields.len());
        for (column, field) in definition.fields.iter().enumerate() {
            let cell = row.get(column).unwrap_or(&Data::Empty);
            converted.push(convert_cell(cell, &field.kind).map_err(|reason| {
                PortError::Unsupported {
                    reason: format!(
                        "sheet {name:?}, row {}, column {:?}: {reason}",
                        row_index + 2,
                        field.name
                    ),
                }
            })?);
        }
        values.push(converted);
    }
    Ok(Some(Sheet {
        definition,
        rows: values,
    }))
}

/// Header names for the used width; empty headers become `Column<N>`
/// (1-based) and duplicates fail precisely (DataCraft compares field
/// names case-insensitively).
fn column_names(header: &[Data], width: usize, sheet: &str) -> Result<Vec<String>, PortError> {
    let mut names = Vec::with_capacity(width);
    for column in 0..width {
        let name = match header.get(column) {
            Some(Data::String(text)) if !text.trim().is_empty() => text.trim().to_owned(),
            Some(Data::Float(number)) if (*number).fract() == 0.0 => (*number as i64).to_string(),
            Some(Data::Int(number)) => number.to_string(),
            _ => format!("Column{}", column + 1),
        };
        if names
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&name))
        {
            return Err(PortError::Unsupported {
                reason: format!("sheet {sheet:?}: duplicate column name {name:?}"),
            });
        }
        names.push(name);
    }
    Ok(names)
}

/// Infers one column's domain kind from every non-empty cell.
pub(crate) fn infer_kind(rows: &[&[Data]], column: usize) -> FieldKind {
    let mut ints = false;
    let mut floats = false;
    let mut dates = false;
    let mut bools = false;
    let mut texts = false;
    let mut longest = 0usize;
    for row in rows {
        match row.get(column) {
            None | Some(Data::Empty) => {}
            Some(Data::Int(_)) => ints = true,
            // Excel's OpenXML families deliver every number as a float;
            // an integral float votes for the integer family, so the same
            // sheet infers the same kind across .xlsx/.xls/.xlsb.
            Some(Data::Float(value)) if (*value).fract() == 0.0 => ints = true,
            Some(Data::Float(_)) => floats = true,
            Some(Data::DateTime(_)) => dates = true,
            Some(Data::Bool(_)) => bools = true,
            Some(Data::String(text)) => {
                texts = true;
                longest = longest.max(text.chars().count());
            }
            Some(Data::DateTimeIso(text) | Data::DurationIso(text)) => {
                texts = true;
                longest = longest.max(text.chars().count());
            }
            Some(Data::Error(error)) => {
                texts = true;
                longest = longest.max(error.to_string().chars().count());
            }
        }
    }
    match (ints, floats, dates, bools, texts) {
        (true, false, false, false, false) => FieldKind::Number(NumericKind::LongInteger),
        (true, true, false, false, false) => FieldKind::Number(NumericKind::Double),
        (false, true, false, false, false) => FieldKind::Number(NumericKind::Double),
        (false, false, true, false, false) => FieldKind::DateTime,
        (false, false, false, true, false) => FieldKind::Boolean { tri_state: false },
        _ if longest > 255 => FieldKind::Memo,
        _ => FieldKind::Text { max_len: 255 },
    }
}

/// Converts one cell into the column kind's value; `Empty` is `Null`.
fn convert_cell(cell: &Data, kind: &FieldKind) -> Result<FieldValue, String> {
    if matches!(cell, Data::Empty) {
        return Ok(FieldValue::Null);
    }
    match kind {
        FieldKind::Number(NumericKind::LongInteger) => match cell {
            Data::Int(value) => Ok(FieldValue::Integer(*value)),
            Data::Float(value) if (*value).fract() == 0.0 && value.is_finite() => {
                Ok(FieldValue::Integer(*value as i64))
            }
            other => Err(format!("expected an integer, found {other:?}")),
        },
        FieldKind::Number(NumericKind::Double) => match cell {
            Data::Int(value) => Ok(FieldValue::Float(*value as f64)),
            Data::Float(value) => Ok(FieldValue::Float(*value)),
            other => Err(format!("expected a number, found {other:?}")),
        },
        FieldKind::DateTime => match cell {
            // Excel's serial and OLE automation days agree for dates from
            // 1900-03-01 on (the 1904 date system is normalized by
            // calamine); pre-1900 values are out of scope.
            Data::DateTime(value) => Ok(FieldValue::DateTime(value.as_f64())),
            other => Err(format!("expected a date/time, found {other:?}")),
        },
        FieldKind::Boolean { .. } => match cell {
            Data::Bool(value) => Ok(FieldValue::Boolean(*value)),
            other => Err(format!("expected a boolean, found {other:?}")),
        },
        FieldKind::Memo | FieldKind::Text { .. } => Ok(FieldValue::Text(cell_text(cell))),
        other => Err(format!(
            "the column kind {other:?} cannot hold an Excel cell"
        )),
    }
}

/// Deterministic text form of one cell (the Text/Memo fallback); error
/// cells keep their Excel label instead of disappearing.
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(text) | Data::DateTimeIso(text) | Data::DurationIso(text) => text.clone(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => datacraft_core::present::display_value(&FieldValue::Float(*value)),
        Data::Bool(value) => datacraft_core::present::display_value(&FieldValue::Boolean(*value)),
        Data::DateTime(value) => {
            datacraft_core::present::display_value(&FieldValue::DateTime(value.as_f64()))
        }
        Data::Error(error) => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_inference_is_conservative() {
        let data: Vec<Vec<Data>> = vec![
            vec![Data::Int(1)],
            vec![Data::Int(2)],
            vec![Data::Float(1.5)],
            vec![Data::Bool(true)],
            vec![Data::String("x".into())],
            vec![Data::DateTime(calamine::ExcelDateTime::new(
                45_000.0,
                calamine::ExcelDateTimeType::DateTime,
                false,
            ))],
        ];
        let rows: Vec<&[Data]> = data.iter().map(Vec::as_slice).collect();
        assert_eq!(
            infer_kind(&rows, 0),
            FieldKind::Text { max_len: 255 },
            "mixed families stay text"
        );
        assert_eq!(
            infer_kind(&rows[..2], 0),
            FieldKind::Number(NumericKind::LongInteger)
        );
        assert_eq!(
            infer_kind(&rows[..3], 0),
            FieldKind::Number(NumericKind::Double)
        );
        assert_eq!(
            infer_kind(&rows[3..4], 0),
            FieldKind::Boolean { tri_state: false }
        );
    }
}
