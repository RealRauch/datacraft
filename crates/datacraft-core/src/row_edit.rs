//! Datasheet cell parsing (step 11.7).
//!
//! The datasheet's new-record row collects text; this module turns those
//! strings into typed domain values. Parsing is strict — a mistyped cell
//! never becomes a silent Null — and follows Access conventions: an empty
//! cell is Null unless the field requires a zero-length string, dates use
//! the OLE epoch 1899-12-30, and currency/decimal values are scaled
//! integers.

use crate::schema::{FieldDef, FieldKind, FieldValue, NumericKind};

/// Parses one cell against its field, applying Access's zero-length rule
/// (ADR-8): a blank entry is `Null`, except on a field with `Required`
/// **and** `AllowZeroLength` set, where it stores a zero-length string.
/// The literal `""` stores a zero-length string where allowed and is
/// rejected elsewhere.
pub fn parse_field_value(field: &FieldDef, text: &str) -> Result<FieldValue, String> {
    let text_field = matches!(field.kind, FieldKind::Text { .. } | FieldKind::Memo);
    let zero_length = text_field && field.properties.allow_zero_length;
    if text.trim().is_empty() {
        if zero_length && field.properties.required {
            return Ok(FieldValue::Text(String::new()));
        }
        return Ok(FieldValue::Null);
    }
    if text_field && text == "\"\"" {
        if zero_length {
            return Ok(FieldValue::Text(String::new()));
        }
        return Err(format!(
            "field {:?} does not allow zero-length strings",
            field.name
        ));
    }
    parse_value(&field.kind, text)
}

/// Parses one datasheet cell into a domain value of `kind`.
pub fn parse_value(kind: &FieldKind, text: &str) -> Result<FieldValue, String> {
    if text.trim().is_empty() {
        return Ok(FieldValue::Null);
    }
    match kind {
        FieldKind::Text { .. } | FieldKind::Memo => Ok(FieldValue::Text(text.to_owned())),
        FieldKind::Number(numeric) => parse_number(*numeric, text),
        FieldKind::DateTime => parse_datetime(text).map(FieldValue::DateTime),
        FieldKind::Currency => {
            let scaled = parse_scaled(text, 4)?;
            let value = i64::try_from(scaled)
                .map_err(|_| format!("currency value is out of range: {text:?}"))?;
            Ok(FieldValue::Currency(value))
        }
        FieldKind::AutoNumber(_) => {
            Err("AutoNumber values are engine-assigned — leave the cell empty".to_owned())
        }
        FieldKind::Boolean { .. } => parse_boolean(text).map(FieldValue::Boolean),
        FieldKind::Binary(_) => {
            Err("binary values cannot be typed into a datasheet cell".to_owned())
        }
        FieldKind::MultiValue(_) => {
            Err("multi-value values cannot be typed into a datasheet cell".to_owned())
        }
        FieldKind::Complex => {
            Err("complex values cannot be typed into a datasheet cell".to_owned())
        }
        FieldKind::Guid => parse_guid(text).map(FieldValue::Guid),
    }
}

/// Parses the full positional row of a table's new-record cells.
pub fn row_values(fields: &[FieldDef], cells: &[String]) -> Result<Vec<FieldValue>, String> {
    if cells.len() != fields.len() {
        return Err(format!(
            "expected {} cells, found {}",
            fields.len(),
            cells.len()
        ));
    }
    fields
        .iter()
        .zip(cells)
        .map(|(field, cell)| {
            parse_field_value(field, cell)
                .map_err(|reason| format!("field {:?}: {reason}", field.name))
        })
        .collect()
}

fn parse_number(numeric: NumericKind, text: &str) -> Result<FieldValue, String> {
    match numeric {
        NumericKind::Byte | NumericKind::Integer | NumericKind::LongInteger => {
            let parsed = text
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("expected an integer, found {text:?}"))?;
            numeric
                .validate_value(&FieldValue::Integer(parsed))
                .map_err(|error| error.to_string())?;
            Ok(FieldValue::Integer(parsed))
        }
        NumericKind::Single | NumericKind::Double => {
            let parsed = text
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("expected a number, found {text:?}"))?;
            if !parsed.is_finite() {
                return Err(format!(
                    "floating-point values must be finite, found {text:?}"
                ));
            }
            Ok(FieldValue::Float(parsed))
        }
        NumericKind::Decimal { precision, scale } => {
            let scaled = parse_scaled(text, scale)?;
            check_precision(scaled, precision)?;
            Ok(FieldValue::Decimal(scaled))
        }
    }
}

/// Parses a decimal string into a `scale`-scaled integer (no rounding:
/// excess fractional digits are rejected).
fn parse_scaled(text: &str, scale: u8) -> Result<i128, String> {
    let trimmed = text.trim();
    let (negative, unsigned) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let (int_part, frac_part) = match unsigned.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (unsigned, ""),
    };
    if (int_part.is_empty() && frac_part.is_empty())
        || !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err(format!("expected a number, found {text:?}"));
    }
    if frac_part.len() > usize::from(scale) {
        return Err(format!(
            "at most {scale} decimal places, found {}",
            frac_part.len()
        ));
    }
    let malformed = || format!("expected a number, found {text:?}");
    let integer: i128 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().map_err(|_| malformed())?
    };
    let padded = format!(
        "{frac_part}{}",
        "0".repeat(usize::from(scale) - frac_part.len())
    );
    let fraction: i128 = if padded.is_empty() {
        0
    } else {
        padded.parse().map_err(|_| malformed())?
    };
    let factor = 10i128
        .checked_pow(u32::from(scale))
        .ok_or_else(|| format!("value is out of range: {text:?}"))?;
    let magnitude = integer
        .checked_mul(factor)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| format!("value is out of range: {text:?}"))?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn check_precision(scaled: i128, precision: u8) -> Result<(), String> {
    let magnitude = scaled.unsigned_abs();
    let digits = if magnitude == 0 {
        1
    } else {
        magnitude.ilog10() + 1
    };
    if digits > u32::from(precision) {
        return Err(format!("at most {precision} digits, found {digits}"));
    }
    Ok(())
}

fn parse_boolean(text: &str) -> Result<bool, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" | "on" | "-1" => Ok(true),
        "false" | "no" | "n" | "0" | "off" => Ok(false),
        _ => Err(format!("expected a Yes/No value, found {text:?}")),
    }
}

fn parse_guid(text: &str) -> Result<[u8; 16], String> {
    let trimmed = text.trim().trim_start_matches('{').trim_end_matches('}');
    let hex: String = trimmed.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("expected a GUID, found {text:?}"));
    }
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| format!("expected a GUID, found {text:?}"))?;
    }
    Ok(bytes)
}

/// OLE Automation serial of an ISO date/time (`YYYY-MM-DD[ HH:MM[:SS]]`).
fn parse_datetime(text: &str) -> Result<f64, String> {
    let trimmed = text.trim();
    let (date_part, time_part) = match trimmed.split_once([' ', 'T']) {
        Some((date, time)) => (date, Some(time.trim())),
        None => (trimmed, None),
    };
    let days = parse_date(date_part)?;
    let fraction = match time_part {
        Some(time) => parse_time(time)?,
        None => 0.0,
    };
    Ok(days as f64 + fraction)
}

fn parse_date(text: &str) -> Result<i64, String> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("expected a date as YYYY-MM-DD, found {text:?}"));
    };
    let malformed = || format!("expected a date as YYYY-MM-DD, found {text:?}");
    let year: i64 = year.parse().map_err(|_| malformed())?;
    let month: u32 = month.parse().map_err(|_| malformed())?;
    let day: u32 = day.parse().map_err(|_| malformed())?;
    if !(1..=12).contains(&month) {
        return Err(format!("date month must be 1..=12, found {month}"));
    }
    let max_day = days_in_month(year, month);
    if !(1..=max_day).contains(&day) {
        return Err(format!(
            "date {year:04}-{month:02} has 1..={max_day} days, found {day}"
        ));
    }
    Ok(days_from_civil(year, month, day) + 25_569)
}

fn parse_time(text: &str) -> Result<f64, String> {
    let mut parts = text.split(':');
    let (Some(hours), Some(minutes), seconds, None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("expected a time as HH:MM[:SS], found {text:?}"));
    };
    let malformed = || format!("expected a time as HH:MM[:SS], found {text:?}");
    let hours: u32 = hours.parse().map_err(|_| malformed())?;
    let minutes: u32 = minutes.parse().map_err(|_| malformed())?;
    let seconds: u32 = match seconds {
        Some(seconds) => seconds.parse().map_err(|_| malformed())?,
        None => 0,
    };
    if hours > 23 {
        return Err(format!("time hours must be 0..=23, found {hours}"));
    }
    if minutes > 59 {
        return Err(format!("time minutes must be 0..=59, found {minutes}"));
    }
    if seconds > 59 {
        return Err(format!("time seconds must be 0..=59, found {seconds}"));
    }
    Ok(f64::from(hours * 3600 + minutes * 60 + seconds) / 86_400.0)
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

/// Days from 1970-01-01 (proleptic Gregorian; Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = i64::from((month + 9) % 12);
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 12), 31);
    }

    #[test]
    fn zero_decimal_values_are_one_digit() {
        assert!(check_precision(0, 1).is_ok());
        assert!(check_precision(0, 0).is_err());
    }

    #[test]
    fn complex_kinds_are_not_typeable_cells() {
        assert!(
            parse_value(
                &FieldKind::MultiValue(crate::schema::MultiValueKind::Text { max_len: 5 }),
                "a"
            )
            .is_err()
        );
        assert!(parse_value(&FieldKind::Complex, "a").is_err());
    }
}
