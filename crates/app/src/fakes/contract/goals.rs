use chrono::Utc;
use domain::{AccountKind, Cents};

use super::day;
use crate::model::{Goal, GoalId, GoalTarget, NewAccount};
use crate::services::StorePorts;

async fn house_goal(stores: &StorePorts, target: GoalTarget) -> Goal {
    let (name, initial_balance) = ("Meta Casa".to_owned(), Cents::new(20));
    let pot = NewAccount { name, kind: AccountKind::Pot, initial_balance, opened_on: day(1, 1) };
    stores.goals.create_goal(pot, target).await.unwrap()
}

pub async fn goal_create_and_find(stores: StorePorts) {
    let target = GoalTarget { target: Cents::new(100), target_date: Some(day(12, 31)) };
    let goal = house_goal(&stores, target).await;
    assert_eq!(
        (goal.pot.kind, goal.pot.initial_balance, goal.target),
        (AccountKind::Pot, Cents::new(20), target)
    );
    assert_eq!(stores.goals.find_goal(goal.id).await.unwrap().as_ref(), Some(&goal));
    assert!(stores.goals.list_goals().await.unwrap().iter().any(|row| row.id == goal.id));
    assert_eq!(stores.goals.find_goal(GoalId::generate()).await.unwrap(), None);
}

pub async fn goal_update_and_hide_when_archived(stores: StorePorts) {
    let goal = house_goal(&stores, GoalTarget { target: Cents::new(100), target_date: None }).await;
    let raised = GoalTarget { target: Cents::new(500), target_date: None };
    let updated = stores.goals.update_goal_target(goal.id, raised).await.unwrap().unwrap();
    assert_eq!(updated.target, raised);
    stores.accounts.archive_account(goal.pot.id, Utc::now()).await.unwrap();
    assert!(!stores.goals.list_goals().await.unwrap().iter().any(|row| row.id == goal.id));
    assert_eq!(stores.goals.update_goal_target(GoalId::generate(), raised).await.unwrap(), None);
}
