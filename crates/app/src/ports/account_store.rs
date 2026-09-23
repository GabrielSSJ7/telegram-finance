use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::balance::AccountFlow;

use super::StoreResult;
use crate::model::{Account, AccountId, NewAccount};

#[async_trait]
pub trait AccountStore: Send + Sync {
    async fn create_account(&self, account: NewAccount) -> StoreResult<Account>;
    async fn list_accounts(&self, include_archived: bool) -> StoreResult<Vec<Account>>;
    async fn find_account(&self, id: AccountId) -> StoreResult<Option<Account>>;
    /// Returns false when the account does not exist or is already archived.
    async fn archive_account(&self, id: AccountId, at: DateTime<Utc>) -> StoreResult<bool>;
    /// The renamed account; `None` when it does not exist or is archived.
    async fn rename_account(&self, id: AccountId, name: &str) -> StoreResult<Option<Account>>;
    /// Totals of non-deleted entries per account, kind and role with
    /// `accounting_date <= up_to`.
    async fn account_flows(&self, up_to: NaiveDate) -> StoreResult<Vec<(AccountId, AccountFlow)>>;
}
