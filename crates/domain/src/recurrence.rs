//! When a monthly recurring entry (salary, rent, subscriptions) is due.

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

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

/// The first date on `day` (clamped) that is not before `starts_on`.
///
/// ```
/// use chrono::NaiveDate;
/// use domain::{DayOfMonth, recurrence::first_due_date};
/// let date = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
/// assert_eq!(first_due_date(DayOfMonth::new(10).unwrap(), date(9, 22)), date(10, 10));
/// ```
pub fn first_due_date(day: DayOfMonth, starts_on: NaiveDate) -> NaiveDate {
    let this_month = YearMonth::of(starts_on).clamped(day);
    if this_month >= starts_on {
        return this_month;
    }
    YearMonth::of(starts_on).next().clamped(day)
}

/// A recurrence that ends: a financing or loan paid monthly. The first due
/// date pays installment `first_number` (above 1 for a plan already under
/// way) and the last one pays installment `count`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallmentPlan {
    pub first_number: u32,
    pub count: u32,
}

impl InstallmentPlan {
    /// The installment paid on `date`, counting from the first due date.
    #[allow(clippy::cast_sign_loss)]
    pub fn number_on(self, first_due: NaiveDate, date: NaiveDate) -> u32 {
        let months = YearMonth::of(first_due).months_until(YearMonth::of(date)).max(0);
        self.first_number + months as u32
    }

    /// The date of the last installment.
    #[allow(clippy::cast_possible_wrap)]
    pub fn last_due(self, day: DayOfMonth, first_due: NaiveDate) -> NaiveDate {
        let remaining = self.count.saturating_sub(self.first_number) as i32;
        YearMonth::of(first_due).plus_months(remaining).clamped(day)
    }
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

    #[test]
    fn installment_plan_numbers_and_end() {
        let day = DayOfMonth::new(31).unwrap();
        let first_due = first_due_date(day, date(1, 15));
        assert_eq!(first_due, date(1, 31));
        let plan = InstallmentPlan { first_number: 23, count: 36 };
        assert_eq!(plan.number_on(first_due, date(1, 31)), 23);
        assert_eq!(plan.number_on(first_due, date(2, 28)), 24);
        assert_eq!(
            plan.number_on(first_due, date(1, 1)),
            23,
            "before the start counts as the first"
        );
        assert_eq!(plan.last_due(day, first_due), NaiveDate::from_ymd_opt(2027, 2, 28).unwrap());
        assert_eq!(first_due_date(DayOfMonth::new(5).unwrap(), date(3, 5)), date(3, 5));
    }
}
