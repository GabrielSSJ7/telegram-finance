//! The basic cost of living: spending in the categories marked essential.

use domain::Cents;
use domain::cycle::Cycle;
use serde::Serialize;

use super::Category;

/// How many months of the basic cost an emergency reserve should cover.
pub const RESERVE_MONTHS: i64 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LivingCost {
    pub cycle: Cycle,
    /// Essential spending dated up to today.
    pub spent: Cents,
    /// Essential card installments dated later in the cycle plus essential
    /// bills from recurrences not recorded yet.
    pub still_coming: Cents,
    /// Essential spending per category in the cycle, recorded and coming,
    /// largest first.
    pub by_category: Vec<(Category, Cents)>,
    /// Average of up to three earlier cycles tracked from their first day.
    pub recent_average: Option<Cents>,
    pub averaged_cycles: usize,
    /// Income recorded in the cycle plus income recurrences still due.
    pub expected_income: Cents,
    /// Money in goal pots, the natural emergency reserve.
    pub reserved: Cents,
}

impl LivingCost {
    /// The whole cycle: what was spent plus what is already known to come.
    pub fn projected(&self) -> Cents {
        self.spent + self.still_coming
    }

    /// The monthly figure to plan with: the recent average once there is
    /// one, since a single cycle swings with occasional bills.
    pub fn monthly_cost(&self) -> Cents {
        self.recent_average.unwrap_or_else(|| self.projected())
    }

    /// Projected essential spending over expected income, in basis points.
    pub fn income_share_bp(&self) -> Option<i64> {
        let income = self.expected_income.value();
        (income > 0).then(|| self.projected().value() * 10_000 / income)
    }

    pub fn reserve_target(&self) -> Cents {
        self.monthly_cost().times(RESERVE_MONTHS)
    }

    /// Months the pots would cover, in tenths: 35 is 3,5 months.
    pub fn reserve_tenths_of_month(&self) -> Option<i64> {
        let cost = self.monthly_cost().value();
        (cost > 0).then(|| self.reserved.value() * 10 / cost)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use domain::DayOfMonth;

    use super::*;

    fn cost(
        spent: i64,
        coming: i64,
        average: Option<i64>,
        income: i64,
        reserved: i64,
    ) -> LivingCost {
        let start = NaiveDate::from_ymd_opt(2026, 9, 5).unwrap();
        LivingCost {
            cycle: Cycle::containing(start, DayOfMonth::new(5).unwrap()),
            spent: Cents::new(spent),
            still_coming: Cents::new(coming),
            by_category: Vec::new(),
            recent_average: average.map(Cents::new),
            averaged_cycles: usize::from(average.is_some()),
            expected_income: Cents::new(income),
            reserved: Cents::new(reserved),
        }
    }

    #[test]
    fn projects_and_prefers_the_recent_average() {
        let fresh = cost(300_000, 100_000, None, 1_000_000, 2_000_000);
        assert_eq!(fresh.projected(), Cents::new(400_000));
        assert_eq!(fresh.monthly_cost(), Cents::new(400_000));
        assert_eq!(fresh.income_share_bp(), Some(4_000));
        assert_eq!(fresh.reserve_target(), Cents::new(2_400_000));
        assert_eq!(fresh.reserve_tenths_of_month(), Some(50));
        let settled = cost(300_000, 0, Some(500_000), 0, 2_000_000);
        assert_eq!(
            (settled.monthly_cost(), settled.income_share_bp()),
            (Cents::new(500_000), None)
        );
        assert_eq!(cost(0, 0, None, 0, 1).reserve_tenths_of_month(), None);
    }
}
