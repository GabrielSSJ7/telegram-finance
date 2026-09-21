use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{InMemoryStore, same_name, unique_violation};
use crate::model::{Category, CategoryId, NewCategory};
use crate::ports::{CategoryStore, StoreResult};

#[async_trait]
impl CategoryStore for InMemoryStore {
    async fn create_category(&self, category: NewCategory) -> StoreResult<Category> {
        let mut state = self.lock();
        let taken = state.categories.iter().any(|row| {
            !row.archived && row.kind == category.kind && same_name(&row.name, &category.name)
        });
        if taken {
            return Err(unique_violation("categories_active_name"));
        }
        let created = Category {
            id: CategoryId::generate(),
            name: category.name,
            kind: category.kind,
            emoji: category.emoji,
            archived: false,
        };
        state.categories.push(created.clone());
        Ok(created)
    }

    async fn list_categories(&self, include_archived: bool) -> StoreResult<Vec<Category>> {
        let state = self.lock();
        Ok(state
            .categories
            .iter()
            .filter(|row| include_archived || !row.archived)
            .cloned()
            .collect())
    }

    async fn find_category(&self, id: CategoryId) -> StoreResult<Option<Category>> {
        Ok(self.lock().categories.iter().find(|row| row.id == id).cloned())
    }

    async fn archive_category(&self, id: CategoryId, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(row) = state.categories.iter_mut().find(|row| row.id == id && !row.archived)
        else {
            return Ok(false);
        };
        row.archived = true;
        Ok(true)
    }
}
