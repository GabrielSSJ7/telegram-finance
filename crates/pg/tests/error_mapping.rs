// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Postgres-only failure paths that the in-memory fake does not model.

use app::model::{AccountId, NewEntry};
use app::ports::{EntryStore, SettingsStore, StoreError};
use chrono::NaiveDate;
use domain::{Cents, EntryKind};
use pg::PgStore;
use sqlx::PgPool;

#[sqlx::test(migrator = "pg::MIGRATOR")]
async fn unknown_account_is_missing_reference(pool: PgPool) {
    let entry = NewEntry {
        kind: EntryKind::AdjustIn,
        amount: Cents::new(1),
        description: String::new(),
        category_id: None,
        account_id: Some(AccountId::generate()),
        counter_account_id: None,
        accounting_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        created_by: None,
    };
    let error = PgStore::new(pool).record_entry(entry, None).await.unwrap_err();
    assert!(
        matches!(error, StoreError::MissingReference { ref constraint } if constraint.contains("account")),
        "{error:?}"
    );
}

#[sqlx::test(migrator = "pg::MIGRATOR")]
async fn closed_pool_is_backend_error(pool: PgPool) {
    let store = PgStore::new(pool.clone());
    pool.close().await;
    let error = store.load_settings().await;
    assert!(matches!(error, Err(StoreError::Backend(_))), "{error:?}");
}
