//! Splits a card purchase into installments ("parcelas") and places each one
//! on its invoice. Everything is computed from the purchase date, never
//! chained from the previous installment, so month-end dates do not drift.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Cents;
use crate::calendar::add_months_clamped;
use crate::invoice_cycle::{CardSchedule, InvoicePeriod};

pub const MAX_INSTALLMENTS: u32 = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InstallmentError {
    #[error("invalid installment count {0}: expected 1 to {MAX_INSTALLMENTS}")]
    Count(u32),
    #[error("invalid first installment {first} of {count}: expected 1 to {count}")]
    FirstNumber { first: u32, count: u32 },
    #[error(
        "invalid purchase total {total} cents for {count} installments: expected at least 1 cent per installment"
    )]
    TotalTooSmall { total: i64, count: u32 },
}

/// A purchase to schedule. `first_number > 1` imports a purchase already in
/// progress ("parcelado em andamento"): earlier installments are skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallmentPlan {
    pub total: Cents,
    pub count: u32,
    pub first_number: u32,
    pub purchase_date: NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallmentSlot {
    pub number: u32,
    pub amount: Cents,
    pub accounting_date: NaiveDate,
    pub invoice: InvoicePeriod,
}

/// Splits `total` in `count` parts; leftover cents go on the first part.
///
/// ```
/// use domain::{Cents, installments::split_amount};
/// let parts = split_amount(Cents::new(10_000), 3).unwrap();
/// assert_eq!(parts, vec![Cents::new(3334), Cents::new(3333), Cents::new(3333)]);
/// ```
pub fn split_amount(total: Cents, count: u32) -> Result<Vec<Cents>, InstallmentError> {
    validate_count(count)?;
    let parts = i64::from(count);
    if total.value() < parts {
        return Err(InstallmentError::TotalTooSmall { total: total.value(), count });
    }
    let base = total.value() / parts;
    let first = base + total.value() % parts;
    let rest = std::iter::repeat_n(Cents::new(base), usize::try_from(count - 1).unwrap_or(0));
    Ok(std::iter::once(Cents::new(first)).chain(rest).collect())
}

/// Places installments `first_number..=count` on their invoices.
pub fn schedule_installments(
    plan: InstallmentPlan,
    card: CardSchedule,
) -> Result<Vec<InstallmentSlot>, InstallmentError> {
    let amounts = split_amount(plan.total, plan.count)?;
    if !(1..=plan.count).contains(&plan.first_number) {
        return Err(InstallmentError::FirstNumber { first: plan.first_number, count: plan.count });
    }
    let first_invoice = card.period_for_purchase(plan.purchase_date);
    let slots = (plan.first_number..=plan.count).zip(amounts.into_iter().skip(skip_count(plan)));
    Ok(slots.map(|(number, amount)| slot(plan, card, first_invoice, number, amount)).collect())
}

fn slot(
    plan: InstallmentPlan,
    card: CardSchedule,
    first_invoice: InvoicePeriod,
    number: u32,
    amount: Cents,
) -> InstallmentSlot {
    InstallmentSlot {
        number,
        amount,
        accounting_date: add_months_clamped(plan.purchase_date, number - 1),
        invoice: card.period_after(first_invoice, number - 1),
    }
}

fn skip_count(plan: InstallmentPlan) -> usize {
    usize::try_from(plan.first_number - 1).unwrap_or(0)
}

fn validate_count(count: u32) -> Result<(), InstallmentError> {
    if (1..=MAX_INSTALLMENTS).contains(&count) {
        return Ok(());
    }
    Err(InstallmentError::Count(count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DayOfMonth, YearMonth};
    use proptest::prelude::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn card() -> CardSchedule {
        CardSchedule {
            closing_day: DayOfMonth::new(3).unwrap(),
            due_day: DayOfMonth::new(10).unwrap(),
            closing_day_goes_next: true,
        }
    }

    fn plan(total: i64, count: u32, first_number: u32, purchase: NaiveDate) -> InstallmentPlan {
        InstallmentPlan { total: Cents::new(total), count, first_number, purchase_date: purchase }
    }

    #[test]
    fn three_installments_on_consecutive_invoices() {
        let slots = schedule_installments(plan(30_000, 3, 1, date(2026, 1, 31)), card()).unwrap();
        let refs: Vec<YearMonth> = slots.iter().map(|slot| slot.invoice.reference_month).collect();
        let expected: Vec<YearMonth> = [(2026, 2), (2026, 3), (2026, 4)]
            .map(|(year, month)| YearMonth::new(year, month).unwrap())
            .into();
        assert_eq!(refs, expected);
        let dates: Vec<NaiveDate> = slots.iter().map(|slot| slot.accounting_date).collect();
        assert_eq!(dates, vec![date(2026, 1, 31), date(2026, 2, 28), date(2026, 3, 31)]);
        assert!(slots.iter().all(|slot| slot.amount == Cents::new(10_000)));
    }

    #[test]
    fn importing_in_progress_skips_paid_installments() {
        let slots = schedule_installments(plan(1000, 10, 8, date(2026, 1, 20)), card()).unwrap();
        let numbers: Vec<u32> = slots.iter().map(|slot| slot.number).collect();
        assert_eq!(numbers, vec![8, 9, 10]);
        assert_eq!(slots[0].accounting_date, date(2026, 8, 20));
        assert_eq!(slots[0].amount, Cents::new(100));
    }

    #[test]
    fn rejects_invalid_plans() {
        let on = date(2026, 1, 1);
        assert_eq!(split_amount(Cents::new(100), 0), Err(InstallmentError::Count(0)));
        assert_eq!(split_amount(Cents::new(100), 49), Err(InstallmentError::Count(49)));
        assert_eq!(
            split_amount(Cents::new(2), 3),
            Err(InstallmentError::TotalTooSmall { total: 2, count: 3 })
        );
        assert_eq!(
            schedule_installments(plan(100, 3, 4, on), card()),
            Err(InstallmentError::FirstNumber { first: 4, count: 3 })
        );
    }

    proptest! {
        #[test]
        fn parts_sum_to_total(total in 48i64..100_000_000, count in 1u32..=48) {
            let parts = split_amount(Cents::new(total), count).unwrap();
            prop_assert_eq!(parts.len(), count as usize);
            prop_assert_eq!(parts.iter().sum::<Cents>(), Cents::new(total));
            prop_assert!(parts.iter().all(|part| part.is_positive()));
        }
    }
}
