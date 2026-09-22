//! `/custodevida`: what the household's basic life costs, from the
//! categories marked essential.

use std::sync::Arc;

use chrono::NaiveDate;
use domain::Cents;
use domain::cycle::Cycle;

use super::category_activity::{category_activity, category_effect};
use super::{
    AccountService, CategoryService, LedgerService, PositionService, RecurrenceService,
    SettingsService,
};
use crate::AppResult;
use crate::model::{
    Category, CategoryKind, EntryFilter, LedgerEntry, LivingCost, Recurrence, RecurrenceKind,
};
use crate::ports::Clock;

/// Earlier cycles averaged into the monthly cost.
pub const AVERAGED_CYCLES: usize = 3;

/// Far more entries than a couple records in a cycle.
const MAX_CYCLE_ENTRIES: u32 = 20_000;

pub struct LivingCostSources {
    pub ledger: Arc<LedgerService>,
    pub categories: Arc<CategoryService>,
    pub recurrences: Arc<RecurrenceService>,
    pub settings: Arc<SettingsService>,
    pub position: Arc<PositionService>,
    pub accounts: Arc<AccountService>,
}

pub struct LivingCostService {
    sources: LivingCostSources,
    clock: Arc<dyn Clock>,
}

impl LivingCostService {
    pub fn new(sources: LivingCostSources, clock: Arc<dyn Clock>) -> Self {
        Self { sources, clock }
    }

    /// The basic cost of living in `cycle`, with what is still to come.
    ///
    /// ```ignore
    /// let cost = living_costs.for_cycle(cycle).await?;
    /// println!("{}", cost.projected().value());
    /// ```
    pub async fn for_cycle(&self, cycle: Cycle) -> AppResult<LivingCost> {
        let essentials = self.essential_categories().await?;
        let entries = self.entries_in(cycle).await?;
        let bills = self.bills_still_due(cycle).await?;
        let (spent, later) = split_by_date(&entries, self.clock.today(), &essentials);
        let (recent_average, averaged_cycles) = self.recent_average(cycle, &essentials).await?;
        Ok(LivingCost {
            cycle,
            spent,
            still_coming: later + bill_total(&bills, RecurrenceKind::Expense, Some(&essentials)),
            by_category: projected_by_category(&entries, &bills, &essentials),
            recent_average,
            averaged_cycles,
            expected_income: income_of(&entries) + bill_total(&bills, RecurrenceKind::Income, None),
            reserved: self.sources.position.balance_sheet().await?.position.reserved_in_pots,
        })
    }

    async fn essential_categories(&self) -> AppResult<Vec<Category>> {
        let mut found = self.sources.categories.list(Some(CategoryKind::Expense)).await?;
        found.retain(|category| category.essential);
        Ok(found)
    }

    async fn entries_in(&self, cycle: Cycle) -> AppResult<Vec<LedgerEntry>> {
        let filter = EntryFilter {
            from: Some(cycle.start),
            to_inclusive: Some(cycle.last_day()),
            limit: MAX_CYCLE_ENTRIES,
            ..EntryFilter::default()
        };
        self.sources.ledger.list(&filter).await
    }

    /// Recurring bills and income due after today and inside `cycle`.
    async fn bills_still_due(&self, cycle: Cycle) -> AppResult<Vec<Recurrence>> {
        let today = self.clock.today();
        let Ok(days) = u64::try_from((cycle.last_day() - today).num_days()) else {
            return Ok(Vec::new());
        };
        let upcoming = self.sources.recurrences.upcoming(today, days).await?;
        Ok(upcoming
            .into_iter()
            .filter(|(_, date)| cycle.contains(*date))
            .map(|(bill, _)| bill)
            .collect())
    }

    /// Average essential spending of the cycles before `cycle` that were
    /// tracked from their first day; a half-tracked cycle would drag it down.
    async fn recent_average(
        &self,
        cycle: Cycle,
        essentials: &[Category],
    ) -> AppResult<(Option<Cents>, usize)> {
        let start_day = self.sources.settings.get().await?.cycle_start_day;
        let tracked_since = self.tracked_since().await?;
        let mut totals = Vec::new();
        let mut earlier = cycle.previous(start_day);
        while totals.len() < AVERAGED_CYCLES
            && tracked_since.is_some_and(|since| earlier.start >= since)
        {
            totals.push(essential_spending(&self.entries_in(earlier).await?, essentials));
            earlier = earlier.previous(start_day);
        }
        let count = i64::try_from(totals.len()).unwrap_or(1).max(1);
        let average =
            (!totals.is_empty()).then(|| Cents::new(totals.iter().sum::<Cents>().value() / count));
        Ok((average, totals.len()))
    }

    /// The day the first account was opened in finbot.
    async fn tracked_since(&self) -> AppResult<Option<NaiveDate>> {
        let accounts = self.sources.accounts.list(true).await?;
        Ok(accounts.iter().map(|account| account.opened_on).min())
    }
}

fn is_essential(entry: &LedgerEntry, essentials: &[Category]) -> bool {
    entry.category_id.is_some_and(|id| essentials.iter().any(|category| category.id == id))
}

fn essential_spending(entries: &[LedgerEntry], essentials: &[Category]) -> Cents {
    entries
        .iter()
        .filter(|entry| is_essential(entry, essentials))
        .map(|entry| category_effect(entry, CategoryKind::Expense))
        .sum()
}

/// Essential spending up to `today`, and dated later (card installments).
fn split_by_date(
    entries: &[LedgerEntry],
    today: NaiveDate,
    essentials: &[Category],
) -> (Cents, Cents) {
    let (past, later): (Vec<LedgerEntry>, Vec<LedgerEntry>) =
        entries.iter().cloned().partition(|entry| entry.accounting_date <= today);
    (essential_spending(&past, essentials), essential_spending(&later, essentials))
}

fn income_of(entries: &[LedgerEntry]) -> Cents {
    entries.iter().map(|entry| category_effect(entry, CategoryKind::Income)).sum()
}

/// Sum of `kind` bills, only those in `categories` when given.
fn bill_total(
    bills: &[Recurrence],
    kind: RecurrenceKind,
    categories: Option<&[Category]>,
) -> Cents {
    let in_categories = |bill: &&Recurrence| {
        categories.is_none_or(|list| list.iter().any(|category| category.id == bill.category_id))
    };
    bills
        .iter()
        .filter(|bill| bill.kind == kind)
        .filter(in_categories)
        .map(|bill| bill.amount)
        .sum()
}

fn projected_by_category(
    entries: &[LedgerEntry],
    bills: &[Recurrence],
    essentials: &[Category],
) -> Vec<(Category, Cents)> {
    let recorded = category_activity(entries, essentials);
    let mut totals: Vec<(Category, Cents)> = essentials
        .iter()
        .map(|category| {
            let spent = recorded
                .iter()
                .find(|item| item.category.id == category.id)
                .map_or(Cents::ZERO, |item| item.total);
            let coming =
                bill_total(bills, RecurrenceKind::Expense, Some(std::slice::from_ref(category)));
            (category.clone(), spent + coming)
        })
        .filter(|(_, total)| total.value() != 0)
        .collect();
    totals.sort_by_key(|(_, total)| -total.value());
    totals
}
