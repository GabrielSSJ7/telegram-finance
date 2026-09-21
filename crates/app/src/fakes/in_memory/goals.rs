use async_trait::async_trait;

use super::{GoalRow, InMemoryStore, MemoryState, account_from, name_taken, unique_violation};
use crate::model::{Goal, GoalId, GoalTarget, NewAccount};
use crate::ports::{GoalStore, StoreResult};

#[async_trait]
impl GoalStore for InMemoryStore {
    async fn create_goal(&self, pot: NewAccount, target: GoalTarget) -> StoreResult<Goal> {
        let mut state = self.lock();
        if name_taken(&state, &pot.name) {
            return Err(unique_violation("accounts_active_name"));
        }
        let account = account_from(pot);
        let row = GoalRow { id: GoalId::generate(), account_id: account.id, target };
        let goal = Goal { id: row.id, pot: account.clone(), target };
        state.accounts.push(account);
        state.goals.push(row);
        Ok(goal)
    }

    async fn list_goals(&self) -> StoreResult<Vec<Goal>> {
        let state = self.lock();
        let goals = state.goals.iter().filter_map(|row| goal_of(&state, row));
        Ok(goals.filter(|goal| !goal.pot.archived).collect())
    }

    async fn find_goal(&self, id: GoalId) -> StoreResult<Option<Goal>> {
        let state = self.lock();
        Ok(state.goals.iter().find(|row| row.id == id).and_then(|row| goal_of(&state, row)))
    }

    async fn update_goal_target(
        &self,
        id: GoalId,
        target: GoalTarget,
    ) -> StoreResult<Option<Goal>> {
        let mut state = self.lock();
        let Some(row) = state.goals.iter_mut().find(|row| row.id == id) else {
            return Ok(None);
        };
        row.target = target;
        let state = &*state;
        Ok(state.goals.iter().find(|row| row.id == id).and_then(|row| goal_of(state, row)))
    }
}

fn goal_of(state: &MemoryState, row: &GoalRow) -> Option<Goal> {
    let pot = state.accounts.iter().find(|account| account.id == row.account_id)?;
    Some(Goal { id: row.id, pot: pot.clone(), target: row.target })
}
