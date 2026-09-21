use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::StoreResult;
use crate::model::{Category, CategoryId, NewCategory};

#[async_trait]
pub trait CategoryStore: Send + Sync {
    async fn create_category(&self, category: NewCategory) -> StoreResult<Category>;
    async fn list_categories(&self, include_archived: bool) -> StoreResult<Vec<Category>>;
    async fn find_category(&self, id: CategoryId) -> StoreResult<Option<Category>>;
    async fn archive_category(&self, id: CategoryId, at: DateTime<Utc>) -> StoreResult<bool>;
}
