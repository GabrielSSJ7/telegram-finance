use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::balance::AccountFlow;
use domain::entry_kind::AccountRole;

use super::{InMemoryStore, account_from, name_taken, same_name, unique_violation};
use crate::model::{Account, AccountId, LedgerEntry, NewAccount};
use crate::ports::{AccountStore, StoreResult};

#[async_trait]
impl AccountStore for InMemoryStore {
    async fn rename_account(&self, id: AccountId, name: &str) -> StoreResult<Option<Account>> {
        let mut state = self.lock();
        let taken = state
            .accounts
            .iter()
            .any(|row| row.id != id && !row.archived && same_name(&row.name, name));
        if taken {
            return Err(unique_violation("accounts_active_name"));
        }
        let Some(row) = state.accounts.iter_mut().find(|row| row.id == id && !row.archived) else {
            return Ok(None);
        };
        name.clone_into(&mut row.name);
        Ok(Some(row.clone()))
    }

    async fn create_account(&self, account: NewAccount) -> StoreResult<Account> {
        let mut state = self.lock();
        if name_taken(&state, &account.name) {
            return Err(unique_violation("accounts_active_name"));
        }
        let created = account_from(account);
        state.accounts.push(created.clone());
        Ok(created)
    }

    async fn list_accounts(&self, include_archived: bool) -> StoreResult<Vec<Account>> {
        let state = self.lock();
        Ok(state.accounts.iter().filter(|row| include_archived || !row.archived).cloned().collect())
    }

    async fn find_account(&self, id: AccountId) -> StoreResult<Option<Account>> {
        Ok(self.lock().accounts.iter().find(|row| row.id == id).cloned())
    }

    async fn archive_account(&self, id: AccountId, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(row) = state.accounts.iter_mut().find(|row| row.id == id && !row.archived) else {
            return Ok(false);
        };
        row.archived = true;
        Ok(true)
    }

    async fn account_flows(&self, up_to: NaiveDate) -> StoreResult<Vec<(AccountId, AccountFlow)>> {
        let state = self.lock();
        let live =
            state.entries.iter().filter(|entry| !entry.deleted && entry.accounting_date <= up_to);
        Ok(live.flat_map(entry_flows).collect())
    }
}

/// One flow per account column the entry fills. The Postgres store sums
/// these per group; balances only need the totals, so unsummed is fine.
fn entry_flows(entry: &LedgerEntry) -> Vec<(AccountId, AccountFlow)> {
    let primary = entry.account_id.map(|id| (id, AccountRole::Primary));
    let counter = entry.counter_account_id.map(|id| (id, AccountRole::Counter));
    [primary, counter]
        .into_iter()
        .flatten()
        .map(|(id, role)| (id, AccountFlow { kind: entry.kind, role, total: entry.amount }))
        .collect()
}
