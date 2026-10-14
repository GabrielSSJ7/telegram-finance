//! What is still owed on card invoices, split around the end of the running
//! cycle.
//!
//! The daily report counts a card purchase as spending on the day it was
//! made, but the money only leaves on the invoice's due date — often in the
//! next cycle. This module is what lets the report say so.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::Cents;

/// Invoice money owed, split by whether it falls due inside the cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InvoicesDue {
    /// Owed on invoices due up to and including the cycle's last day.
    pub in_cycle: Cents,
    /// Owed afterwards: invoices due later plus installments already
    /// committed to invoices that have not been opened yet.
    pub later: Cents,
}

impl InvoicesDue {
    pub fn total(self) -> Cents {
        self.in_cycle + self.later
    }

    pub fn is_zero(self) -> bool {
        self.total() == Cents::ZERO
    }
}

/// Splits `(due date, outstanding)` pairs at `last_day`; `committed_later`
/// is what is already charged to invoices that do not exist yet.
///
/// ```
/// use chrono::NaiveDate;
/// use domain::{Cents, due::invoices_due};
/// let day = |d| NaiveDate::from_ymd_opt(2026, 11, d).unwrap();
/// let invoices = [(day(5), Cents::new(7_100)), (day(11), Cents::new(1_184_109))];
/// let due = invoices_due(invoices, day(4), Cents::ZERO);
/// assert_eq!(due.in_cycle, Cents::ZERO);
/// assert_eq!(due.later, Cents::new(1_191_209));
/// ```
pub fn invoices_due<I>(invoices: I, last_day: NaiveDate, committed_later: Cents) -> InvoicesDue
where
    I: IntoIterator<Item = (NaiveDate, Cents)>,
{
    let mut due = InvoicesDue { in_cycle: Cents::ZERO, later: committed_later };
    for (due_date, outstanding) in invoices {
        if due_date <= last_day {
            due.in_cycle += outstanding;
        } else {
            due.later += outstanding;
        }
    }
    due
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn splits_on_the_last_day_of_the_cycle() {
        let invoices = [(day(4), Cents::new(500)), (day(5), Cents::new(700))];
        let due = invoices_due(invoices, day(4), Cents::new(100));
        assert_eq!(due.in_cycle, Cents::new(500));
        assert_eq!(due.later, Cents::new(800));
        assert_eq!(due.total(), Cents::new(1300));
    }

    #[test]
    fn a_credit_on_an_invoice_lowers_what_is_owed() {
        let invoices = [(day(4), Cents::new(500)), (day(4), Cents::new(-200))];
        let due = invoices_due(invoices, day(4), Cents::ZERO);
        assert_eq!(due.in_cycle, Cents::new(300));
    }

    #[test]
    fn nothing_owed_is_zero() {
        let due = invoices_due([], day(4), Cents::ZERO);
        assert!(due.is_zero());
        assert_eq!(due, InvoicesDue::default());
    }
}
