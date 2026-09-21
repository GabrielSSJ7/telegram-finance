//! `/ajuste`: makes an account match the bank. The difference between the
//! real balance and finbot's is recorded as an adjustment, which is
//! neither income nor spending, so reports stay honest.

use std::sync::Arc;

use domain::Cents;

use super::ledger::{AdjustmentEntry, EntryOrigin, EntryRequest};
use super::{AccountService, LedgerService};
use crate::model::{AccountId, LedgerEntry};
use crate::{AppError, AppResult};

/// The balance the bank shows for `account_id` right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileBalance {
    pub account_id: AccountId,
    pub actual_balance: Cents,
}

pub struct AdjustmentService {
    accounts: Arc<AccountService>,
    ledger: Arc<LedgerService>,
}

impl AdjustmentService {
    pub fn new(accounts: Arc<AccountService>, ledger: Arc<LedgerService>) -> Self {
        Self { accounts, ledger }
    }

    /// Records the gap between `actual_balance` and the account's balance
    /// as an adjustment dated today.
    ///
    /// ```ignore
    /// let request = ReconcileBalance { account_id, actual_balance: Cents::new(15_230) };
    /// adjustments.reconcile(request, origin).await?;
    /// ```
    pub async fn reconcile(
        &self,
        request: ReconcileBalance,
        origin: EntryOrigin,
    ) -> AppResult<LedgerEntry> {
        let current = self.accounts.balance_of(request.account_id).await?;
        let gap = request.actual_balance - current;
        if gap == Cents::ZERO {
            let expected = format!("a balance other than the current {} cents", current.value());
            return Err(AppError::invalid(
                "actual balance",
                request.actual_balance.value(),
                expected,
            ));
        }
        self.ledger.record(adjustment(request.account_id, gap), origin).await
    }
}

/// Money found (`gap` > 0) or missing (`gap` < 0) in `account_id`.
fn adjustment(account_id: AccountId, gap: Cents) -> EntryRequest {
    let entry =
        AdjustmentEntry { account_id, amount: gap.abs(), description: String::new(), date: None };
    if gap.is_positive() {
        return EntryRequest::AdjustIn(entry);
    }
    EntryRequest::AdjustOut(entry)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use domain::EntryKind;

    use super::*;
    use crate::fakes::FakeServiceSet;
    use crate::fakes::requests::open_checking;
    use crate::services::AllowedUsers;

    async fn reconcile(
        set: &FakeServiceSet,
        account_id: AccountId,
        cents: i64,
    ) -> AppResult<LedgerEntry> {
        let request = ReconcileBalance { account_id, actual_balance: Cents::new(cents) };
        set.services.adjustments.reconcile(request, EntryOrigin::default()).await
    }

    #[tokio::test]
    async fn records_the_gap_in_either_direction() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let set = FakeServiceSet::new(today, AllowedUsers::default());
        let account = set.services.accounts.open(open_checking("Nubank", 10_000)).await.unwrap().id;
        let raise = reconcile(&set, account, 12_500).await.unwrap();
        assert_eq!((raise.kind, raise.amount), (EntryKind::AdjustIn, Cents::new(2_500)));
        let lower = reconcile(&set, account, -1_000).await.unwrap();
        assert_eq!((lower.kind, lower.amount), (EntryKind::AdjustOut, Cents::new(13_500)));
        assert_eq!(set.services.accounts.balance_of(account).await.unwrap(), Cents::new(-1_000));
    }

    #[tokio::test]
    async fn refuses_a_balance_that_already_matches() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let set = FakeServiceSet::new(today, AllowedUsers::default());
        let account = set.services.accounts.open(open_checking("Nubank", 10_000)).await.unwrap().id;
        let error = reconcile(&set, account, 10_000).await.unwrap_err();
        assert!(error.to_string().contains("10000"), "{error}");
        assert!(matches!(
            reconcile(&set, AccountId::generate(), 1).await,
            Err(AppError::NotFound { .. })
        ));
    }
}
