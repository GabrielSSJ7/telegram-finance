//! When a monthly recurring entry (salary, rent, subscriptions) is due.

use chrono::{Days, NaiveDate};

use crate::{DayOfMonth, YearMonth};

/// Backfill limit after downtime: a server off for a year should not
/// flood the ledger with twelve rent payments at once.
pub const MAX_BACKFILL_MONTHS: i32 = 3;

/// Due dates not generated yet, oldest first: every month's `day`
/// (clamped to short months) after `last_generated` (or from
/// `starts_on`) up to and including `today`.
///
/// ```
/// use chrono::NaiveDate;
/// use domain::{DayOfMonth, recurrence::due_dates};
/// let date = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
/// let due = due_dates(DayOfMonth::new(5).unwrap(), date(1, 1), Some(date(2, 5)), date(4, 10));
/// assert_eq!(due, vec![date(3, 5), date(4, 5)]);
/// ```
pub fn due_dates(
    day: DayOfMonth,
    starts_on: NaiveDate,
    last_generated: Option<NaiveDate>,
    today: NaiveDate,
) -> Vec<NaiveDate> {
    let after_last = last_generated.and_then(|last| last.checked_add_days(Days::new(1)));
    let from = after_last.map_or(starts_on, |next| next.max(starts_on));
    let first_month =
        YearMonth::of(from).max(YearMonth::of(today).plus_months(-MAX_BACKFILL_MONTHS));
    let months = (0..=MAX_BACKFILL_MONTHS).map(|offset| first_month.plus_months(offset));
    months
        .filter(|month| *month <= YearMonth::of(today))
        .map(|month| month.clamped(day))
        .filter(|date| *date >= from && *date <= today)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn day(value: u8) -> DayOfMonth {
        DayOfMonth::new(value).unwrap()
    }

    #[test]
    fn nothing_due_before_the_day() {
        assert!(due_dates(day(10), date(3, 1), None, date(3, 9)).is_empty());
        assert_eq!(due_dates(day(10), date(3, 1), None, date(3, 10)), vec![date(3, 10)]);
    }

    #[test]
    fn already_generated_is_not_due_again() {
        assert!(due_dates(day(10), date(1, 1), Some(date(3, 10)), date(3, 31)).is_empty());
    }

    #[test]
    fn short_months_clamp_to_last_day() {
        assert_eq!(
            due_dates(day(31), date(1, 1), Some(date(1, 31)), date(2, 28)),
            vec![date(2, 28)]
        );
    }

    #[test]
    fn start_date_is_respected() {
        assert!(due_dates(day(5), date(3, 6), None, date(3, 31)).is_empty());
        assert_eq!(due_dates(day(5), date(3, 5), None, date(3, 31)), vec![date(3, 5)]);
    }

    #[test]
    fn backfill_is_capped() {
        let due = due_dates(day(1), date(1, 1), None, date(12, 15));
        assert_eq!(due, vec![date(9, 1), date(10, 1), date(11, 1), date(12, 1)]);
    }
}
