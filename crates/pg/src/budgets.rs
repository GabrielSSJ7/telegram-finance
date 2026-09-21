use app::model::{Budget, BudgetId, CategoryId};
use app::ports::{BudgetStore, StoreResult};
use async_trait::async_trait;
use chrono::NaiveDate;
use domain::Cents;
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::store_error;

struct BudgetRow {
    id: Uuid,
    category_id: Uuid,
    limit_cents: i64,
}

impl From<BudgetRow> for Budget {
    fn from(row: BudgetRow) -> Self {
        Budget {
            id: BudgetId(row.id),
            category_id: CategoryId(row.category_id),
            limit: Cents::new(row.limit_cents),
        }
    }
}

#[async_trait]
impl BudgetStore for PgStore {
    async fn set_budget(&self, category: CategoryId, limit: Cents) -> StoreResult<Budget> {
        let row = sqlx::query_as!(
            BudgetRow,
            "insert into category_budgets (category_id, limit_cents) values ($1, $2)
             on conflict (category_id) do update set limit_cents = excluded.limit_cents
             returning id, category_id, limit_cents",
            category.0,
            limit.value(),
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.into())
    }

    async fn remove_budget(&self, category: CategoryId) -> StoreResult<bool> {
        let result =
            sqlx::query!("delete from category_budgets where category_id = $1", category.0)
                .execute(self.pool())
                .await
                .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn list_budgets(&self) -> StoreResult<Vec<Budget>> {
        let rows = sqlx::query_as!(
            BudgetRow,
            "select id, category_id, limit_cents from category_budgets order by created_at"
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        Ok(rows.into_iter().map(Budget::from).collect())
    }

    async fn claim_budget_alert(
        &self,
        budget: BudgetId,
        cycle_start: NaiveDate,
        threshold: u8,
    ) -> StoreResult<bool> {
        let claimed = sqlx::query_scalar!(
            "insert into budget_alerts (budget_id, cycle_start, threshold_pct) values ($1, $2, $3)
             on conflict do nothing returning budget_id",
            budget.0,
            cycle_start,
            i16::from(threshold),
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        Ok(claimed.is_some())
    }
}
