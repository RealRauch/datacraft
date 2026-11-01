//! The `Format` engine of the expression service (step 15.1, item H41).
//!
//! Documented subset: named pictures `General Number`, `General Date`,
//! `Fixed`, `Standard`, `Percent`; numeric pictures with `0`/`#`
//! placeholders, thousands separator and `%` suffix; date pictures with
//! `yyyy yy mm m dd d hh h nn n ss s` tokens. Numeric rounding is half
//! away from zero (VBA `Format`); `Null` renders as the empty string.

use super::ExprError;
use super::coerce::{display, parse_num_text, runtime};
use crate::schema::FieldValue;

/// Entry point of the dispatch table (`format`).
pub(crate) fn format_value(
    value: &FieldValue,
    pattern: &FieldValue,
    name: &str,
) -> Result<FieldValue, ExprError> {
    let FieldValue::Text(pattern) = pattern else {
        return Err(runtime(format!(
            "{name} expects a text picture, found {}",
            pattern.family()
        )));
    };
    if matches!(value, FieldValue::Null) {
        return Ok(FieldValue::Text(String::new()));
    }
    Ok(FieldValue::Text(match value {
        FieldValue::DateTime(days) => format_date(*days, pattern),
        value => match pattern.to_lowercase().as_str() {
            "general number" => return Ok(FieldValue::Text(display(value))),
            "general date" => {
                return Ok(FieldValue::Text(crate::present::display_ole_datetime(
                    numeric_f64(value, name)?,
                )));
            }
            "fixed" => format_number(numeric_f64(value, name)?, 2, false, false),
            "standard" => format_number(numeric_f64(value, name)?, 2, true, false),
            "percent" => format_number(numeric_f64(value, name)? * 100.0, 2, false, true),
            picture if looks_like_date_picture(picture) => {
                format_date(numeric_f64(value, name)?, pattern)
            }
            picture => format_number(
                numeric_f64(value, name)?,
                decimals_of(picture),
                picture.contains(','),
                false,
            ),
        },
    }))
}

/// Numeric pictures carry at least one `0`/`#` placeholder.
fn looks_like_date_picture(picture: &str) -> bool {
    !picture.contains(['#', '0']) && picture.contains(['y', 'm', 'd', 'h', 'n', 's'])
}

fn decimals_of(picture: &str) -> usize {
    picture
        .split_once('.')
        .map(|(_, fraction)| fraction.chars().filter(|c| matches!(c, '0' | '#')).count())
        .unwrap_or(0)
}

/// Numeric value view for `Format` (text coerces, other families refuse).
fn numeric_f64(value: &FieldValue, name: &str) -> Result<f64, ExprError> {
    match value {
        FieldValue::Text(text) => Ok(f64::from(parse_num_text(text)?)),
        FieldValue::Integer(i) => Ok(*i as f64),
        FieldValue::Currency(c) => Ok(*c as f64 / 10_000.0),
        FieldValue::Float(f) | FieldValue::DateTime(f) => Ok(*f),
        FieldValue::Decimal(d) => Ok(*d as f64),
        other => Err(runtime(format!(
            "{name} cannot format {} values",
            other.family()
        ))),
    }
}

/// Numeric pictures: `0`/`#` decimals, optional thousands separator and
/// `%` suffix; rounding is half away from zero (VBA `Format`).
fn format_number(value: f64, decimals: usize, thousands: bool, percent: bool) -> String {
    let factor = 10f64.powi(decimals as i32);
    let rounded = (value * factor).round() / factor;
    let mut text = format!("{rounded:.decimals$}");
    if thousands {
        text = insert_thousands(text);
    }
    if percent {
        text.push('%');
    }
    text
}

fn insert_thousands(text: String) -> String {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.as_str()),
    };
    let (integer, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    let mut grouped = String::new();
    for (index, character) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    if fraction.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{fraction}")
    }
}

/// Date pictures: `yyyy yy mm m dd d hh h nn n ss s`, other characters
/// are literal.
fn format_date(days: f64, pattern: &str) -> String {
    let (year, month, day, hour, minute) = ole_components(days);
    let lowercase = pattern.to_lowercase();
    let chars: Vec<char> = lowercase.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let rest = &chars[i..];
        // Longer tokens first; `nn` is minutes (`m` is the month).
        let token = [
            ("yyyy", 4, i64::from(year)),
            ("yy", 2, i64::from(year % 100)),
            ("mm", 2, i64::from(month)),
            ("dd", 2, i64::from(day)),
            ("hh", 2, i64::from(hour)),
            ("nn", 2, i64::from(minute)),
            ("ss", 2, 0),
            ("m", 1, i64::from(month)),
            ("d", 1, i64::from(day)),
            ("h", 1, i64::from(hour)),
            ("n", 1, i64::from(minute)),
            ("s", 1, 0),
        ]
        .into_iter()
        .find(|(token, _, _)| rest.starts_with(&token.chars().collect::<Vec<_>>()));
        match token {
            Some((text, width, value)) => {
                out.push_str(&format!("{value:0width$}"));
                i += text.len();
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// (year, month, day, hour, minute) of an OLE Automation date; shared
/// with the date component functions in `funcs`.
pub(crate) fn ole_components(days: f64) -> (u32, u32, u32, u32, u32) {
    let whole = days.floor();
    let mut minutes = ((days - whole) * 1440.0).round() as i64;
    let mut date = whole as i64;
    if minutes >= 1440 {
        minutes -= 1440;
        date += 1;
    }
    let (year, month, day) = crate::present::civil_from_days(date - 25_569);
    (
        year as u32,
        month as u32,
        day as u32,
        (minutes / 60) as u32,
        (minutes % 60) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separate_only_the_integer_part() {
        assert_eq!(insert_thousands("1234567.89".into()), "1,234,567.89");
        assert_eq!(insert_thousands("-1234.5".into()), "-1,234.5");
        assert_eq!(insert_thousands("42".into()), "42");
    }

    #[test]
    fn date_pictures_render_documented_tokens() {
        assert_eq!(
            format_date(45_662.0 + 825.0 / 1440.0, "yyyy-mm-dd hh:nn"),
            "2025-01-05 13:45"
        );
        assert_eq!(format_date(45_662.0, "d.m.yy"), "5.1.25");
        // "ddd" is the "dd" token followed by the literal-day "d" token.
        assert_eq!(format_date(45_662.0, "ddd"), "055");
    }

    #[test]
    fn named_pictures_render_the_documented_shapes() {
        let fixed = format_value(&FieldValue::Float(4523.5), &text("Fixed"), "Format")
            .expect("fixed must format");
        assert_eq!(fixed, FieldValue::Text("4523.50".into()));
        let percent = format_value(&FieldValue::Float(0.5), &text("Percent"), "Format")
            .expect("percent must format");
        assert_eq!(percent, FieldValue::Text("50.00%".into()));
        let general =
            format_value(&FieldValue::Null, &text("Fixed"), "Format").expect("null formats");
        assert_eq!(general, FieldValue::Text(String::new()));
    }

    #[test]
    fn non_text_pictures_are_refused() {
        let error = format_value(&FieldValue::Integer(1), &FieldValue::Integer(2), "Format")
            .expect_err("picture must be text");
        assert!(error.to_string().contains("text picture"));
    }

    fn text(value: &str) -> FieldValue {
        FieldValue::Text(value.to_owned())
    }
}
