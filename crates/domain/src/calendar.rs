//! Month arithmetic with day clamping (Jan 31 + 1 month = Feb 28/29).

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DayOfMonth;

/// A calendar month. Valid for years 1..=9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct YearMonth {
    year: i32,
    month: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid year-month {year}-{month}: expected year 1..=9999 and month 1..=12")]
pub struct YearMonthError {
    pub year: i32,
    pub month: u32,
}

impl YearMonth {
    /// ```
    /// use domain::YearMonth;
    /// assert!(YearMonth::new(2026, 13).is_err());
    /// ```
    pub fn new(year: i32, month: u32) -> Result<Self, YearMonthError> {
        let valid = (1..=9999).contains(&year) && (1..=12).contains(&month);
        if !valid {
            return Err(YearMonthError { year, month });
        }
        Ok(Self { year, month })
    }

    pub fn of(date: NaiveDate) -> Self {
        Self { year: date.year(), month: date.month() }
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month(self) -> u32 {
        self.month
    }

    /// Shifts by `months` (negative goes back).
    ///
    /// ```
    /// use domain::YearMonth;
    /// let jan = YearMonth::new(2026, 1).unwrap();
    /// assert_eq!(jan.plus_months(-1), YearMonth::new(2025, 12).unwrap());
    /// ```
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    pub fn plus_months(self, months: i32) -> Self {
        let index = self.year * 12 + (self.month as i32 - 1) + months;
        Self { year: index.div_euclid(12), month: index.rem_euclid(12) as u32 + 1 }
    }

    pub fn next(self) -> Self {
        self.plus_months(1)
    }

    /// Whole months from `self` to `later` (negative when `later` is earlier).
    ///
    /// ```
    /// use domain::YearMonth;
    /// let month = |year, month| YearMonth::new(year, month).unwrap();
    /// assert_eq!(month(2026, 11).months_until(month(2027, 2)), 3);
    /// ```
    #[allow(clippy::cast_possible_wrap)]
    pub fn months_until(self, later: YearMonth) -> i32 {
        (later.year - self.year) * 12 + (later.month as i32 - self.month as i32)
    }

    pub fn prev(self) -> Self {
        self.plus_months(-1)
    }

    pub fn last_day(self) -> u32 {
        match self.month {
            2 if is_leap_year(self.year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    pub fn first_day(self) -> NaiveDate {
        self.clamped_date(1)
    }

    /// The given day in this month, or the month's last day if shorter.
    pub fn clamped(self, day: DayOfMonth) -> NaiveDate {
        self.clamped_date(u32::from(day.get()))
    }

    fn clamped_date(self, day: u32) -> NaiveDate {
        valid_date(self.year, self.month, day.min(self.last_day()))
    }
}

impl std::fmt::Display for YearMonth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:02}/{}", self.month, self.year)
    }
}

/// Same day `months` later, clamped to the target month's last day.
/// Always computed from the original date, never chained, so Jan 31 gives
/// Feb 28 then Mar 31 (not Mar 28).
///
/// ```
/// use chrono::NaiveDate;
/// use domain::calendar::add_months_clamped;
/// let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
/// assert_eq!(add_months_clamped(jan31, 2), NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
/// ```
pub fn add_months_clamped(date: NaiveDate, months: u32) -> NaiveDate {
    let shift = i32::try_from(months).unwrap_or(i32::MAX / 24);
    YearMonth::of(date).plus_months(shift).clamped_date(date.day())
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Invariant: callers pass a month 1..=12 and a day already clamped to the
/// month length, so chrono cannot reject the date for years 1..=9999.
#[allow(clippy::expect_used)]
fn valid_date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("year-month-day built from clamped components must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ym(year: i32, month: u32) -> YearMonth {
        YearMonth::new(year, month).unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn plus_months_crosses_years_both_ways() {
        assert_eq!(ym(2026, 11).plus_months(3), ym(2027, 2));
        assert_eq!(ym(2026, 2).plus_months(-14), ym(2024, 12));
        assert_eq!(ym(2026, 12).next(), ym(2027, 1));
        assert_eq!(ym(2026, 1).prev(), ym(2025, 12));
    }

    #[test]
    fn last_day_handles_leap_years() {
        assert_eq!(ym(2024, 2).last_day(), 29);
        assert_eq!(ym(2026, 2).last_day(), 28);
        assert_eq!(ym(2000, 2).last_day(), 29);
        assert_eq!(ym(1900, 2).last_day(), 28);
        assert_eq!(ym(2026, 4).last_day(), 30);
        assert_eq!(ym(2026, 12).last_day(), 31);
    }

    #[test]
    fn clamped_uses_month_end_for_short_months() {
        let day31 = DayOfMonth::new(31).unwrap();
        assert_eq!(ym(2026, 2).clamped(day31), date(2026, 2, 28));
        assert_eq!(ym(2026, 3).clamped(day31), date(2026, 3, 31));
        assert_eq!(ym(2026, 3).first_day(), date(2026, 3, 1));
    }

    #[test]
    fn add_months_is_not_chained() {
        let jan31 = date(2026, 1, 31);
        assert_eq!(add_months_clamped(jan31, 0), jan31);
        assert_eq!(add_months_clamped(jan31, 1), date(2026, 2, 28));
        assert_eq!(add_months_clamped(jan31, 2), date(2026, 3, 31));
        assert_eq!(add_months_clamped(jan31, 13), date(2027, 2, 28));
    }

    #[test]
    fn rejects_invalid_year_month() {
        assert_eq!(YearMonth::new(2026, 0), Err(YearMonthError { year: 2026, month: 0 }));
        assert!(YearMonth::new(0, 5).is_err());
    }

    #[test]
    fn displays_as_mm_yyyy() {
        assert_eq!(ym(2026, 3).to_string(), "03/2026");
    }
}
