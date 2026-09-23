use app::model::{Account, AccountId, NewAccount};
use app::ports::{AccountStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::balance::AccountFlow;
use domain::entry_kind::AccountRole;
use domain::{AccountKind, Cents, EntryKind};
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::{corrupt, store_error};

pub(crate) struct AccountRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub initial_balance_cents: i64,
    pub opened_on: NaiveDate,
    pub archived_at: Option<DateTime<Utc>>,
}

impl AccountRow {
    pub(crate) fn into_account(self) -> StoreResult<Account> {
        let kind =
            self.kind.parse::<AccountKind>().map_err(|error| corrupt("accounts.kind", error))?;
        Ok(Account {
            id: AccountId(self.id),
            name: self.name,
            kind,
            initial_balance: Cents::new(self.initial_balance_cents),
            opened_on: self.opened_on,
            archived: self.archived_at.is_some(),
        })
    }
}

struct FlowRow {
    account_id: Uuid,
    kind: String,
    role: String,
    total: i64,
}

impl FlowRow {
    fn into_flow(self) -> StoreResult<(AccountId, AccountFlow)> {
        let kind = self
            .kind
            .parse::<EntryKind>()
            .map_err(|error| corrupt("ledger_entries.kind", error))?;
        let role = if self.role == "counter" { AccountRole::Counter } else { AccountRole::Primary };
        Ok((AccountId(self.account_id), AccountFlow { kind, role, total: Cents::new(self.total) }))
    }
}

#[async_trait]
impl AccountStore for PgStore {
    async fn create_account(&self, account: NewAccount) -> StoreResult<Account> {
        let row = sqlx::query_as!(
            AccountRow,
            "insert into accounts (name, kind, initial_balance_cents, opened_on) values ($1, $2, $3, $4)
             returning id, name, kind, initial_balance_cents, opened_on, archived_at",
            account.name,
            account.kind.as_str(),
            account.initial_balance.value(),
            account.opened_on,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_account()
    }

    async fn list_accounts(&self, include_archived: bool) -> StoreResult<Vec<Account>> {
        let rows = sqlx::query_as!(
            AccountRow,
            "select id, name, kind, initial_balance_cents, opened_on, archived_at from accounts
             where $1 or archived_at is null order by created_at, id",
            include_archived,
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows.into_iter().map(AccountRow::into_account).collect()
    }

    async fn find_account(&self, id: AccountId) -> StoreResult<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            "select id, name, kind, initial_balance_cents, opened_on, archived_at from accounts where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(AccountRow::into_account).transpose()
    }

    async fn rename_account(&self, id: AccountId, name: &str) -> StoreResult<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            "update accounts set name = $2 where id = $1 and archived_at is null
             returning id, name, kind, initial_balance_cents, opened_on, archived_at",
            id.0,
            name,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(AccountRow::into_account).transpose()
    }

    async fn archive_account(&self, id: AccountId, at: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query!(
            "update accounts set archived_at = $2 where id = $1 and archived_at is null",
            id.0,
            at
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn account_flows(&self, up_to: NaiveDate) -> StoreResult<Vec<(AccountId, AccountFlow)>> {
        let rows = sqlx::query_file_as!(FlowRow, "queries/account_flows.sql", up_to,)
            .fetch_all(self.pool())
            .await
            .map_err(store_error)?;
        rows.into_iter().map(FlowRow::into_flow).collect()
    }
}
