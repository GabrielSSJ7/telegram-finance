//! Dates typed in chat: `dd/mm`, `dd/mm/aa` or `dd/mm/aaaa`.

use chrono::{Datelike, NaiveDate};

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
    fn short_date_hides_current_year() {
        let today = date(2026, 3, 10);
        assert_eq!(short_date(date(2026, 3, 5), today), "05/03");
        assert_eq!(short_date(date(2025, 12, 31), today), "31/12/2025");
    }
}
