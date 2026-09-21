//! The household's accounting cycle ("mês financeiro"), which starts on a
//! user-chosen day such as salary day instead of the 1st.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{DayOfMonth, YearMonth};

/// Half-open range `[start, end_exclusive)`. Consecutive cycles never
/// overlap or leave gaps, even for start days 29-31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cycle {
    pub start: NaiveDate,
    pub end_exclusive: NaiveDate,
}

impl Cycle {
    /// Cycle that contains `date`.
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use domain::{DayOfMonth, cycle::Cycle};
    /// let day5 = DayOfMonth::new(5).unwrap();
    /// let cycle = Cycle::containing(NaiveDate::from_ymd_opt(2026, 3, 4).unwrap(), day5);
    /// assert_eq!(cycle.start, NaiveDate::from_ymd_opt(2026, 2, 5).unwrap());
    /// ```
    pub fn containing(date: NaiveDate, start_day: DayOfMonth) -> Self {
        let month = YearMonth::of(date);
        let starts_this_month = date >= month.clamped(start_day);
        let start_month = if starts_this_month {
            month
        } else {
            month.prev()
        };
        Self::starting_in(start_month, start_day)
    }

    /// Cycle whose first day falls in `month`.
    pub fn starting_in(month: YearMonth, start_day: DayOfMonth) -> Self {
        Self {
            start: month.clamped(start_day),
            end_exclusive: month.next().clamped(start_day),
        }
    }

    pub fn previous(self, start_day: DayOfMonth) -> Self {
        Self::starting_in(YearMonth::of(self.start).prev(), start_day)
    }

    pub fn contains(self, date: NaiveDate) -> bool {
        self.start <= date && date < self.end_exclusive
    }

    /// Last day inside the cycle (inclusive).
    pub fn last_day(self) -> NaiveDate {
        self.end_exclusive.pred_opt().unwrap_or(self.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn day(value: u8) -> DayOfMonth {
        DayOfMonth::new(value).unwrap()
    }

    type Ymd = (i32, u32, u32);

    /// (date, start day, expected start, expected `end_exclusive`)
    const CASES: &[(Ymd, u8, Ymd, Ymd)] = &[
        ((2026, 3, 4), 5, (2026, 2, 5), (2026, 3, 5)),
        ((2026, 3, 5), 5, (2026, 3, 5), (2026, 4, 5)),
        ((2026, 1, 1), 1, (2026, 1, 1), (2026, 2, 1)),
        ((2026, 1, 3), 5, (2025, 12, 5), (2026, 1, 5)),
        ((2026, 2, 27), 31, (2026, 1, 31), (2026, 2, 28)),
        ((2026, 2, 28), 31, (2026, 2, 28), (2026, 3, 31)),
        ((2026, 3, 30), 31, (2026, 2, 28), (2026, 3, 31)),
        ((2024, 2, 29), 30, (2024, 2, 29), (2024, 3, 30)),
    ];

    #[test]
    fn containing_matches_table() {
        for &(on, start_day, (sy, sm, sd), (ey, em, ed)) in CASES {
            let cycle = Cycle::containing(date(on.0, on.1, on.2), day(start_day));
            assert_eq!(cycle.start, date(sy, sm, sd), "{on:?} day {start_day}");
            assert_eq!(
                cycle.end_exclusive,
                date(ey, em, ed),
                "{on:?} day {start_day}"
            );
        }
    }

    #[test]
    fn previous_and_last_day() {
        let cycle = Cycle::containing(date(2026, 3, 10), day(5));
        assert_eq!(cycle.previous(day(5)).start, date(2026, 2, 5));
        assert_eq!(cycle.last_day(), date(2026, 4, 4));
        assert!(cycle.contains(date(2026, 4, 4)));
        assert!(!cycle.contains(date(2026, 4, 5)));
    }

    proptest! {
        #[test]
        fn cycles_tile_the_calendar(offset in 0i64..3650, start in 1u8..=31) {
            let on = date(2020, 1, 1) + chrono::Days::new(offset.unsigned_abs());
            let cycle = Cycle::containing(on, day(start));
            prop_assert!(cycle.contains(on));
            let next = Cycle::containing(cycle.end_exclusive, day(start));
            prop_assert_eq!(next.start, cycle.end_exclusive);
            prop_assert_eq!(next.previous(day(start)), cycle);
        }
    }
}
