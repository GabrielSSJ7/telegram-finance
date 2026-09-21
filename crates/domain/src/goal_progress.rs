//! Progress of a savings goal ("meta").

use chrono::{Datelike, NaiveDate};

use crate::Cents;

/// `saved / target` in basis points (100% = 10000), clamped to 0..=10000.
///
/// ```
/// use domain::{Cents, goal_progress::progress_bp};
/// assert_eq!(progress_bp(Cents::new(2_000_000), Cents::new(10_000_000)), 2000);
/// ```
pub fn progress_bp(saved: Cents, target: Cents) -> i64 {
    if !target.is_positive() {
        return 0;
    }
    (saved.value().max(0).saturating_mul(10_000) / target.value()).min(10_000)
}

/// What is still missing to reach the target, never negative.
pub fn remaining(saved: Cents, target: Cents) -> Cents {
    Cents::new((target.value() - saved.value()).max(0))
}

/// What to save each month to reach the target by `target_date`,
/// counting the current month. `None` once the date has passed or when
/// nothing is left to save.
///
/// ```
/// use chrono::NaiveDate;
/// use domain::{Cents, goal_progress::monthly_needed};
/// let date = |month| NaiveDate::from_ymd_opt(2026, month, 1).unwrap();
/// assert_eq!(monthly_needed(Cents::new(1200), date(1), date(12)), Some(Cents::new(100)));
/// ```
pub fn monthly_needed(remaining: Cents, today: NaiveDate, target_date: NaiveDate) -> Option<Cents> {
    if !remaining.is_positive() || target_date < today {
        return None;
    }
    let months = (target_date.year() - today.year()) * 12
        + (i32::try_from(target_date.month()).unwrap_or(0)
            - i32::try_from(today.month()).unwrap_or(0))
        + 1;
    let months = i64::from(months.max(1));
    Some(Cents::new((remaining.value() + months - 1) / months))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped() {
        assert_eq!(progress_bp(Cents::new(-5), Cents::new(100)), 0);
        assert_eq!(progress_bp(Cents::new(50), Cents::new(100)), 5000);
        assert_eq!(progress_bp(Cents::new(500), Cents::new(100)), 10_000);
        assert_eq!(progress_bp(Cents::new(500), Cents::ZERO), 0);
    }

    #[test]
    fn monthly_needed_rounds_up_and_stops_after_deadline() {
        let date = |year, month| NaiveDate::from_ymd_opt(year, month, 15).unwrap();
        assert_eq!(
            monthly_needed(Cents::new(1000), date(2026, 3), date(2026, 5)),
            Some(Cents::new(334))
        );
        assert_eq!(
            monthly_needed(Cents::new(1000), date(2026, 3), date(2026, 3)),
            Some(Cents::new(1000))
        );
        assert_eq!(monthly_needed(Cents::new(1000), date(2026, 3), date(2026, 2)), None);
        assert_eq!(monthly_needed(Cents::ZERO, date(2026, 3), date(2027, 3)), None);
        assert_eq!(
            monthly_needed(Cents::new(1300), date(2026, 12), date(2027, 12)),
            Some(Cents::new(100))
        );
    }

    #[test]
    fn remaining_never_negative() {
        assert_eq!(remaining(Cents::new(30), Cents::new(100)), Cents::new(70));
        assert_eq!(remaining(Cents::new(300), Cents::new(100)), Cents::ZERO);
    }
}
