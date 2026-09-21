use async_trait::async_trait;

use super::StoreResult;
use crate::model::{Goal, GoalId, GoalTarget, NewAccount};

#[async_trait]
pub trait GoalStore: Send + Sync {
    /// Creates the pot account and its goal in one transaction.
    async fn create_goal(&self, pot: NewAccount, target: GoalTarget) -> StoreResult<Goal>;
    /// Goals whose pot is not archived.
    async fn list_goals(&self) -> StoreResult<Vec<Goal>>;
    async fn find_goal(&self, id: GoalId) -> StoreResult<Option<Goal>>;
    async fn update_goal_target(&self, id: GoalId, target: GoalTarget)
    -> StoreResult<Option<Goal>>;
}
