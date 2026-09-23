use std::sync::Arc;

use chrono::NaiveDate;
use domain::goal_progress::{progress_bp, remaining};
use domain::{AccountKind, Cents};

use super::ledger::{EntryOrigin, EntryRequest, TransferEntry};
use super::text_rules::clean_name;
use super::{AccountService, LedgerService};
use crate::model::{AccountId, Goal, GoalId, GoalProgress, GoalTarget, LedgerEntry, NewAccount};
use crate::ports::{Clock, GoalStore};
use crate::services::accounts::MAX_ACCOUNT_NAME_CHARS;
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGoal {
    pub name: String,
    pub target: Cents,
    pub target_date: Option<NaiveDate>,
    /// Money already saved before using finbot; becomes the pot's
    /// initial balance, not income.
    pub already_saved: Cents,
}

/// Money moved into (`/guardar`) or out of (`/resgatar`) a goal's pot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PotMove {
    pub goal_id: GoalId,
    pub account_id: AccountId,
    pub amount: Cents,
    pub description: String,
}

pub struct GoalService {
    goals: Arc<dyn GoalStore>,
    accounts: Arc<AccountService>,
    ledger: Arc<LedgerService>,
    clock: Arc<dyn Clock>,
}

impl GoalService {
    pub fn new(
        goals: Arc<dyn GoalStore>,
        accounts: Arc<AccountService>,
        ledger: Arc<LedgerService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { goals, accounts, ledger, clock }
    }

    /// Creates a goal and its pot.
    ///
    /// ```ignore
    /// goals.create(CreateGoal { name: "Casa própria".into(), target: Cents::new(10_000_000),
    ///     target_date: None, already_saved: Cents::new(2_000_000) }).await?;
    /// ```
    pub async fn create(&self, request: CreateGoal) -> AppResult<Goal> {
        ensure_positive_target(request.target)?;
        if request.already_saved.value() < 0 {
            return Err(AppError::invalid(
                "already saved",
                request.already_saved.value(),
                "zero or more cents",
            ));
        }
        let pot = NewAccount {
            name: clean_name("goal name", &request.name, MAX_ACCOUNT_NAME_CHARS)?,
            kind: AccountKind::Pot,
            initial_balance: request.already_saved,
            opened_on: self.clock.today(),
        };
        let target = GoalTarget { target: request.target, target_date: request.target_date };
        Ok(self.goals.create_goal(pot, target).await?)
    }

    pub async fn list_progress(&self) -> AppResult<Vec<GoalProgress>> {
        let goals = self.goals.list_goals().await?;
        let balances = self.accounts.balances().await?;
        let saved_in = |pot: AccountId| {
            balances
                .iter()
                .find(|item| item.account.id == pot)
                .map_or(Cents::ZERO, |item| item.balance)
        };
        Ok(goals.into_iter().map(|goal| progress_of(saved_in(goal.pot.id), goal)).collect())
    }

    /// Renames a goal, which is the name of its pot.
    pub async fn rename(&self, id: GoalId, name: &str) -> AppResult<Goal> {
        let goal = self.require_goal(id).await?;
        self.accounts.rename(goal.pot.id, name).await?;
        self.goals.find_goal(id).await?.ok_or_else(|| AppError::not_found("goal", id))
    }

    /// Changes how much to save, keeping the deadline.
    pub async fn set_target(&self, id: GoalId, target: Cents) -> AppResult<Goal> {
        let current = self.require_goal(id).await?.target;
        self.update_target(id, GoalTarget { target, ..current }).await
    }

    /// Changes the deadline, keeping the amount; `None` clears it.
    pub async fn set_deadline(
        &self,
        id: GoalId,
        target_date: Option<NaiveDate>,
    ) -> AppResult<Goal> {
        let current = self.require_goal(id).await?.target;
        self.update_target(id, GoalTarget { target_date, ..current }).await
    }

    pub async fn update_target(&self, id: GoalId, target: GoalTarget) -> AppResult<Goal> {
        ensure_positive_target(target.target)?;
        let updated = self.goals.update_goal_target(id, target).await?;
        updated.ok_or_else(|| AppError::not_found("goal", id))
    }

    pub async fn deposit(&self, request: PotMove, origin: EntryOrigin) -> AppResult<LedgerEntry> {
        let goal = self.require_goal(request.goal_id).await?;
        let transfer = pot_transfer(request.account_id, goal.pot.id, &request);
        self.ledger.record(EntryRequest::Transfer(transfer), origin).await
    }

    /// Moves money back out of the pot; the pot cannot go negative.
    pub async fn withdraw(&self, request: PotMove, origin: EntryOrigin) -> AppResult<LedgerEntry> {
        let goal = self.require_goal(request.goal_id).await?;
        let saved = self.accounts.balance_of(goal.pot.id).await?;
        if request.amount > saved {
            let expected =
                format!("at most the {} cents saved in {}", saved.value(), goal.pot.name);
            return Err(AppError::invalid("amount", request.amount.value(), expected));
        }
        let transfer = pot_transfer(goal.pot.id, request.account_id, &request);
        self.ledger.record(EntryRequest::Transfer(transfer), origin).await
    }

    async fn require_goal(&self, id: GoalId) -> AppResult<Goal> {
        match self.goals.find_goal(id).await? {
            Some(goal) if !goal.pot.archived => Ok(goal),
            _ => Err(AppError::not_found("goal", id)),
        }
    }
}

fn ensure_positive_target(target: Cents) -> AppResult<()> {
    if target.is_positive() {
        return Ok(());
    }
    Err(AppError::invalid("goal target", target.value(), "a positive number of cents"))
}

fn pot_transfer(from: AccountId, to: AccountId, request: &PotMove) -> TransferEntry {
    TransferEntry {
        from_account_id: from,
        to_account_id: to,
        amount: request.amount,
        description: request.description.clone(),
        date: None,
    }
}

fn progress_of(saved: Cents, goal: Goal) -> GoalProgress {
    let target = goal.target.target;
    GoalProgress {
        saved,
        remaining: remaining(saved, target),
        progress_bp: progress_bp(saved, target),
        goal,
    }
}
