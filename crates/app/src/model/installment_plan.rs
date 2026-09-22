//! `/parcelas`: purchases and financings paid in monthly installments.

use chrono::NaiveDate;
use domain::Cents;
use serde::Serialize;

use super::CategoryId;

/// What pays the installments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "name", rename_all = "snake_case")]
pub enum PlanSource {
    Card(String),
    Account(String),
}

/// One plan still running, with how much of it is behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallmentProgress {
    pub description: String,
    pub source: PlanSource,
    pub category_id: CategoryId,
    pub count: u32,
    /// Installments dated up to today, including those paid before the
    /// plan was entered in finbot.
    pub paid_count: u32,
    pub total: Cents,
    pub paid: Cents,
    /// The usual monthly installment.
    pub installment: Cents,
    pub last_due: NaiveDate,
}

impl InstallmentProgress {
    pub fn remaining(&self) -> Cents {
        self.total - self.paid
    }

    /// Share already paid, in basis points (2500 = 25%).
    pub fn paid_bp(&self) -> i64 {
        let total = self.total.value();
        if total <= 0 {
            return 0;
        }
        self.paid.value() * 10_000 / total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_and_share() {
        let plan = InstallmentProgress {
            description: "Enoxaparina".into(),
            source: PlanSource::Card("Itau Black".into()),
            category_id: CategoryId::generate(),
            count: 4,
            paid_count: 1,
            total: Cents::new(410_700),
            paid: Cents::new(102_675),
            installment: Cents::new(102_675),
            last_due: NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
        };
        assert_eq!((plan.remaining(), plan.paid_bp()), (Cents::new(308_025), 2_500));
        let empty = InstallmentProgress { total: Cents::ZERO, ..plan };
        assert_eq!(empty.paid_bp(), 0);
    }
}
