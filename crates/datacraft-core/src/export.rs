//! Table export to files (phase 99, step 99.1; item M130; roadmap §11).
//!
//! One export is an explicit user action producing a **new** file: the
//! source database is only read, an existing target is refused, and the
//! bytes are deterministic per input state — no timestamps, no object
//! iteration order that a map could reorder, fixed row order.
//!
//! Two formats, two contracts:
//!
//! - **CSV** is the text form the datasheet shows: a header row with the
//!   field names in storage order, then one record per row. Cells follow
//!   the CSV driver's convention (`Null` empty, text as-is, everything
//!   else [`display_value`]) with one deviation: a decimal keeps the
//!   declared scale of its field (`2550` at scale 2 is `25.50`, not
//!   flattened) so the value survives a round trip through a CSV
//!   directory.
//! - **JSON** is the agent-first surface: the field-kind schema in the
//!   wire vocabulary plus typed rows. Numbers stay numbers, `Null` is
//!   `null`, booleans stay booleans, dates use the datasheet's
//!   `YYYY-MM-DD HH:MM` text, a binary cell is `{"binary": "<hex>"}`
//!   (lossless) and a complex cell is an array of its scalars.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::present::{display_value, display_value_for};
use crate::schema::{FieldKind, FieldValue, TableDef};

/// The output format of [`export_table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Comma-separated text with a header row (the `csv` writer, LF
    /// record terminators — a checkout's line-ending policy cannot
    /// change the bytes).
    Csv,
    /// One JSON document: table name, field-kind schema and typed rows.
    Json,
}

impl ExportFormat {
    /// The canonical file extension of the format (without the dot).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }

    /// The format's name as the UI and the wire show it.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Json => "JSON",
        }
    }
}

/// Why an export did not produce a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// A value has no JSON spelling (a non-finite float — JSON defines no
    /// `NaN`/`Infinity`).
    Unrepresentable(String),
    /// A row's arity differs from the table's field count (a port
    /// invariant broke; a document is never written ragged).
    Shape(String),
    /// The document could not be built or the target could not be
    /// created; the reason carries the path where one is involved.
    Write(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrepresentable(reason) | Self::Shape(reason) | Self::Write(reason) => {
                f.write_str(reason)
            }
        }
    }
}

/// Writes `rows` of `table` to a new file in `format`.
///
/// The source is read-only by construction (the caller hands in rows);
/// an existing target is never opened — the create is exclusive, so a
/// foreign file stays byte-identical and a failed export leaves no
/// partial file behind.
pub fn export_table(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    format: ExportFormat,
    destination: &Path,
) -> Result<(), ExportError> {
    let bytes = match format {
        ExportFormat::Csv => csv_document(table, rows)?,
        ExportFormat::Json => json_document(table, rows)?,
    };
    crate::atomic_write::write_new(destination, &bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            ExportError::Write(format!(
                "{}: the target file already exists",
                destination.display()
            ))
        } else {
            ExportError::Write(format!("{}: {error}", destination.display()))
        }
    })
}

/// The CSV document: header plus one record per row.
fn csv_document(table: &TableDef, rows: &[Vec<FieldValue>]) -> Result<Vec<u8>, ExportError> {
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer
        .write_record(table.fields.iter().map(|field| field.name.as_str()))
        .map_err(|error| ExportError::Write(format!("CSV document: {error}")))?;
    for (index, row) in rows.iter().enumerate() {
        if row.len() != table.fields.len() {
            return Err(shape_error(index, row.len(), table.fields.len()));
        }
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(column, value)| {
                let kind = table.fields.get(column).map(|field| &field.kind);
                csv_cell(kind, value)
            })
            .collect();
        writer
            .write_record(&cells)
            .map_err(|error| ExportError::Write(format!("CSV row {}: {error}", index + 1)))?;
    }
    writer
        .into_inner()
        .map_err(|error| ExportError::Write(format!("CSV document: {error}")))
}

/// One CSV cell: the datasheet's text, with a decimal under its declared
/// scale (phase 104, M135: the shared kind-aware helper).
fn csv_cell(kind: Option<&FieldKind>, value: &FieldValue) -> String {
    display_value_for(value, kind)
}

/// The JSON document: table name, field-kind schema and typed rows.
fn json_document(table: &TableDef, rows: &[Vec<FieldValue>]) -> Result<Vec<u8>, ExportError> {
    let mut typed_rows = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if row.len() != table.fields.len() {
            return Err(shape_error(index, row.len(), table.fields.len()));
        }
        let mut object = serde_json::Map::new();
        for (field, value) in table.fields.iter().zip(row) {
            let typed = json_value(&field.kind, value).map_err(|reason| {
                ExportError::Unrepresentable(format!(
                    "row {}, field {:?}: {reason}",
                    index + 1,
                    field.name
                ))
            })?;
            object.insert(field.name.clone(), typed);
        }
        typed_rows.push(serde_json::Value::Object(object));
    }
    let schema = table
        .fields
        .iter()
        .map(|field| SchemaEntry {
            name: &field.name,
            kind: &field.kind,
            required: field.properties.required,
        })
        .collect();
    let document = Document {
        table: &table.name,
        schema,
        rows: typed_rows,
    };
    serde_json::to_vec(&document)
        .map_err(|error| ExportError::Write(format!("JSON document: {error}")))
}

/// The top-level JSON document; the field order is the declared one.
#[derive(Serialize)]
struct Document<'a> {
    /// The exported table's canonical name.
    table: &'a str,
    /// One entry per field in storage order.
    schema: Vec<SchemaEntry<'a>>,
    /// One object per row, keyed by field name.
    rows: Vec<serde_json::Value>,
}

/// One schema entry: the field name, its kind in the wire vocabulary and
/// whether the field rejects `Null`.
#[derive(Serialize)]
struct SchemaEntry<'a> {
    /// Field name as the table stores it.
    name: &'a str,
    /// The field kind, serde-serialised exactly as the wire carries it.
    kind: &'a FieldKind,
    /// Whether the field is declared required.
    required: bool,
}

/// Maps one domain value onto its typed JSON form.
fn json_value(kind: &FieldKind, value: &FieldValue) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    Ok(match value {
        FieldValue::Null => Value::Null,
        FieldValue::Boolean(flag) => Value::Bool(*flag),
        FieldValue::Integer(integer) => Value::Number((*integer).into()),
        FieldValue::Float(float) => serde_json::Number::from_f64(*float)
            .map(Value::Number)
            .ok_or_else(|| format!("{float} is not a JSON number"))?,
        FieldValue::Currency(_) => number_from_text(&display_value(value))?,
        FieldValue::Decimal(_) => number_from_text(&display_value_for(value, Some(kind)))?,
        FieldValue::DateTime(_) => Value::String(display_value(value)),
        FieldValue::Text(text) => Value::String(text.clone()),
        FieldValue::Binary(bytes) => binary_object(bytes),
        FieldValue::Guid(_) => Value::String(display_value(value)),
        FieldValue::Complex(records) => Value::Array(
            records
                .iter()
                .filter_map(|record| record.scalar())
                .map(json_scalar)
                .collect::<Result<Vec<_>, String>>()?,
        ),
    })
}

/// A sub-record scalar (no field kind in reach): an exact integer when it
/// fits, otherwise the plain decimal text.
fn json_scalar(value: &FieldValue) -> Result<serde_json::Value, String> {
    match value {
        FieldValue::Decimal(scaled) => Ok(match i64::try_from(*scaled) {
            Ok(integer) => serde_json::Value::Number(integer.into()),
            Err(_) => serde_json::Value::String(scaled.to_string()),
        }),
        other => json_value(&FieldKind::Complex, other),
    }
}

/// A binary cell keeps its bytes as lowercase hex.
fn binary_object(bytes: &[u8]) -> serde_json::Value {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    serde_json::json!({ "binary": hex })
}

/// Parses exact decimal text into a JSON number.
fn number_from_text(text: &str) -> Result<serde_json::Value, String> {
    text.parse::<serde_json::Number>()
        .map(serde_json::Value::Number)
        .map_err(|_| format!("{text:?} is not a JSON number"))
}

/// A row whose arity differs from the table's field count.
fn shape_error(index: usize, found: usize, expected: usize) -> ExportError {
    ExportError::Shape(format!(
        "row {}: {found} values for {expected} fields",
        index + 1
    ))
}

#[cfg(test)]
mod tests {
    //! The text helpers, exercised directly (the document contracts run
    //! through the command bus in `tests/table_export.rs`).

    use super::*;

    #[test]
    fn the_format_names_are_the_canonical_extensions() {
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.display_name(), "CSV");
        assert_eq!(ExportFormat::Json.display_name(), "JSON");
    }
}
