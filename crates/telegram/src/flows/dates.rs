//! Dates typed in chat: `dd/mm`, `dd/mm/aa` or `dd/mm/aaaa`; times as
//! `21`, `21h`, `21:30` or `21h30`.

use chrono::{Datelike, NaiveDate, NaiveTime};

/// Parses a typed date; `dd/mm` uses the year of `today`.
///
/// ```
/// use chrono::NaiveDate;
/// use telegram::flows::dates::parse_typed_date;
/// let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
/// assert_eq!(parse_typed_date("5/3", today), NaiveDate::from_ymd_opt(2026, 3, 5));
/// ```
pub fn parse_typed_date(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = text.trim().split(['/', '-', '.']).collect();
    let (day, month) = (parts.first()?.parse().ok()?, parts.get(1)?.parse().ok()?);
    let year = match parts.get(2) {
        None => today.year(),
        Some(raw) => full_year(raw)?,
    };
    if parts.len() > 3 {
        return None;
    }
    NaiveDate::from_ymd_opt(year, month, day)
}

fn full_year(raw: &str) -> Option<i32> {
    let value: i32 = raw.parse().ok()?;
    match raw.len() {
        2 => Some(2000 + value),
        4 => Some(value),
        _ => None,
    }
}

/// Parses a typed time of day.
///
/// ```
/// use chrono::NaiveTime;
/// use telegram::flows::dates::parse_typed_time;
/// assert_eq!(parse_typed_time("21h30"), NaiveTime::from_hms_opt(21, 30, 0));
/// ```
pub fn parse_typed_time(text: &str) -> Option<NaiveTime> {
    let cleaned = text.trim().to_lowercase().replace('h', ":");
    let (hours, minutes) = cleaned.split_once(':').unwrap_or((cleaned.as_str(), ""));
    let minutes = if minutes.is_empty() { 0 } else { minutes.parse().ok()? };
    NaiveTime::from_hms_opt(hours.trim().parse().ok()?, minutes, 0)
}

/// `dd/mm`, adding the year only when it differs from `today`'s.
pub fn short_date(date: NaiveDate, today: NaiveDate) -> String {
    if date.year() == today.year() {
        return date.format("%d/%m").to_string();
    }
    date.format("%d/%m/%Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn parses_supported_shapes() {
        let today = date(2026, 3, 10);
        assert_eq!(parse_typed_date("05/03", today), Some(date(2026, 3, 5)));
        assert_eq!(parse_typed_date("31/12/25", today), Some(date(2025, 12, 31)));
        assert_eq!(parse_typed_date(" 1-2-2024 ", today), Some(date(2024, 2, 1)));
    }

    #[test]
    fn rejects_impossible_or_malformed_dates() {
        let today = date(2026, 3, 10);
        for text in ["31/02", "abc", "10", "1/2/3", "1/2/2024/5", "32/01", "1/13"] {
            assert_eq!(parse_typed_date(text, today), None, "{text}");
        }
    }

    #[test]
    fn parses_times_of_day() {
        let at = |hour, minute| NaiveTime::from_hms_opt(hour, minute, 0);
        assert_eq!(parse_typed_time("21"), at(21, 0));
        assert_eq!(parse_typed_time(" 21H "), at(21, 0));
        assert_eq!(parse_typed_time("9:05"), at(9, 5));
        assert_eq!(parse_typed_time("24:00"), None);
        assert_eq!(parse_typed_time("noite"), None);
    }

    #[test]
    fn short_date_hides_current_year() {
        let today = date(2026, 3, 10);
        assert_eq!(short_date(date(2026, 3, 5), today), "05/03");
        assert_eq!(short_date(date(2025, 12, 31), today), "31/12/2025");
    }
}
