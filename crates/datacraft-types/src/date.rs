//! OLE Automation date arithmetic (step 7.3, item H27).
//!
//! Access dates are float days since 1899-12-30 (25569 days after the Unix
//! epoch). The civil-date conversions are the shared primitives of the
//! criteria parser's date literals, the expression service's `DateSerial`
//! and its date-component functions, and the value renderer.

use crate::FieldValue;

/// Days since 1970-01-01 for a civil date (inverse of
/// [`civil_from_days`]).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days since 1970-01-01 -> (year, month, day).
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
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

/// Parses `YYYY-MM-DD` (optionally ` HH:MM`) into OLE Automation days.
pub fn parse_date(text: &str) -> Option<FieldValue> {
    let (date, time) = match text.split_once(' ') {
        Some((d, t)) => (d, Some(t)),
        None => (text, None),
    };
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut days = (days_from_civil(year, month, day) + 25_569) as f64;
    if let Some(time) = time {
        let mut parts = time.split(':');
        let hour: i64 = parts.next()?.parse().ok()?;
        let minute: i64 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(0..24).contains(&hour) || !(0..60).contains(&minute) {
            return None;
        }
        days += (hour * 60 + minute) as f64 / 1440.0;
    }
    Some(FieldValue::DateTime(days))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Unix epoch and one OLE day are the fixed anchors of the
    /// conversion; both directions invert each other.
    #[test]
    fn civil_and_ole_days_invert_the_epoch_anchors() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(1899, 12, 30), -25_569);
        assert_eq!(civil_from_days(-25_569), (1899, 12, 30));
        assert_eq!(
            parse_date("1970-01-01"),
            Some(FieldValue::DateTime(25_569.0))
        );
    }

    /// Date literals accept an optional `HH:MM` tail and reject malformed
    /// or out-of-range components precisely.
    #[test]
    fn date_literals_accept_time_and_reject_malformed_parts() {
        assert_eq!(
            parse_date("1899-12-30 12:00"),
            Some(FieldValue::DateTime(0.5))
        );
        for invalid in [
            "2025-13-01",
            "2025-00-01",
            "2025-01-32",
            "2025-01-01 25:00",
            "2025-01-01 12:60",
            "2025-01",
            "2025-01-01-05",
            "nope",
            "2025-01-01 12",
        ] {
            assert_eq!(parse_date(invalid), None, "{invalid}");
        }
    }
}
