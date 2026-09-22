//! `/projecao`: how the cycle should end, joining what is recorded with
//! what is already known to come.

use domain::Cents;
use domain::cycle::Cycle;
use serde::Serialize;

use super::Category;

/// Money of one direction: what happened and what is still expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Flow {
    pub recorded: Cents,
    /// Card installments dated later in the cycle and recurring entries
    /// not recorded yet.
    pub coming: Cents,
}

impl Flow {
    pub fn total(self) -> Cents {
        self.recorded + self.coming
    }
}

/// What the couple should have at the end of the cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CycleProjection {
    pub cycle: Cycle,
    /// Days left, today included.
    pub days_left: i64,
    pub income: Flow,
    pub spending: Flow,
    /// Biggest projected spending per category, largest first.
    pub by_category: Vec<(Category, Cents)>,
    /// "Disponível" right now.
    pub available: Cents,
    /// Invoices and bills to pay out of the accounts before the cycle ends.
    pub due_from_accounts: Cents,
    /// Bills waiting for a [Registrar] tap; they are not counted anywhere.
    pub bills_to_confirm: usize,
}

impl CycleProjection {
    /// Income minus spending over the whole cycle.
    pub fn result(&self) -> Cents {
        self.income.total() - self.spending.total()
    }

    /// Result over income, in basis points; `None` without income.
    pub fn saved_bp(&self) -> Option<i64> {
        let income = self.income.total().value();
        (income > 0).then(|| self.result().value() * 10_000 / income)
    }

    /// What should be left in the accounts at the end of the cycle.
    pub fn cash_at_end(&self) -> Cents {
        self.available + self.income.coming - self.due_from_accounts
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use domain::DayOfMonth;

    use super::*;

    fn projection(income: (i64, i64), spending: (i64, i64)) -> CycleProjection {
        let start = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        CycleProjection {
            cycle: Cycle::containing(start, DayOfMonth::new(5).unwrap()),
            days_left: 13,
            income: Flow { recorded: Cents::new(income.0), coming: Cents::new(income.1) },
            spending: Flow { recorded: Cents::new(spending.0), coming: Cents::new(spending.1) },
            by_category: Vec::new(),
            available: Cents::new(540_384),
            due_from_accounts: Cents::new(313_890),
            bills_to_confirm: 0,
        }
    }

    #[test]
    fn result_and_cash_at_the_end() {
        let cycle = projection((800_000, 400_000), (407_923, 313_890));
        assert_eq!(cycle.income.total(), Cents::new(1_200_000));
        assert_eq!(cycle.result(), Cents::new(478_187));
        assert_eq!(cycle.saved_bp(), Some(3_984));
        assert_eq!(cycle.cash_at_end(), Cents::new(626_494));
        assert_eq!(projection((0, 0), (100, 0)).saved_bp(), None);
    }
}
