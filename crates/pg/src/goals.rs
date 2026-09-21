use app::model::{Goal, GoalId, GoalTarget, NewAccount};
use app::ports::{GoalStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::Cents;
use uuid::Uuid;

use crate::PgStore;
use crate::accounts::AccountRow;
use crate::error_mapping::store_error;

struct GoalRow {
    goal_id: Uuid,
    target_cents: i64,
    target_date: Option<NaiveDate>,
    id: Uuid,
    name: String,
    kind: String,
    initial_balance_cents: i64,
    opened_on: NaiveDate,
    archived_at: Option<DateTime<Utc>>,
}

impl GoalRow {
    fn into_goal(self) -> StoreResult<Goal> {
        let target =
            GoalTarget { target: Cents::new(self.target_cents), target_date: self.target_date };
        let pot = AccountRow {
            id: self.id,
            name: self.name,
            kind: self.kind,
            initial_balance_cents: self.initial_balance_cents,
            opened_on: self.opened_on,
            archived_at: self.archived_at,
        };
        Ok(Goal { id: GoalId(self.goal_id), pot: pot.into_account()?, target })
    }
}

#[async_trait]
impl GoalStore for PgStore {
    async fn create_goal(&self, pot: NewAccount, target: GoalTarget) -> StoreResult<Goal> {
        let mut transaction = self.pool().begin().await.map_err(store_error)?;
        let row = sqlx::query_file_as!(
            GoalRow,
            "queries/create_goal.sql",
            pot.name,
            pot.kind.as_str(),
            pot.initial_balance.value(),
            pot.opened_on,
            target.target.value(),
            target.target_date,
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(store_error)?;
        transaction.commit().await.map_err(store_error)?;
        row.into_goal()
    }

    async fn list_goals(&self) -> StoreResult<Vec<Goal>> {
        let rows = sqlx::query_as!(
            GoalRow,
            "select goals.id as goal_id, goals.target_cents, goals.target_date, accounts.id, accounts.name,
                    accounts.kind, accounts.initial_balance_cents, accounts.opened_on, accounts.archived_at
             from goals join accounts on accounts.id = goals.account_id
             where accounts.archived_at is null
             order by goals.created_at, goals.id",
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows.into_iter().map(GoalRow::into_goal).collect()
    }

    async fn find_goal(&self, id: GoalId) -> StoreResult<Option<Goal>> {
        let row = sqlx::query_as!(
            GoalRow,
            "select goals.id as goal_id, goals.target_cents, goals.target_date, accounts.id, accounts.name,
                    accounts.kind, accounts.initial_balance_cents, accounts.opened_on, accounts.archived_at
             from goals join accounts on accounts.id = goals.account_id
             where goals.id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(GoalRow::into_goal).transpose()
    }

    async fn update_goal_target(
        &self,
        id: GoalId,
        target: GoalTarget,
    ) -> StoreResult<Option<Goal>> {
        let updated = sqlx::query!(
            "update goals set target_cents = $2, target_date = $3 where id = $1",
            id.0,
            target.target.value(),
            target.target_date,
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        if updated.rows_affected() == 0 {
            return Ok(None);
        }
        self.find_goal(id).await
    }
}
