//! What is owed on each card invoice. Status is derived, never stored:
//! "closed" comes from the date, "paid" from payments covering the bill.
//!
//! Brazilian banks roll any unpaid rest of a closed invoice into the next
//! one ("saldo anterior"), and an overpayment into a credit. So each
//! invoice starts from what the previous closed invoice left over.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::Cents;
use crate::invoice_cycle::{CardSchedule, InvoicePeriod};

/// Sums of the ledger rows attached to one invoice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTotals {
    /// Installments charged on this invoice.
    pub charges: Cents,
    /// Refunds ("estornos") credited to this invoice.
    pub credits: Cents,
    pub payments: Cents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    /// Still collecting purchases.
    Open,
    /// Closed with money still owed.
    Closed,
    /// Closed and covered (or in credit).
    Paid,
    /// Closed, not fully paid, and the rest already moved into the next
    /// closed invoice.
    Rolled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceStatement {
    pub period: InvoicePeriod,
    pub totals: InvoiceTotals,
    /// Left by the previous closed invoice: positive = unpaid rest,
    /// negative = credit.
    pub carried_in: Cents,
    /// Still owed on this invoice; negative is a credit.
    pub outstanding: Cents,
    pub status: InvoiceStatus,
}

/// Statements for one card, in the order given (oldest closing first).
///
/// ```
/// use chrono::NaiveDate;
/// use domain::{Cents, DayOfMonth};
/// use domain::invoice_cycle::CardSchedule;
/// use domain::invoice_settlement::{InvoiceStatus, InvoiceTotals, statements};
/// let card = CardSchedule { closing_day: DayOfMonth::new(3).unwrap(), due_day: DayOfMonth::new(10).unwrap(), closing_day_goes_next: true };
/// let period = card.period_for_purchase(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
/// let totals = InvoiceTotals { charges: Cents::new(500), ..InvoiceTotals::default() };
/// let today = NaiveDate::from_ymd_opt(2026, 3, 5).unwrap();
/// assert_eq!(statements(card, &[(period, totals)], today)[0].status, InvoiceStatus::Closed);
/// ```
pub fn statements(
    schedule: CardSchedule,
    invoices: &[(InvoicePeriod, InvoiceTotals)],
    today: NaiveDate,
) -> Vec<InvoiceStatement> {
    let mut carried = Cents::ZERO;
    let mut result: Vec<InvoiceStatement> = invoices
        .iter()
        .map(|(period, totals)| {
            let statement = statement(schedule, *period, *totals, carried, today);
            carried = if statement.status == InvoiceStatus::Open {
                Cents::ZERO
            } else {
                statement.outstanding
            };
            statement
        })
        .collect();
    mark_rolled(&mut result);
    result
}

fn statement(
    schedule: CardSchedule,
    period: InvoicePeriod,
    totals: InvoiceTotals,
    carried_in: Cents,
    today: NaiveDate,
) -> InvoiceStatement {
    let outstanding = carried_in + totals.charges - totals.credits - totals.payments;
    let status = if !schedule.is_closed(period, today) {
        InvoiceStatus::Open
    } else if outstanding.is_positive() {
        InvoiceStatus::Closed
    } else {
        InvoiceStatus::Paid
    };
    InvoiceStatement { period, totals, carried_in, outstanding, status }
}

/// An unpaid closed invoice followed by another closed one has had its
/// rest carried forward; only the newest closed invoice is still due.
fn mark_rolled(result: &mut [InvoiceStatement]) {
    let closed: Vec<usize> =
        (0..result.len()).filter(|index| result[*index].status != InvoiceStatus::Open).collect();
    for window in closed.windows(2) {
        if result[window[0]].status == InvoiceStatus::Closed {
            result[window[0]].status = InvoiceStatus::Rolled;
        }
    }
}

/// Owed on closed invoices not yet paid; counted once (the newest closed
/// invoice already includes older rests).
pub fn unpaid_closed(statements: &[InvoiceStatement]) -> Cents {
    let newest_closed =
        statements.iter().rev().find(|statement| statement.status != InvoiceStatus::Open);
    newest_closed
        .filter(|statement| statement.status == InvoiceStatus::Closed)
        .map_or(Cents::ZERO, |statement| statement.outstanding)
}

/// The invoice collecting today's purchases, if it exists yet.
pub fn current_open(statements: &[InvoiceStatement]) -> Option<&InvoiceStatement> {
    statements.iter().find(|statement| statement.status == InvoiceStatus::Open)
}

/// Installments already charged to invoices after the current one.
pub fn future_committed(statements: &[InvoiceStatement]) -> Cents {
    let open = statements.iter().filter(|statement| statement.status == InvoiceStatus::Open);
    open.skip(1).map(|statement| statement.totals.charges - statement.totals.credits).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DayOfMonth;

    fn card() -> CardSchedule {
        CardSchedule {
            closing_day: DayOfMonth::new(3).unwrap(),
            due_day: DayOfMonth::new(10).unwrap(),
            closing_day_goes_next: true,
        }
    }

    fn period(month: u32) -> InvoicePeriod {
        card().period_for_purchase(NaiveDate::from_ymd_opt(2026, month, 1).unwrap())
    }

    fn totals(charges: i64, credits: i64, payments: i64) -> InvoiceTotals {
        InvoiceTotals {
            charges: Cents::new(charges),
            credits: Cents::new(credits),
            payments: Cents::new(payments),
        }
    }

    fn on(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn statuses(result: &[InvoiceStatement]) -> Vec<InvoiceStatus> {
        result.iter().map(|statement| statement.status).collect()
    }

    #[test]
    fn partial_payment_rolls_rest_into_next_invoice() {
        let invoices = [(period(1), totals(1000, 0, 600)), (period(2), totals(500, 0, 0))];
        let result = statements(card(), &invoices, on(2, 5));
        assert_eq!(statuses(&result), vec![InvoiceStatus::Rolled, InvoiceStatus::Closed]);
        assert_eq!(
            (result[1].carried_in, result[1].outstanding),
            (Cents::new(400), Cents::new(900))
        );
        assert_eq!(unpaid_closed(&result), Cents::new(900));
    }

    #[test]
    fn overpayment_becomes_credit_and_open_invoices_do_not_carry() {
        let invoices = [
            (period(1), totals(1000, 100, 1000)),
            (period(2), totals(300, 0, 0)),
            (period(3), totals(200, 0, 0)),
        ];
        let result = statements(card(), &invoices, on(2, 1));
        assert_eq!(
            statuses(&result),
            vec![InvoiceStatus::Paid, InvoiceStatus::Open, InvoiceStatus::Open]
        );
        assert_eq!(
            (result[1].carried_in, result[1].outstanding),
            (Cents::new(-100), Cents::new(200))
        );
        assert_eq!(result[2].carried_in, Cents::ZERO);
        assert_eq!(unpaid_closed(&result), Cents::ZERO);
    }

    #[test]
    fn current_and_future_commitments() {
        let invoices = [
            (period(1), totals(1000, 0, 0)),
            (period(2), totals(300, 0, 0)),
            (period(3), totals(200, 50, 0)),
        ];
        let result = statements(card(), &invoices, on(1, 10));
        assert_eq!(current_open(&result).map(|statement| statement.period), Some(period(2)));
        assert_eq!(future_committed(&result), Cents::new(150));
        assert_eq!(unpaid_closed(&result), Cents::new(1000));
    }

    #[test]
    fn no_invoices_means_nothing_owed() {
        let result = statements(card(), &[], on(1, 1));
        assert!(result.is_empty() && current_open(&result).is_none());
        assert_eq!((unpaid_closed(&result), future_committed(&result)), (Cents::ZERO, Cents::ZERO));
    }
}
