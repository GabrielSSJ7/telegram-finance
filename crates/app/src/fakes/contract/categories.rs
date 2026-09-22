use chrono::Utc;

use super::new_category;
use crate::model::{CategoryId, CategoryKind, NewCategory};
use crate::ports::StoreError;
use crate::services::StorePorts;

pub async fn category_create_find_archive(stores: StorePorts) {
    let category = new_category(&stores, "contrato-categoria", CategoryKind::Expense).await;
    assert_eq!(
        stores.categories.find_category(category.id).await.unwrap().as_ref(),
        Some(&category)
    );
    assert!(stores.categories.list_categories(false).await.unwrap().contains(&category));
    assert!(stores.categories.archive_category(category.id, Utc::now()).await.unwrap());
    assert!(!stores.categories.archive_category(category.id, Utc::now()).await.unwrap());
    let active = stores.categories.list_categories(false).await.unwrap();
    assert!(!active.iter().any(|row| row.id == category.id));
    let all = stores.categories.list_categories(true).await.unwrap();
    assert!(all.iter().any(|row| row.id == category.id && row.archived));
    assert_eq!(stores.categories.find_category(CategoryId::generate()).await.unwrap(), None);
}

pub async fn category_names_unique_per_kind(stores: StorePorts) {
    new_category(&stores, "Duplicada", CategoryKind::Expense).await;
    let clash = NewCategory {
        name: "duplicada".into(),
        kind: CategoryKind::Expense,
        emoji: None,
        essential: false,
    };
    let error = stores.categories.create_category(clash).await.unwrap_err();
    assert!(matches!(error, StoreError::UniqueViolation { .. }), "{error:?}");
    let other_kind = NewCategory {
        name: "duplicada".into(),
        kind: CategoryKind::Income,
        emoji: Some("✨".into()),
        essential: false,
    };
    let created = stores.categories.create_category(other_kind).await.unwrap();
    assert_eq!(created.emoji.as_deref(), Some("✨"));
}

pub async fn category_essential_flag(stores: StorePorts) {
    let rent = NewCategory {
        name: "aluguel-contrato".into(),
        kind: CategoryKind::Expense,
        emoji: None,
        essential: true,
    };
    let created = stores.categories.create_category(rent).await.unwrap();
    assert!(created.essential);
    let cleared = stores.categories.set_category_essential(created.id, false).await.unwrap();
    assert_eq!(cleared.map(|category| category.essential), Some(false));
    let found = stores.categories.find_category(created.id).await.unwrap();
    assert!(!found.unwrap().essential);
    let missing = stores.categories.set_category_essential(CategoryId::generate(), true).await;
    assert_eq!(missing.unwrap(), None);
}
