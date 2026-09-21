use chrono::NaiveDate;
use domain::{AccountKind, Cents};

use super::goals::{CreateGoal, PotMove};
use super::ledger::EntryOrigin;
use super::{AllowedUsers, OpenAccount};
use crate::AppError;
use crate::fakes::FakeServiceSet;
use crate::model::{AccountId, BalanceSheet, GoalId, GoalTarget};

async fn with_checking() -> (FakeServiceSet, AccountId) {
    let set =
        FakeServiceSet::new(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(), AllowedUsers::default());
    let (name, initial_balance) = ("Nubank".to_owned(), Cents::new(500_000));
    let open = OpenAccount { name, kind: AccountKind::Checking, initial_balance, opened_on: None };
    let checking = set.services.accounts.open(open).await.unwrap().id;
    (set, checking)
}

fn house_goal() -> CreateGoal {
    let (target, already_saved) = (Cents::new(10_000_000), Cents::new(2_000_000));
    CreateGoal { name: "Casa própria".into(), target, target_date: None, already_saved }
}

fn pot_move(goal_id: GoalId, account_id: AccountId, cents: i64) -> PotMove {
    PotMove { goal_id, account_id, amount: Cents::new(cents), description: String::new() }
}

async fn sheet(set: &FakeServiceSet) -> BalanceSheet {
    set.services.accounts.balance_sheet().await.unwrap()
}

#[tokio::test]
async fn already_saved_counts_as_progress_not_income() {
    let (set, _) = with_checking().await;
    let goal = set.services.goals.create(house_goal()).await.unwrap();
    assert_eq!(goal.pot.kind, AccountKind::Pot);
    let progress = set.services.goals.list_progress().await.unwrap();
    assert_eq!((progress[0].saved, progress[0].progress_bp), (Cents::new(2_000_000), 2000));
    assert_eq!(progress[0].remaining, Cents::new(8_000_000));
    let position = sheet(&set).await.position;
    assert_eq!(position.available, Cents::new(500_000));
    assert_eq!(position.reserved_in_pots, Cents::new(2_000_000));
}

#[tokio::test]
async fn deposit_and_withdraw_move_money_through_pot() {
    let (set, checking) = with_checking().await;
    let goals = &set.services.goals;
    let goal = goals.create(house_goal()).await.unwrap();
    goals.deposit(pot_move(goal.id, checking, 100_000), EntryOrigin::default()).await.unwrap();
    goals.withdraw(pot_move(goal.id, checking, 50_000), EntryOrigin::default()).await.unwrap();
    let position = sheet(&set).await.position;
    assert_eq!(position.available, Cents::new(450_000));
    assert_eq!(position.reserved_in_pots, Cents::new(2_050_000));
}

#[tokio::test]
async fn withdraw_cannot_exceed_pot_balance() {
    let (set, checking) = with_checking().await;
    let goal = set.services.goals.create(house_goal()).await.unwrap();
    let request = pot_move(goal.id, checking, 2_000_001);
    let error = set.services.goals.withdraw(request, EntryOrigin::default()).await.unwrap_err();
    assert!(error.to_string().contains("2000000"), "{error}");
}

#[tokio::test]
async fn rejects_zero_target_and_unknown_goal() {
    let (set, checking) = with_checking().await;
    let goals = &set.services.goals;
    let zero = CreateGoal { target: Cents::ZERO, ..house_goal() };
    assert!(matches!(goals.create(zero).await, Err(AppError::Invalid { .. })));
    let request = pot_move(GoalId::generate(), checking, 1);
    let unknown = goals.deposit(request, EntryOrigin::default()).await;
    assert!(matches!(unknown, Err(AppError::NotFound { .. })));
}

#[tokio::test]
async fn update_target_replaces_target() {
    let (set, _) = with_checking().await;
    let goal = set.services.goals.create(house_goal()).await.unwrap();
    let target = GoalTarget { target: Cents::new(20_000_000), target_date: None };
    let updated = set.services.goals.update_target(goal.id, target).await.unwrap();
    assert_eq!(updated.target, target);
}
