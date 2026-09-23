//! Recurring entries: salary, rent, subscriptions. Each (recurrence, date)
//! is recorded at most once ever, through a draft id derived from both, so
//! a restart, a double tap or an undo never produces a duplicate.

use std::sync::Arc;

use chrono::{Days, NaiveDate};
use domain::Cents;
use domain::recurrence::{InstallmentPlan, due_dates};

use super::card_spending::CardPurchaseRequest;
use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use super::text_rules::clean_name;
use super::{AccountService, CardService, CategoryService, LedgerService};
use crate::model::{
    CategoryKind, DraftId, MemberId, NewRecurrence, Recurrence, RecurrenceEdit, RecurrenceId,
    RecurrenceKind, RecurrenceMode, RecurrenceTarget,
};
use crate::ports::{Clock, RecurrenceStore};
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRecurrence {
    pub kind: RecurrenceKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: crate::model::CategoryId,
    pub target: RecurrenceTarget,
    pub day: domain::DayOfMonth,
    pub mode: RecurrenceMode,
    /// Defaults to today.
    pub starts_on: Option<NaiveDate>,
    /// Ends the recurrence after a number of installments (a financing).
    pub plan: Option<InstallmentPlan>,
}

pub struct RecurrenceService {
    store: Arc<dyn RecurrenceStore>,
    ledger: Arc<LedgerService>,
    cards: Arc<CardService>,
    accounts: Arc<AccountService>,
    categories: Arc<CategoryService>,
    clock: Arc<dyn Clock>,
}

/// What `RecurrenceService::new` needs besides its store and clock.
pub struct RecurrenceDependencies {
    pub ledger: Arc<LedgerService>,
    pub cards: Arc<CardService>,
    pub accounts: Arc<AccountService>,
    pub categories: Arc<CategoryService>,
}

impl RecurrenceService {
    pub fn new(
        store: Arc<dyn RecurrenceStore>,
        dependencies: RecurrenceDependencies,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let RecurrenceDependencies { ledger, cards, accounts, categories } = dependencies;
        Self { store, ledger, cards, accounts, categories, clock }
    }

    pub async fn create(&self, request: CreateRecurrence) -> AppResult<Recurrence> {
        if !request.amount.is_positive() {
            return Err(AppError::invalid(
                "amount",
                request.amount.value(),
                "a positive number of cents",
            ));
        }
        let expected = if request.kind == RecurrenceKind::Income {
            CategoryKind::Income
        } else {
            CategoryKind::Expense
        };
        self.categories.require_kind(request.category_id, expected).await?;
        self.check_target(request.kind, request.target).await?;
        check_plan(request.plan)?;
        let description = clean_name("description", &request.description, 60)?;
        let starts_on = request.starts_on.unwrap_or_else(|| self.clock.today());
        Ok(self.store.create_recurrence(new_recurrence(&request, description, starts_on)).await?)
    }

    pub async fn list(&self, include_inactive: bool) -> AppResult<Vec<Recurrence>> {
        Ok(self.store.list_recurrences(include_inactive).await?)
    }

    pub async fn find(&self, id: RecurrenceId) -> AppResult<Recurrence> {
        let found = self.store.find_recurrence(id).await?;
        found.ok_or_else(|| AppError::not_found("recurrence", id))
    }

    /// Changes the amount, the day or the mode of a recurring entry.
    pub async fn update(&self, id: RecurrenceId, edit: RecurrenceEdit) -> AppResult<Recurrence> {
        if edit.amount.is_some_and(|amount| !amount.is_positive()) {
            let amount = edit.amount.unwrap_or(Cents::ZERO).value();
            return Err(AppError::invalid("amount", amount, "a positive number of cents"));
        }
        let updated = self.store.update_recurrence(id, edit).await?;
        updated.ok_or_else(|| AppError::not_found("active recurrence", id))
    }

    pub async fn deactivate(&self, id: RecurrenceId) -> AppResult<()> {
        if !self.store.deactivate_recurrence(id).await? {
            return Err(AppError::not_found("active recurrence", id));
        }
        Ok(())
    }

    /// Active recurrences with dates due up to `today`, oldest first.
    pub async fn due(&self, today: NaiveDate) -> AppResult<Vec<(Recurrence, Vec<NaiveDate>)>> {
        let active = self.store.list_recurrences(false).await?;
        let due = active.into_iter().map(|recurrence| {
            let mut dates = due_dates(
                recurrence.day,
                recurrence.starts_on,
                recurrence.last_generated_on,
                today,
            );
            dates.retain(|date| recurrence.runs_on(*date));
            (recurrence, dates)
        });
        Ok(due.filter(|(_, dates)| !dates.is_empty()).collect())
    }

    /// Occurrences in the `days` after `today` (for "coming up" lists).
    pub async fn upcoming(
        &self,
        today: NaiveDate,
        days: u64,
    ) -> AppResult<Vec<(Recurrence, NaiveDate)>> {
        let horizon = today.checked_add_days(Days::new(days)).unwrap_or(today);
        let active = self.store.list_recurrences(false).await?;
        let upcoming = active.into_iter().flat_map(|recurrence| {
            let dates = due_dates(
                recurrence.day,
                recurrence.starts_on.max(today.succ_opt().unwrap_or(today)),
                None,
                horizon,
            );
            let dates =
                dates.into_iter().filter(|date| recurrence.runs_on(*date)).collect::<Vec<_>>();
            dates.into_iter().map(move |date| (recurrence.clone(), date))
        });
        Ok(upcoming.collect())
    }

    /// Records the occurrence on `date`; false when it was already recorded.
    pub async fn record(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
        amount: Cents,
        by: Option<MemberId>,
    ) -> AppResult<bool> {
        let draft = DraftId::derived(&format!("recurrence:{}:{date}", recurrence.id));
        let origin = EntryOrigin { created_by: by, draft: Some(draft) };
        match self.record_with(recurrence, date, amount, origin).await {
            Ok(()) => Ok(true),
            Err(AppError::AlreadyCommitted) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Records that `date` was generated; a plan's last installment also
    /// ends the recurrence, so it leaves `/recorrentes`.
    pub async fn mark_generated(&self, id: RecurrenceId, date: NaiveDate) -> AppResult<()> {
        self.store.mark_generated(id, date).await?;
        let finished = self.find(id).await?.last_due().is_some_and(|last| date >= last);
        if finished {
            self.store.deactivate_recurrence(id).await?;
        }
        Ok(())
    }

    async fn record_with(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
        amount: Cents,
        origin: EntryOrigin,
    ) -> AppResult<()> {
        let RecurrenceTarget::Card(card_id) = recurrence.target else {
            return self.record_on_account(recurrence, date, amount, origin).await;
        };
        let request = CardPurchaseRequest {
            card_id,
            category_id: recurrence.category_id,
            total: amount,
            installments: 1,
            first_installment_no: 1,
            description: recurrence.description_on(date),
            purchased_on: Some(date),
        };
        self.cards.purchase(request, origin).await.map(|_| ())
    }

    async fn record_on_account(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
        amount: Cents,
        origin: EntryOrigin,
    ) -> AppResult<()> {
        let RecurrenceTarget::Account(account_id) = recurrence.target else {
            return Err(AppError::invalid("recurrence target", "card", "an account"));
        };
        let (category_id, description) = (recurrence.category_id, recurrence.description_on(date));
        let entry = AccountEntry { account_id, category_id, amount, description, date: Some(date) };
        let request = match recurrence.kind {
            RecurrenceKind::Income => EntryRequest::Income(entry),
            RecurrenceKind::Expense => EntryRequest::Expense(entry),
        };
        self.ledger.record(request, origin).await.map(|_| ())
    }

    async fn check_target(&self, kind: RecurrenceKind, target: RecurrenceTarget) -> AppResult<()> {
        match (kind, target) {
            (_, RecurrenceTarget::Account(account)) => {
                self.accounts.require_active(account).await.map(|_| ())
            }
            (RecurrenceKind::Expense, RecurrenceTarget::Card(card)) => {
                self.cards.require_active(card).await.map(|_| ())
            }
            (RecurrenceKind::Income, RecurrenceTarget::Card(card)) => Err(AppError::invalid(
                "recurrence target",
                card,
                "an account (income cannot go to a card)",
            )),
        }
    }
}

fn new_recurrence(
    request: &CreateRecurrence,
    description: String,
    starts_on: NaiveDate,
) -> NewRecurrence {
    NewRecurrence {
        kind: request.kind,
        amount: request.amount,
        description,
        category_id: request.category_id,
        target: request.target,
        day: request.day,
        mode: request.mode,
        starts_on,
        plan: request.plan,
    }
}

/// Longest plan accepted: 40 years of monthly installments.
pub const MAX_PLAN_INSTALLMENTS: u32 = 480;

fn check_plan(plan: Option<InstallmentPlan>) -> AppResult<()> {
    let Some(plan) = plan else {
        return Ok(());
    };
    if (1..=MAX_PLAN_INSTALLMENTS).contains(&plan.count)
        && (1..=plan.count).contains(&plan.first_number)
    {
        return Ok(());
    }
    let value = format!("{}/{}", plan.first_number, plan.count);
    Err(AppError::invalid(
        "installments",
        value,
        format!("1 to {MAX_PLAN_INSTALLMENTS} installments, starting at one of them"),
    ))
}
