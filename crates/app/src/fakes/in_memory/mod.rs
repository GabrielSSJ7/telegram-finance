//! `InMemoryStore`: one fake that implements every store port over shared
//! state, so balances computed from entries stay consistent across ports.
//! It mimics the unique rules and draft idempotency of the Postgres schema.

mod accounts;
mod api_keys;
mod budgets;
mod cards;
mod categories;
mod chat_flows;
mod entries;
mod goals;
mod job_runs;
mod members;
mod recurrences;
mod reports;
mod settings;

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, NaiveDate, Utc};

use crate::model::{
    Account, AccountId, ApiKey, Budget, BudgetId, CardPurchase, Category, CategoryId, CategoryKind,
    CreditCard, DraftId, GoalId, GoalTarget, HouseholdSettings, Invoice, LedgerEntry, Member,
    NewAccount, Recurrence,
};
use crate::ports::{ChatUserKey, StoreError, StoredFlow};

#[derive(Debug)]
struct GoalRow {
    id: GoalId,
    account_id: AccountId,
    target: GoalTarget,
}

#[derive(Debug)]
struct ApiKeyRow {
    key: ApiKey,
    token_sha256: [u8; 32],
    last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
struct MemoryState {
    accounts: Vec<Account>,
    goals: Vec<GoalRow>,
    categories: Vec<Category>,
    entries: Vec<LedgerEntry>,
    drafts: HashSet<DraftId>,
    members: Vec<Member>,
    settings: HouseholdSettings,
    api_keys: Vec<ApiKeyRow>,
    flows: HashMap<ChatUserKey, StoredFlow>,
    update_offset: Option<i64>,
    cards: Vec<CreditCard>,
    invoices: Vec<Invoice>,
    purchases: Vec<CardPurchase>,
    recurrences: Vec<Recurrence>,
    job_runs: HashMap<(String, NaiveDate), JobRunRow>,
    budgets: Vec<Budget>,
    budget_alerts: HashSet<(BudgetId, NaiveDate, u8)>,
}

#[derive(Debug, Clone, Copy)]
struct JobRunRow {
    succeeded: bool,
    running: bool,
    attempts: u32,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct InMemoryStore {
    state: Mutex<MemoryState>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    /// Empty store with the default settings (cycle day 1, report 21:00).
    pub fn new() -> Self {
        Self { state: Mutex::new(MemoryState::default()) }
    }

    /// Adds an active category directly and returns its id.
    pub fn seed_category(&self, name: &str, kind: CategoryKind) -> CategoryId {
        let id = CategoryId::generate();
        let category = Category {
            id,
            name: name.to_owned(),
            kind,
            emoji: None,
            archived: false,
            essential: false,
        };
        self.lock().categories.push(category);
        id
    }

    fn lock(&self) -> MutexGuard<'_, MemoryState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn account_from(new: NewAccount) -> Account {
    Account {
        id: AccountId::generate(),
        name: new.name,
        kind: new.kind,
        initial_balance: new.initial_balance,
        opened_on: new.opened_on,
        archived: false,
    }
}

fn name_taken(state: &MemoryState, name: &str) -> bool {
    state.accounts.iter().any(|row| !row.archived && same_name(&row.name, name))
}

fn unique_violation(constraint: &str) -> StoreError {
    StoreError::UniqueViolation { constraint: constraint.to_owned() }
}

fn same_name(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}
