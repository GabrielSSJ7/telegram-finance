use chrono::NaiveDate;
use domain::Cents;

use super::AllowedUsers;
use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use crate::AppError;
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::open_checking;
use crate::model::{AccountId, CategoryId, CategoryKind};

struct Scene {
    set: FakeServiceSet,
    checking: AccountId,
    groceries: CategoryId,
}

async fn scene() -> Scene {
    let set =
        FakeServiceSet::new(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(), AllowedUsers::default());
    let checking = set.services.accounts.open(open_checking("Nubank", 1_000_000)).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    Scene { set, checking, groceries }
}

impl Scene {
    async fn spend(&self, cents: i64) {
        let (amount, description) = (Cents::new(cents), String::new());
        let entry = AccountEntry {
            account_id: self.checking,
            category_id: self.groceries,
            amount,
            description,
            date: None,
        };
        self.set
            .services
            .ledger
            .record(EntryRequest::Expense(entry), EntryOrigin::default())
            .await
            .unwrap();
    }

    async fn thresholds(&self) -> Vec<u8> {
        self.set
            .services
            .budgets
            .new_alerts()
            .await
            .unwrap()
            .into_iter()
            .map(|alert| alert.threshold)
            .collect()
    }
}

#[tokio::test]
async fn alerts_fire_once_per_threshold() {
    let scene = scene().await;
    scene.set.services.budgets.set(scene.groceries, Cents::new(10_000)).await.unwrap();
    scene.spend(7_000).await;
    assert!(scene.thresholds().await.is_empty());
    scene.spend(1_000).await;
    assert_eq!(scene.thresholds().await, vec![80]);
    assert!(scene.thresholds().await.is_empty());
    scene.spend(5_000).await;
    assert_eq!(scene.thresholds().await, vec![100]);
}

#[tokio::test]
async fn jumping_past_both_reports_only_the_limit() {
    let scene = scene().await;
    scene.set.services.budgets.set(scene.groceries, Cents::new(10_000)).await.unwrap();
    scene.spend(15_000).await;
    let alerts = scene.set.services.budgets.new_alerts().await.unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!((alerts[0].threshold, alerts[0].status.used_bp), (100, 15_000));
}

#[tokio::test]
async fn set_validates_kind_and_limit() {
    let scene = scene().await;
    let budgets = &scene.set.services.budgets;
    let salary = scene.set.store.seed_category("salário", CategoryKind::Income);
    assert!(matches!(budgets.set(salary, Cents::new(1)).await, Err(AppError::Invalid { .. })));
    let zero = budgets.set(scene.groceries, Cents::ZERO).await;
    assert!(matches!(zero, Err(AppError::Invalid { .. })));
}

#[tokio::test]
async fn statuses_sorted_by_use_and_removal() {
    let scene = scene().await;
    let budgets = &scene.set.services.budgets;
    let pets = scene.set.store.seed_category("pets", CategoryKind::Expense);
    budgets.set(pets, Cents::new(100)).await.unwrap();
    budgets.set(scene.groceries, Cents::new(100)).await.unwrap();
    scene.spend(50).await;
    let statuses = budgets.statuses(budgets.current_cycle().await.unwrap()).await.unwrap();
    let names: Vec<String> = statuses.into_iter().map(|status| status.category.name).collect();
    assert_eq!(names, vec!["mercado", "pets"]);
    budgets.remove(pets).await.unwrap();
    assert!(matches!(budgets.remove(pets).await, Err(AppError::NotFound { .. })));
}
