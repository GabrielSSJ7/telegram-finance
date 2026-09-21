//! `InMemoryStore`: one fake that implements every store port over shared
//! state, so balances computed from entries stay consistent across ports.
//! It mimics the unique rules and draft idempotency of the Postgres schema.

mod accounts;
mod api_keys;
mod categories;
mod entries;
mod goals;
mod members;
mod settings;

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, NaiveTime, Utc};
use domain::DayOfMonth;

use crate::model::{
    Account, AccountId, ApiKey, Category, CategoryId, CategoryKind, DraftId, GoalId, GoalTarget,
    HouseholdSettings, LedgerEntry, Member, NewAccount,
};
use crate::ports::StoreError;

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

#[derive(Debug)]
struct MemoryState {
    accounts: Vec<Account>,
    goals: Vec<GoalRow>,
    categories: Vec<Category>,
    entries: Vec<LedgerEntry>,
    drafts: HashSet<DraftId>,
    members: Vec<Member>,
    settings: HouseholdSettings,
    api_keys: Vec<ApiKeyRow>,
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
        let settings = HouseholdSettings {
            telegram_chat_id: None,
            cycle_start_day: DayOfMonth::new(1)
                .unwrap_or_else(|_| unreachable!("1 is a valid day")),
            daily_report_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap_or_default(),
        };
        let state = MemoryState {
            accounts: Vec::new(),
            goals: Vec::new(),
            categories: Vec::new(),
            entries: Vec::new(),
            drafts: HashSet::new(),
            members: Vec::new(),
            settings,
            api_keys: Vec::new(),
        };
        Self { state: Mutex::new(state) }
    }

    /// Adds an active category directly and returns its id.
    pub fn seed_category(&self, name: &str, kind: CategoryKind) -> CategoryId {
        let id = CategoryId::generate();
        let category = Category { id, name: name.to_owned(), kind, emoji: None, archived: false };
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
