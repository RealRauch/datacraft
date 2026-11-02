//! Presentation helpers (step 5.1, item H17; ARCHITECTURE §8).
//!
//! The UI renders strings and models computed here so every piece of
//! presentation logic is unit-testable without a window; paint code stays
//! free of layout or business logic.

use crate::port::DatabaseFormat;
use crate::schema::FieldKind;
use crate::schema::FieldValue;
use crate::snapshot::UiStateSnapshot;

/// Status bar line for the given snapshot (step 12.4): the database
/// context, replaced by the active document's object when one is open —
/// the Access status-bar convention. `active` carries the tab snapshot's
/// `(object_kind, object_name)`.
pub fn status_line(snapshot: &UiStateSnapshot, active: Option<(&str, &str)>) -> String {
    let Some(db) = &snapshot.database else {
        return "No database open".to_owned();
    };
    match active {
        Some((_, name)) => format!("{} ({}) — {name}", db.name, db.format),
        None => {
            let count = db.tables.len();
            let noun = if count == 1 { "table" } else { "tables" };
            format!("{} ({}) — {count} {noun}", db.name, db.format)
        }
    }
}

impl std::fmt::Display for DatabaseFormat {
    /// The single rendering point for format families: the window title,
    /// the status bar and `status_line` all render through this path, so
    /// a new variant adds exactly one arm here (phase 30, item M17;
    /// ADR-7 data-driven rendering).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Jet3 => write!(f, "Jet3"),
            Self::Jet4 => write!(f, "Jet4"),
            Self::Ace => write!(f, "ACE"),
            Self::Sqlite => write!(f, "SQLite"),
        }
    }
}

/// Access UI type name of a field kind (the design grid's Data Type
/// column).
pub fn field_kind_display(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Text { .. } => "Short Text".to_owned(),
        FieldKind::Memo => "Long Text".to_owned(),
        FieldKind::Number(numeric) => match numeric {
            crate::schema::NumericKind::Byte => "Byte".to_owned(),
            crate::schema::NumericKind::Integer => "Integer".to_owned(),
            crate::schema::NumericKind::LongInteger => "Long Integer".to_owned(),
            crate::schema::NumericKind::Single => "Single".to_owned(),
            crate::schema::NumericKind::Double => "Double".to_owned(),
            crate::schema::NumericKind::Decimal { precision, scale } => {
                format!("Decimal ({precision}, {scale})")
            }
        },
        FieldKind::DateTime => "Date/Time".to_owned(),
        FieldKind::Currency => "Currency".to_owned(),
        FieldKind::AutoNumber(crate::schema::AutoKind::LongInteger) => "AutoNumber".to_owned(),
        FieldKind::AutoNumber(crate::schema::AutoKind::ReplicationId) => {
            "AutoNumber (Replication ID)".to_owned()
        }
        FieldKind::Boolean { .. } => "Yes/No".to_owned(),
        FieldKind::Binary(crate::schema::BinaryKind::OleObject) => "OLE Object".to_owned(),
        FieldKind::Binary(crate::schema::BinaryKind::Attachment) => "Attachment".to_owned(),
        FieldKind::MultiValue(_) => "Multi-Value".to_owned(),
        FieldKind::Complex => "Complex".to_owned(),
        FieldKind::Guid => "GUID".to_owned(),
    }
}

/// User-renderable text for one cell (Access-like conventions: `Yes`/`No`
/// booleans, currency with up to 4 scaled decimals, OLE dates as
/// `YYYY-MM-DD HH:MM`).
pub fn display_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => String::new(),
        FieldValue::Boolean(true) => "Yes".to_owned(),
        FieldValue::Boolean(false) => "No".to_owned(),
        FieldValue::Integer(i) => i.to_string(),
        FieldValue::Float(f) => f.to_string(),
        FieldValue::Currency(scaled) => display_currency(*scaled),
        FieldValue::DateTime(days) => display_ole_datetime(*days),
        FieldValue::Decimal(scaled) => scaled.to_string(),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Binary(bytes) => format!("<binary: {} bytes>", bytes.len()),
        FieldValue::Guid(bytes) => display_guid(bytes),
        // Complex cells render as the joined scalar of their sub-records:
        // attachment file names, multi-value elements, version-history text.
        FieldValue::Complex(records) => records
            .iter()
            .filter_map(|record| record.scalar())
            .map(display_value)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Currency is a scaled 64-bit integer (factor 10^-4); trailing zeros trim.
fn display_currency(scaled: i64) -> String {
    let sign = if scaled < 0 { "-" } else { "" };
    let abs = scaled.unsigned_abs();
    let whole = abs / 10_000;
    let frac = abs % 10_000;
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let mut frac = format!("{frac:04}");
        while frac.ends_with('0') {
            frac.pop();
        }
        format!("{sign}{whole}.{frac}")
    }
}

/// OLE Automation date (days since 1899-12-30; 25569 days to the Unix
/// epoch) rendered via the civil-from-days algorithm; shared with the
/// expression service's `Format` ("General Date").
pub(crate) fn display_ole_datetime(days: f64) -> String {
    let whole = days.floor();
    let mut minutes = ((days - whole) * 1440.0).round() as i64;
    let mut day = whole as i64;
    if minutes >= 1440 {
        minutes -= 1440;
        day += 1;
    }
    let (year, month, mday) = civil_from_days(day - 25_569);
    format!(
        "{year:04}-{month:02}-{mday:02} {:02}:{:02}",
        minutes / 60,
        minutes % 60
    )
}

/// Days since 1970-01-01 -> (year, month, day); shared with the
/// expression service's date components.
pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 16 bytes as the canonical 8-4-4-4-12 lowercase hex form.
fn display_guid(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Access toolbox name of a control kind (the designer's palette label).
pub fn control_kind_display(kind: &crate::form::ControlKind) -> &'static str {
    match kind {
        crate::form::ControlKind::Label { .. } => "Label",
        crate::form::ControlKind::TextBox { .. } => "Text Box",
        crate::form::ControlKind::CheckBox { .. } => "Check Box",
        crate::form::ControlKind::ComboBox { .. } => "Combo Box",
        crate::form::ControlKind::Button { .. } => "Button",
        crate::form::ControlKind::SubForm { .. } => "Subform",
    }
}
