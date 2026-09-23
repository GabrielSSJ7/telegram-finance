use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{InMemoryStore, same_name, unique_violation};
use crate::model::{Category, CategoryEdit, CategoryId, EmojiChange, NewCategory};
use crate::ports::{CategoryStore, StoreResult};

fn apply_category_edit(row: &mut Category, edit: CategoryEdit) {
    if let Some(name) = edit.name {
        row.name = name;
    }
    match edit.emoji {
        EmojiChange::Keep => {}
        EmojiChange::Clear => row.emoji = None,
        EmojiChange::Set(emoji) => row.emoji = Some(emoji),
    }
}

fn category_from(new: NewCategory) -> Category {
    Category {
        id: CategoryId::generate(),
        name: new.name,
        kind: new.kind,
        emoji: new.emoji,
        archived: false,
        essential: new.essential,
    }
}

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
        let created = category_from(category);
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

    async fn update_category(
        &self,
        id: CategoryId,
        edit: CategoryEdit,
    ) -> StoreResult<Option<Category>> {
        let mut state = self.lock();
        let kind = state.categories.iter().find(|row| row.id == id).map(|row| row.kind);
        let clash = |row: &&Category| {
            edit.name.as_ref().is_some_and(|name| {
                row.id != id
                    && !row.archived
                    && Some(row.kind) == kind
                    && same_name(&row.name, name)
            })
        };
        if state.categories.iter().any(|row| clash(&row)) {
            return Err(unique_violation("categories_active_name"));
        }
        let Some(row) = state.categories.iter_mut().find(|row| row.id == id && !row.archived)
        else {
            return Ok(None);
        };
        apply_category_edit(row, edit);
        Ok(Some(row.clone()))
    }

    async fn set_category_essential(
        &self,
        id: CategoryId,
        essential: bool,
    ) -> StoreResult<Option<Category>> {
        let mut state = self.lock();
        let Some(row) = state.categories.iter_mut().find(|row| row.id == id && !row.archived)
        else {
            return Ok(None);
        };
        row.essential = essential;
        Ok(Some(row.clone()))
    }
}
