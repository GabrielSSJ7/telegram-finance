use async_trait::async_trait;
use chrono::NaiveDate;
use domain::Cents;

use super::InMemoryStore;
use crate::model::{Budget, BudgetId, CategoryId};
use crate::ports::{BudgetStore, StoreResult};

#[async_trait]
impl BudgetStore for InMemoryStore {
    async fn set_budget(&self, category: CategoryId, limit: Cents) -> StoreResult<Budget> {
        let mut state = self.lock();
        if let Some(existing) =
            state.budgets.iter_mut().find(|budget| budget.category_id == category)
        {
            existing.limit = limit;
            return Ok(*existing);
        }
        let budget = Budget { id: BudgetId::generate(), category_id: category, limit };
        state.budgets.push(budget);
        Ok(budget)
    }

    async fn remove_budget(&self, category: CategoryId) -> StoreResult<bool> {
        let mut state = self.lock();
        let before = state.budgets.len();
        state.budgets.retain(|budget| budget.category_id != category);
        Ok(state.budgets.len() < before)
    }

    async fn list_budgets(&self) -> StoreResult<Vec<Budget>> {
        Ok(self.lock().budgets.clone())
    }

    async fn claim_budget_alert(
        &self,
        budget: BudgetId,
        cycle_start: NaiveDate,
        threshold: u8,
    ) -> StoreResult<bool> {
        Ok(self.lock().budget_alerts.insert((budget, cycle_start, threshold)))
    }
}
