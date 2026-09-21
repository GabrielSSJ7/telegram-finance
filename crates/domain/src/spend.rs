//! Income, spending and savings for a period, all by `accounting_date`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Cents, EntryKind};

/// Sum of non-deleted entries of one kind and category inside a period.
/// `category` is generic so the domain does not depend on id types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryFlow<C> {
    pub category: Option<C>,
    pub kind: EntryKind,
    pub total: Cents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodSummary {
    pub income: Cents,
    pub spending: Cents,
    /// Income minus spending; negative when the couple spent more.
    pub saved: Cents,
    /// `saved / income` in basis points (1% = 100); `None` without income.
    pub savings_rate_bp: Option<i64>,
}

/// Totals for a period.
///
/// ```
/// use domain::{Cents, EntryKind, spend::{CategoryFlow, summarize_period}};
/// let flows: [CategoryFlow<u8>; 2] = [
///     CategoryFlow { category: None, kind: EntryKind::Income, total: Cents::new(1000) },
///     CategoryFlow { category: Some(1), kind: EntryKind::Expense, total: Cents::new(250) },
/// ];
/// assert_eq!(summarize_period(&flows).savings_rate_bp, Some(7500));
/// ```
pub fn summarize_period<C>(flows: &[CategoryFlow<C>]) -> PeriodSummary {
    let income: Cents = flows.iter().map(|flow| flow.total.times(flow.kind.income_effect())).sum();
    let spending: Cents = flows.iter().map(|flow| flow.total.times(flow.kind.spend_effect())).sum();
    let saved = income - spending;
    let savings_rate_bp = income.is_positive().then(|| saved.value() * 10_000 / income.value());
    PeriodSummary { income, spending, saved, savings_rate_bp }
}

/// Net spending per category, largest first. Categories whose refunds
/// cancel all spending are dropped.
pub fn spending_by_category<C: Ord + Clone>(flows: &[CategoryFlow<C>]) -> Vec<(Option<C>, Cents)> {
    let mut totals: BTreeMap<Option<C>, Cents> = BTreeMap::new();
    for flow in flows.iter().filter(|flow| flow.kind.spend_effect() != 0) {
        *totals.entry(flow.category.clone()).or_default() +=
            flow.total.times(flow.kind.spend_effect());
    }
    let mut ranked: Vec<(Option<C>, Cents)> =
        totals.into_iter().filter(|(_, total)| total.is_positive()).collect();
    ranked.sort_by_key(|(_, total)| std::cmp::Reverse(*total));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(
        category: Option<&'static str>,
        kind: EntryKind,
        total: i64,
    ) -> CategoryFlow<&'static str> {
        CategoryFlow { category, kind, total: Cents::new(total) }
    }

    fn sample() -> Vec<CategoryFlow<&'static str>> {
        vec![
            flow(Some("salario"), EntryKind::Income, 1_000_000),
            flow(Some("mercado"), EntryKind::Expense, 80_000),
            flow(Some("mercado"), EntryKind::CardInstallment, 20_000),
            flow(Some("mercado"), EntryKind::CardCredit, 5_000),
            flow(Some("lazer"), EntryKind::CardInstallment, 150_000),
            flow(Some("saude"), EntryKind::Expense, 3_000),
            flow(Some("saude"), EntryKind::Refund, 3_000),
            flow(None, EntryKind::Transfer, 400_000),
            flow(None, EntryKind::InvoicePayment, 170_000),
        ]
    }

    #[test]
    fn refunds_lower_spending_and_never_count_as_income() {
        let summary = summarize_period(&sample());
        assert_eq!(summary.income, Cents::new(1_000_000));
        assert_eq!(summary.spending, Cents::new(245_000));
        assert_eq!(summary.saved, Cents::new(755_000));
        assert_eq!(summary.savings_rate_bp, Some(7_550));
    }

    #[test]
    fn savings_rate_is_none_without_income() {
        let summary = summarize_period(&[flow(Some("lazer"), EntryKind::Expense, 100)]);
        assert_eq!(summary.savings_rate_bp, None);
        assert_eq!(summary.saved, Cents::new(-100));
    }

    #[test]
    fn ranks_categories_by_net_spending() {
        let ranked = spending_by_category(&sample());
        assert_eq!(
            ranked,
            vec![(Some("lazer"), Cents::new(150_000)), (Some("mercado"), Cents::new(95_000))]
        );
    }
}
