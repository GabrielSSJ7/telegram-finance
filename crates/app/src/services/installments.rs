//! `/parcelas`: card purchases with installments still to come and
//! recurring plans (financings, loans) paid from an account or card.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::NaiveDate;
use domain::Cents;
use domain::installments::split_amount;

use super::{AccountService, LedgerService, RecurrenceService};
use crate::AppResult;
use crate::model::{
    CardPurchase, CreditCard, EntryFilter, InstallmentProgress, LedgerEntry, PlanSource,
    PurchaseId, Recurrence, RecurrenceTarget,
};
use crate::ports::{CardStore, Clock};

/// Far more installment rows than a household ever has.
const MAX_INSTALLMENT_ROWS: u32 = 50_000;

pub struct InstallmentSources {
    pub ledger: Arc<LedgerService>,
    pub recurrences: Arc<RecurrenceService>,
    pub accounts: Arc<AccountService>,
    pub card_store: Arc<dyn CardStore>,
}

pub struct InstallmentService {
    sources: InstallmentSources,
    clock: Arc<dyn Clock>,
}

impl InstallmentService {
    pub fn new(sources: InstallmentSources, clock: Arc<dyn Clock>) -> Self {
        Self { sources, clock }
    }

    /// Every plan with installments still to come, ending soonest first.
    ///
    /// ```ignore
    /// for plan in installments.running().await? { println!("{}", plan.remaining().value()); }
    /// ```
    pub async fn running(&self) -> AppResult<Vec<InstallmentProgress>> {
        let cards = self.sources.card_store.list_cards(true).await?;
        let mut plans = self.card_plans(&cards).await?;
        plans.extend(self.recurring_plans(&cards).await?);
        plans.sort_by_key(|plan| plan.last_due);
        Ok(plans)
    }

    async fn card_plans(&self, cards: &[CreditCard]) -> AppResult<Vec<InstallmentProgress>> {
        let today = self.clock.today();
        let mut plans = Vec::new();
        for (purchase_id, rows) in self.installments_by_purchase().await? {
            let still_running = rows.iter().any(|row| row.accounting_date > today);
            let Some(purchase) = self.sources.card_store.find_purchase(purchase_id).await? else {
                continue;
            };
            if still_running && purchase.installment_count > 1 {
                plans.push(card_progress(&purchase, &rows, card_name(cards, &purchase), today));
            }
        }
        Ok(plans)
    }

    async fn installments_by_purchase(&self) -> AppResult<BTreeMap<PurchaseId, Vec<LedgerEntry>>> {
        let filter = EntryFilter {
            kind: Some(domain::EntryKind::CardInstallment),
            limit: MAX_INSTALLMENT_ROWS,
            ..EntryFilter::default()
        };
        let mut groups: BTreeMap<PurchaseId, Vec<LedgerEntry>> = BTreeMap::new();
        for row in self.sources.ledger.list(&filter).await? {
            if let Some(purchase) = row.card_purchase_id {
                groups.entry(purchase).or_default().push(row);
            }
        }
        Ok(groups)
    }

    async fn recurring_plans(&self, cards: &[CreditCard]) -> AppResult<Vec<InstallmentProgress>> {
        let accounts = self.sources.accounts.list(true).await?;
        let recurrences = self.sources.recurrences.list(false).await?;
        let source_of = |recurrence: &Recurrence| match recurrence.target {
            RecurrenceTarget::Account(id) => {
                let found = accounts.iter().find(|account| account.id == id);
                PlanSource::Account(found.map_or_else(String::new, |account| account.name.clone()))
            }
            RecurrenceTarget::Card(id) => {
                let found = cards.iter().find(|card| card.id == id);
                PlanSource::Card(found.map_or_else(String::new, |card| card.name.clone()))
            }
        };
        Ok(recurrences
            .iter()
            .filter_map(|recurrence| recurring_progress(recurrence, source_of(recurrence)))
            .collect())
    }
}

fn card_name(cards: &[CreditCard], purchase: &CardPurchase) -> String {
    let found = cards.iter().find(|card| card.id == purchase.card_id);
    found.map_or_else(String::new, |card| card.name.clone())
}

/// Installments dated up to `today` count as paid, and so do the ones
/// before `first_installment_no` of a plan entered midway.
fn card_progress(
    purchase: &CardPurchase,
    rows: &[LedgerEntry],
    card: String,
    today: NaiveDate,
) -> InstallmentProgress {
    let amounts = split_amount(purchase.total, purchase.installment_count).unwrap_or_default();
    let dated =
        u32::try_from(rows.iter().filter(|row| row.accounting_date <= today).count()).unwrap_or(0);
    let paid_count = purchase.first_installment_no - 1 + dated;
    let paid = amounts.iter().take(paid_count as usize).copied().sum();
    InstallmentProgress {
        description: purchase.description.clone(),
        source: PlanSource::Card(card),
        category_id: purchase.category_id,
        count: purchase.installment_count,
        paid_count,
        total: purchase.total,
        paid,
        installment: amounts.last().copied().unwrap_or(Cents::ZERO),
        last_due: rows.iter().map(|row| row.accounting_date).max().unwrap_or(purchase.purchased_on),
    }
}

/// A recurrence with a plan; the installments generated so far are paid.
fn recurring_progress(recurrence: &Recurrence, source: PlanSource) -> Option<InstallmentProgress> {
    let plan = recurrence.plan?;
    let first_due = recurrence.first_due();
    let paid_count = recurrence
        .last_generated_on
        .map_or(plan.first_number - 1, |last| plan.number_on(first_due, last).min(plan.count));
    Some(InstallmentProgress {
        description: recurrence.description.clone(),
        source,
        category_id: recurrence.category_id,
        count: plan.count,
        paid_count,
        total: recurrence.amount.times(i64::from(plan.count)),
        paid: recurrence.amount.times(i64::from(paid_count)),
        installment: recurrence.amount,
        last_due: recurrence.last_due()?,
    })
}
