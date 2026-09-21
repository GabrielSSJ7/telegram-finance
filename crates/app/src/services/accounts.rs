use std::sync::Arc;

use chrono::NaiveDate;
use domain::{AccountKind, Cents};

use super::balances::balances_from_flows;
use super::text_rules::clean_name;
use crate::model::{Account, AccountBalance, AccountId, NewAccount};
use crate::ports::{AccountStore, Clock};
use crate::{AppError, AppResult};

pub const MAX_ACCOUNT_NAME_CHARS: usize = 40;

/// Request to open a bank, savings or cash account. Pots are opened
/// through `GoalService`, since every pot belongs to a goal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAccount {
    pub name: String,
    pub kind: AccountKind,
    pub initial_balance: Cents,
    pub opened_on: Option<NaiveDate>,
}

pub struct AccountService {
    accounts: Arc<dyn AccountStore>,
    clock: Arc<dyn Clock>,
}

impl AccountService {
    pub fn new(accounts: Arc<dyn AccountStore>, clock: Arc<dyn Clock>) -> Self {
        Self { accounts, clock }
    }

    /// Opens an account.
    ///
    /// ```ignore
    /// let nubank = service.open(OpenAccount { name: "Nubank".into(), kind: AccountKind::Checking,
    ///     initial_balance: Cents::new(150_000), opened_on: None }).await?;
    /// ```
    pub async fn open(&self, request: OpenAccount) -> AppResult<Account> {
        if request.kind == AccountKind::Pot {
            return Err(AppError::invalid(
                "account kind",
                "pot",
                "checking, savings or cash (pots come with goals)",
            ));
        }
        let account = NewAccount {
            name: clean_name("account name", &request.name, MAX_ACCOUNT_NAME_CHARS)?,
            kind: request.kind,
            initial_balance: request.initial_balance,
            opened_on: request.opened_on.unwrap_or_else(|| self.clock.today()),
        };
        Ok(self.accounts.create_account(account).await?)
    }

    pub async fn list(&self, include_archived: bool) -> AppResult<Vec<Account>> {
        Ok(self.accounts.list_accounts(include_archived).await?)
    }

    pub async fn archive(&self, id: AccountId) -> AppResult<()> {
        let archived = self.accounts.archive_account(id, self.clock.now()).await?;
        if !archived {
            return Err(AppError::not_found("active account", id));
        }
        Ok(())
    }

    /// The account if it exists and is not archived.
    pub async fn require_active(&self, id: AccountId) -> AppResult<Account> {
        match self.accounts.find_account(id).await? {
            Some(account) if !account.archived => Ok(account),
            _ => Err(AppError::not_found("active account", id)),
        }
    }

    /// Balance of every active account as of today.
    pub async fn balances(&self) -> AppResult<Vec<AccountBalance>> {
        let accounts = self.accounts.list_accounts(false).await?;
        let flows = self.accounts.account_flows(self.clock.today()).await?;
        Ok(balances_from_flows(accounts, &flows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FixedClock, InMemoryStore};

    fn service() -> AccountService {
        let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        AccountService::new(
            Arc::new(InMemoryStore::new()),
            Arc::new(FixedClock::at_local_noon(today)),
        )
    }

    fn checking(name: &str, initial: i64) -> OpenAccount {
        OpenAccount {
            name: name.into(),
            kind: AccountKind::Checking,
            initial_balance: Cents::new(initial),
            opened_on: None,
        }
    }

    #[tokio::test]
    async fn open_trims_name_and_defaults_date_to_today() {
        let account = service().open(checking("  Nubank ", 100)).await.unwrap();
        assert_eq!(account.name, "Nubank");
        assert_eq!(account.opened_on, NaiveDate::from_ymd_opt(2026, 3, 10).unwrap());
    }

    #[tokio::test]
    async fn open_rejects_pots_and_duplicate_names() {
        let service = service();
        let pot = OpenAccount { kind: AccountKind::Pot, ..checking("Casa", 0) };
        assert!(matches!(service.open(pot).await, Err(AppError::Invalid { .. })));
        service.open(checking("Nubank", 0)).await.unwrap();
        assert!(matches!(service.open(checking("nubank", 0)).await, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn archive_hides_account_and_fails_twice() {
        let service = service();
        let account = service.open(checking("Itaú", 0)).await.unwrap();
        service.archive(account.id).await.unwrap();
        assert!(service.list(false).await.unwrap().is_empty());
        assert!(matches!(service.archive(account.id).await, Err(AppError::NotFound { .. })));
        assert!(service.require_active(account.id).await.is_err());
    }

    #[tokio::test]
    async fn balances_start_from_initial_balances() {
        let service = service();
        service.open(checking("Nubank", 150_000)).await.unwrap();
        service
            .open(OpenAccount { kind: AccountKind::Cash, ..checking("Carteira", 5_000) })
            .await
            .unwrap();
        let balances: Vec<i64> =
            service.balances().await.unwrap().iter().map(|item| item.balance.value()).collect();
        assert_eq!(balances, vec![150_000, 5_000]);
    }
}
