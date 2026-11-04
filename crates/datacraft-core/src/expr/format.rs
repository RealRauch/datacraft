//! The `Format` engine of the expression service (step 15.1, item H41;
//! named date pictures step 39.1, item M35).
//!
//! Documented subset: named pictures `General Number`, `General Date`,
//! `Long Date`, `Medium Date`, `Short Date`, `Long Time`, `Medium Time`,
//! `Fixed`, `Standard`, `Percent` — the date/time shapes are deterministic
//! English (`Short Date` `mm/dd/yyyy`, `Medium Date` `dd-mmm-yy`,
//! `Long Date` `Weekday, Month D, YYYY`, `Long Time` `h:nn:ss AM/PM`,
//! `Medium Time` `h:nn AM/PM`), because the engine is locale-free
//! (deterministic values, LESSONS #6); numeric pictures with `0`/`#`
//! placeholders, thousands separator and `%` suffix; date pictures with
//! `yyyy yy mm m dd d hh h nn n ss s` tokens. Numeric rounding is half
//! away from zero (VBA `Format`); `Null` renders as the empty string;
//! a word picture outside the named table answers a precise error instead
//! of falling into the token heuristic.

use super::ExprError;
use super::coerce::{display, parse_num_text, runtime};
use crate::schema::FieldValue;

/// The named pictures of the documented subset, matched case-insensitively
/// and resolved for every value family (step 39.1, item M35).
const NAMED_PICTURES: [&str; 10] = [
    "general number",
    "general date",
    "long date",
    "medium date",
    "short date",
    "long time",
    "medium time",
    "fixed",
    "standard",
    "percent",
];

/// The alphabetic tokens of the date-picture grammar; any other letter run
/// makes a picture unsupported.
const DATE_TOKENS: [&str; 12] = [
    "yyyy", "yy", "mm", "dd", "hh", "nn", "ss", "m", "d", "h", "n", "s",
];

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
    let picture = pattern.to_lowercase();
    // Named pictures resolve first, for every value family — including
    // DateTime, which previously never reached the named table (M35).
    if NAMED_PICTURES.contains(&picture.as_str()) {
        return Ok(FieldValue::Text(match picture.as_str() {
            "general number" => display(value),
            "general date" => crate::present::display_ole_datetime(numeric_f64(value, name)?),
            "fixed" => format_number(numeric_f64(value, name)?, 2, false, false),
            "standard" => format_number(numeric_f64(value, name)?, 2, true, false),
            "percent" => format_number(numeric_f64(value, name)? * 100.0, 2, false, true),
            "short date" => format_date(numeric_f64(value, name)?, "mm/dd/yyyy"),
            "medium date" => {
                let (year, month, day, _, _) = ole_components(numeric_f64(value, name)?);
                format!("{day:02}-{}-{:02}", month_abbreviation(month), year % 100)
            }
            "long date" => format_long_date(numeric_f64(value, name)?),
            "long time" => format_time(numeric_f64(value, name)?, true),
            "medium time" => format_time(numeric_f64(value, name)?, false),
            _ => unreachable!("the NAMED_PICTURES table is exhaustive"),
        }));
    }
    // A letter run that is not a date token is a word picture outside the
    // subset (`Currency`, `AM/PM`, …) — precise error, never token garbage.
    if !is_token_picture(&picture) {
        return Err(runtime(format!(
            "{name} does not support the picture {pattern:?}; the documented \
             subset is the named pictures plus the date/numeric token pictures"
        )));
    }
    Ok(FieldValue::Text(match value {
        FieldValue::DateTime(days) => format_date(*days, pattern),
        value => match looks_like_date_picture(&picture) {
            true => format_date(numeric_f64(value, name)?, pattern),
            false => format_number(
                numeric_f64(value, name)?,
                decimals_of(&picture),
                picture.contains(','),
                false,
            ),
        },
    }))
}

/// Whether every alphabetic run of `picture` is one of the documented date
/// tokens (`d.m.yy` yes, `AM/PM` no).
fn is_token_picture(picture: &str) -> bool {
    let mut run = String::new();
    for character in picture.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_alphabetic() {
            run.push(character);
        } else if !run.is_empty() {
            if !DATE_TOKENS.contains(&run.as_str()) {
                return false;
            }
            run.clear();
        }
    }
    true
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

/// English month names for the named date pictures (the engine is
/// locale-free; LESSONS #6).
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// English weekday names, indexed with `OLE day 0 (1899-12-30) = Saturday`.
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// The three-letter month abbreviation of `month` (1-based).
fn month_abbreviation(month: u32) -> &'static str {
    &MONTHS[(month - 1) as usize][..3]
}

/// Access `Long Date`: `Weekday, Month D, YYYY` (deterministic English).
fn format_long_date(days: f64) -> String {
    let (year, month, day, _, _) = ole_components(days);
    let weekday = WEEKDAYS[(days.floor() as i64 + 6).rem_euclid(7) as usize];
    format!("{weekday}, {} {day}, {year}", MONTHS[(month - 1) as usize])
}

/// Access `Long Time` (`h:nn:ss AM/PM`) and `Medium Time` (`h:nn AM/PM`),
/// deterministic English with a 12-hour clock.
fn format_time(days: f64, seconds: bool) -> String {
    let whole = days.floor();
    let mut secs = ((days - whole) * 86_400.0).round() as i64;
    if secs >= 86_400 {
        secs -= 86_400;
    }
    let (hour, minute, second) = (
        (secs / 3_600) as u32,
        ((secs % 3_600) / 60) as u32,
        (secs % 60) as u32,
    );
    let (clock, suffix) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };
    if seconds {
        format!("{clock}:{minute:02}:{second:02} {suffix}")
    } else {
        format!("{clock}:{minute:02} {suffix}")
    }
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

    /// Step 39.1 (item M35): the named date/time pictures resolve for
    /// DateTime values too — `General Date` matches the shared datetime
    /// renderer, the Access shapes render deterministically.
    #[test]
    fn named_date_pictures_render_on_datetime_values() {
        let days = 45_662.0 + 825.0 / 1440.0; // 2025-01-05 13:45
        let general = format_value(&FieldValue::DateTime(days), &text("General Date"), "Format")
            .expect("general date must format");
        assert_eq!(
            general,
            FieldValue::Text(crate::present::display_ole_datetime(days)),
            "General Date matches the shared datetime renderer"
        );
        for (picture, expected) in [
            ("Short Date", "01/05/2025"),
            ("Medium Date", "05-Jan-25"),
            ("Long Date", "Sunday, January 5, 2025"),
            ("Long Time", "1:45:00 PM"),
            ("Medium Time", "1:45 PM"),
        ] {
            let rendered = format_value(&FieldValue::DateTime(days), &text(picture), "Format")
                .unwrap_or_else(|error| panic!("{picture} must format: {error}"));
            assert_eq!(rendered, FieldValue::Text(expected.to_owned()), "{picture}");
        }
    }

    /// Step 39.1 (item M35): the named-picture table matches
    /// case-insensitively.
    #[test]
    fn named_pictures_are_case_insensitive() {
        let short = format_value(
            &FieldValue::DateTime(45_662.0),
            &text("SHORT DATE"),
            "Format",
        )
        .expect("the picture match is case-insensitive");
        assert_eq!(short, FieldValue::Text("01/05/2025".into()));
    }

    /// Step 39.1 (item M35): a word picture outside the named table answers
    /// a precise error instead of token-heuristic garbage.
    #[test]
    fn unknown_word_pictures_error_precisely() {
        for picture in ["Currency", "hh:nn AM/PM"] {
            let error = format_value(&FieldValue::Integer(7), &text(picture), "Format")
                .expect_err("{picture} is not in the documented subset");
            assert!(error.to_string().contains(picture), "{error}");
        }
    }

    fn text(value: &str) -> FieldValue {
        FieldValue::Text(value.to_owned())
    }

    /// Every named picture resolves for a plain numeric value — the
    /// dispatch table covers date/time/number spellings (M35).
    #[test]
    fn named_pictures_cover_the_dispatch_table() {
        let value = FieldValue::Float(45_000.75);
        for picture in [
            "general number",
            "general date",
            "fixed",
            "standard",
            "percent",
            "short date",
            "medium date",
            "long date",
            "long time",
            "medium time",
        ] {
            let result = format_value(&value, &text(picture), "Format")
                .unwrap_or_else(|e| panic!("{picture} must format: {e}"));
            assert!(
                matches!(result, FieldValue::Text(ref rendered) if !rendered.is_empty()),
                "{picture} renders non-empty text, got {result:?}"
            );
        }
    }

    /// Numeric value views: text coerces, currency and decimal enter the
    /// tower, non-numeric families refuse precisely.
    #[test]
    fn numeric_view_covers_text_currency_decimal_and_refusals() {
        assert_eq!(
            format_value(&text("12.5"), &text("fixed"), "Format"),
            Ok(FieldValue::Text("12.50".to_owned()))
        );
        assert_eq!(
            format_value(&FieldValue::Currency(50_000), &text("fixed"), "Format"),
            Ok(FieldValue::Text("5.00".to_owned()))
        );
        assert_eq!(
            format_value(&FieldValue::Decimal(125), &text("fixed"), "Format"),
            Ok(FieldValue::Text("125.00".to_owned()))
        );
        let error = format_value(&FieldValue::Guid([0; 16]), &text("fixed"), "Format")
            .expect_err("guids do not format as numbers");
        assert!(error.to_string().contains("cannot format"), "{error}");
    }

    /// A time past the last whole second of a day rolls over into the
    /// next day's 00:00 instead of leaking second 86400.
    #[test]
    fn time_rolls_over_rounded_seconds() {
        assert_eq!(
            format_value(
                &FieldValue::Float(1.999_999_999_9),
                &text("long time"),
                "Format"
            ),
            Ok(FieldValue::Text("12:00:00 AM".to_owned()))
        );
    }
}
