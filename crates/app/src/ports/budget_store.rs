use async_trait::async_trait;
use chrono::NaiveDate;
use domain::Cents;

use super::StoreResult;
use crate::model::{Budget, BudgetId, CategoryId};

#[async_trait]
pub trait BudgetStore: Send + Sync {
    /// Creates or replaces the limit of `category`.
    async fn set_budget(&self, category: CategoryId, limit: Cents) -> StoreResult<Budget>;
    /// Returns false when the category had no budget.
    async fn remove_budget(&self, category: CategoryId) -> StoreResult<bool>;
    async fn list_budgets(&self) -> StoreResult<Vec<Budget>>;
    /// Records that `threshold` was announced for the cycle starting on
    /// `cycle_start`; false when it already was (alert once per cycle).
    async fn claim_budget_alert(
        &self,
        budget: BudgetId,
        cycle_start: NaiveDate,
        threshold: u8,
    ) -> StoreResult<bool>;
}
