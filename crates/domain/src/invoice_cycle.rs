//! Which credit card invoice ("fatura") a purchase lands on.
//!
//! Brazilian banks close the invoice on `closing_day` and charge on
//! `due_day`. Most treat the closing day itself as the "melhor dia de
//! compra" (purchase goes to the next invoice), but not all, so it is a
//! per-card flag.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{DayOfMonth, YearMonth};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSchedule {
    pub closing_day: DayOfMonth,
    pub due_day: DayOfMonth,
    /// Purchases made on the closing date go to the next invoice.
    pub closing_day_goes_next: bool,
}

/// Dates of one invoice. `reference_month` is the month of the due date,
/// which is how banks label invoices ("fatura de março").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvoicePeriod {
    pub reference_month: YearMonth,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
}

impl CardSchedule {
    /// Invoice receiving a purchase made on `purchase_date`.
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use domain::{DayOfMonth, invoice_cycle::CardSchedule};
    /// let card = CardSchedule {
    ///     closing_day: DayOfMonth::new(3).unwrap(),
    ///     due_day: DayOfMonth::new(10).unwrap(),
    ///     closing_day_goes_next: true,
    /// };
    /// let invoice = card.period_for_purchase(NaiveDate::from_ymd_opt(2026, 3, 3).unwrap());
    /// assert_eq!(invoice.due_date, NaiveDate::from_ymd_opt(2026, 4, 10).unwrap());
    /// ```
    pub fn period_for_purchase(self, purchase_date: NaiveDate) -> InvoicePeriod {
        self.period_closing_in(self.closing_month_for(purchase_date))
    }

    /// Invoice `offset` months after `period`; installment k uses k-1.
    pub fn period_after(self, period: InvoicePeriod, offset: u32) -> InvoicePeriod {
        let shift = i32::try_from(offset).unwrap_or(i32::MAX / 24);
        self.period_closing_in(YearMonth::of(period.closing_date).plus_months(shift))
    }

    /// Invoice whose closing date falls in `closing_month`.
    pub fn period_closing_in(self, closing_month: YearMonth) -> InvoicePeriod {
        let closing_date = closing_month.clamped(self.closing_day);
        let due_date = self.first_due_after(closing_date);
        InvoicePeriod { reference_month: YearMonth::of(due_date), closing_date, due_date }
    }

    /// Closed invoices accept no new purchases and can be paid.
    pub fn is_closed(self, period: InvoicePeriod, today: NaiveDate) -> bool {
        if self.closing_day_goes_next {
            return today >= period.closing_date;
        }
        today > period.closing_date
    }

    fn closing_month_for(self, purchase_date: NaiveDate) -> YearMonth {
        let month = YearMonth::of(purchase_date);
        let closing = month.clamped(self.closing_day);
        let on_closing_stays = purchase_date == closing && !self.closing_day_goes_next;
        let stays_this_month = purchase_date < closing || on_closing_stays;
        if stays_this_month { month } else { month.next() }
    }

    /// First due day strictly after closing, so a due day smaller than the
    /// closing day falls in the next month.
    fn first_due_after(self, closing_date: NaiveDate) -> NaiveDate {
        let month = YearMonth::of(closing_date);
        let same_month = month.clamped(self.due_day);
        if same_month > closing_date { same_month } else { month.next().clamped(self.due_day) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn card(closing: u8, due: u8, goes_next: bool) -> CardSchedule {
        CardSchedule {
            closing_day: DayOfMonth::new(closing).unwrap(),
            due_day: DayOfMonth::new(due).unwrap(),
            closing_day_goes_next: goes_next,
        }
    }

    type Ymd = (i32, u32, u32);

    /// (closing, due, `goes_next`, purchase, expected closing, expected due)
    const CASES: &[(u8, u8, bool, Ymd, Ymd, Ymd)] = &[
        (3, 10, true, (2026, 3, 2), (2026, 3, 3), (2026, 3, 10)),
        (3, 10, true, (2026, 3, 3), (2026, 4, 3), (2026, 4, 10)),
        (3, 10, false, (2026, 3, 3), (2026, 3, 3), (2026, 3, 10)),
        (3, 10, false, (2026, 3, 4), (2026, 4, 3), (2026, 4, 10)),
        (25, 5, true, (2026, 3, 20), (2026, 3, 25), (2026, 4, 5)),
        (25, 5, true, (2026, 12, 26), (2027, 1, 25), (2027, 2, 5)),
        (31, 8, true, (2026, 2, 27), (2026, 2, 28), (2026, 3, 8)),
        (31, 8, true, (2026, 2, 28), (2026, 3, 31), (2026, 4, 8)),
        (10, 10, true, (2026, 5, 1), (2026, 5, 10), (2026, 6, 10)),
        (28, 30, true, (2026, 1, 29), (2026, 2, 28), (2026, 3, 30)),
    ];

    #[test]
    fn purchase_lands_on_expected_invoice() {
        for &(closing, due, next, purchase, want_close, want_due) in CASES {
            let got = card(closing, due, next)
                .period_for_purchase(date(purchase.0, purchase.1, purchase.2));
            let label = format!("closing {closing} due {due} next {next} purchase {purchase:?}");
            assert_eq!(got.closing_date, date(want_close.0, want_close.1, want_close.2), "{label}");
            assert_eq!(got.due_date, date(want_due.0, want_due.1, want_due.2), "{label}");
            assert_eq!(got.reference_month, YearMonth::of(got.due_date), "{label}");
        }
    }

    #[test]
    fn period_after_keeps_clamping_from_closing_day() {
        let schedule = card(31, 8, true);
        let first = schedule.period_for_purchase(date(2026, 1, 10));
        assert_eq!(first.closing_date, date(2026, 1, 31));
        assert_eq!(schedule.period_after(first, 1).closing_date, date(2026, 2, 28));
        assert_eq!(schedule.period_after(first, 2).closing_date, date(2026, 3, 31));
        assert_eq!(schedule.period_after(first, 0), first);
    }

    #[test]
    fn closed_depends_on_closing_day_rule() {
        let period = card(3, 10, true).period_for_purchase(date(2026, 3, 1));
        assert!(!card(3, 10, true).is_closed(period, date(2026, 3, 2)));
        assert!(card(3, 10, true).is_closed(period, date(2026, 3, 3)));
        assert!(!card(3, 10, false).is_closed(period, date(2026, 3, 3)));
        assert!(card(3, 10, false).is_closed(period, date(2026, 3, 4)));
    }
}
