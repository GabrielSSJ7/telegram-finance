use app::model::{Category, CategoryId, CategoryKind, NewCategory};
use app::ports::{CategoryStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::{corrupt, store_error};

struct CategoryRow {
    id: Uuid,
    name: String,
    kind: String,
    emoji: Option<String>,
    archived_at: Option<DateTime<Utc>>,
    essential: bool,
}

impl CategoryRow {
    fn into_category(self) -> StoreResult<Category> {
        let kind =
            self.kind.parse::<CategoryKind>().map_err(|error| corrupt("categories.kind", error))?;
        let archived = self.archived_at.is_some();
        Ok(Category {
            id: CategoryId(self.id),
            name: self.name,
            kind,
            emoji: self.emoji,
            archived,
            essential: self.essential,
        })
    }
}

#[async_trait]
impl CategoryStore for PgStore {
    async fn create_category(&self, category: NewCategory) -> StoreResult<Category> {
        let row = sqlx::query_as!(
            CategoryRow,
            "insert into categories (name, kind, emoji, essential) values ($1, $2, $3, $4)
             returning id, name, kind, emoji, archived_at, essential",
            category.name,
            category.kind.as_str(),
            category.emoji,
            category.essential,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_category()
    }

    async fn list_categories(&self, include_archived: bool) -> StoreResult<Vec<Category>> {
        let rows = sqlx::query_as!(
            CategoryRow,
            "select id, name, kind, emoji, archived_at, essential from categories
             where $1 or archived_at is null order by kind, name",
            include_archived,
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows.into_iter().map(CategoryRow::into_category).collect()
    }

    async fn find_category(&self, id: CategoryId) -> StoreResult<Option<Category>> {
        let row = sqlx::query_as!(
            CategoryRow,
            "select id, name, kind, emoji, archived_at, essential from categories where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(CategoryRow::into_category).transpose()
    }

    async fn archive_category(&self, id: CategoryId, at: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query!(
            "update categories set archived_at = $2 where id = $1 and archived_at is null",
            id.0,
            at
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn set_category_essential(
        &self,
        id: CategoryId,
        essential: bool,
    ) -> StoreResult<Option<Category>> {
        let row = sqlx::query_as!(
            CategoryRow,
            "update categories set essential = $2 where id = $1 and archived_at is null
             returning id, name, kind, emoji, archived_at, essential",
            id.0,
            essential,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(CategoryRow::into_category).transpose()
    }
}
