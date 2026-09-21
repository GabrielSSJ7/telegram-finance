use domain::Cents;

use super::{day, new_category};
use crate::model::CategoryKind;
use crate::services::StorePorts;

pub async fn budget_set_replace_list_remove(stores: StorePorts) {
    let category = new_category(&stores, "orçamento", CategoryKind::Expense).await.id;
    let created = stores.budgets.set_budget(category, Cents::new(50_000)).await.unwrap();
    let replaced = stores.budgets.set_budget(category, Cents::new(80_000)).await.unwrap();
    assert_eq!((replaced.id, replaced.limit), (created.id, Cents::new(80_000)));
    assert_eq!(stores.budgets.list_budgets().await.unwrap(), vec![replaced]);
    assert!(stores.budgets.remove_budget(category).await.unwrap());
    assert!(!stores.budgets.remove_budget(category).await.unwrap());
    assert!(stores.budgets.list_budgets().await.unwrap().is_empty());
}

pub async fn budget_alert_claimed_once_per_cycle_and_threshold(stores: StorePorts) {
    let category = new_category(&stores, "alerta", CategoryKind::Expense).await.id;
    let budget = stores.budgets.set_budget(category, Cents::new(1_000)).await.unwrap().id;
    assert!(stores.budgets.claim_budget_alert(budget, day(3, 1), 80).await.unwrap());
    assert!(!stores.budgets.claim_budget_alert(budget, day(3, 1), 80).await.unwrap());
    assert!(stores.budgets.claim_budget_alert(budget, day(3, 1), 100).await.unwrap());
    assert!(stores.budgets.claim_budget_alert(budget, day(4, 1), 80).await.unwrap());
}
