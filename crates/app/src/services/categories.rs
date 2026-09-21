use std::sync::Arc;

use super::text_rules::clean_name;
use crate::model::{Category, CategoryId, CategoryKind, NewCategory};
use crate::ports::{CategoryStore, Clock};
use crate::{AppError, AppResult};

pub const MAX_CATEGORY_NAME_CHARS: usize = 32;

pub struct CategoryService {
    categories: Arc<dyn CategoryStore>,
    clock: Arc<dyn Clock>,
}

impl CategoryService {
    pub fn new(categories: Arc<dyn CategoryStore>, clock: Arc<dyn Clock>) -> Self {
        Self { categories, clock }
    }

    /// Creates a category; names are unique per kind, ignoring case.
    pub async fn create(
        &self,
        name: &str,
        kind: CategoryKind,
        emoji: Option<String>,
    ) -> AppResult<Category> {
        let category = NewCategory {
            name: clean_name("category name", name, MAX_CATEGORY_NAME_CHARS)?,
            kind,
            emoji: emoji.map(|text| text.trim().to_owned()).filter(|text| !text.is_empty()),
        };
        Ok(self.categories.create_category(category).await?)
    }

    /// Active categories of `kind` (or all kinds), sorted by name.
    pub async fn list(&self, kind: Option<CategoryKind>) -> AppResult<Vec<Category>> {
        let mut found = self.categories.list_categories(false).await?;
        found.retain(|category| kind.is_none_or(|wanted| category.kind == wanted));
        found.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(found)
    }

    pub async fn archive(&self, id: CategoryId) -> AppResult<()> {
        if !self.categories.archive_category(id, self.clock.now()).await? {
            return Err(AppError::not_found("active category", id));
        }
        Ok(())
    }

    /// The category if it is active and of the `expected` kind.
    pub async fn require_kind(
        &self,
        id: CategoryId,
        expected: CategoryKind,
    ) -> AppResult<Category> {
        let category = match self.categories.find_category(id).await? {
            Some(category) if !category.archived => category,
            _ => return Err(AppError::not_found("active category", id)),
        };
        if category.kind != expected {
            let expected_text = format!("a category of kind {}", expected.as_str());
            return Err(AppError::invalid("category", &category.name, expected_text));
        }
        Ok(category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FixedClock, InMemoryStore};
    use chrono::NaiveDate;

    fn service() -> CategoryService {
        let clock = FixedClock::at_local_noon(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap());
        CategoryService::new(Arc::new(InMemoryStore::new()), Arc::new(clock))
    }

    fn names(categories: &[Category]) -> Vec<&str> {
        categories.iter().map(|category| category.name.as_str()).collect()
    }

    #[tokio::test]
    async fn list_filters_by_kind_and_sorts_by_name() {
        let service = service();
        service.create("mercado", CategoryKind::Expense, None).await.unwrap();
        service.create("casa", CategoryKind::Expense, None).await.unwrap();
        service.create("salário", CategoryKind::Income, None).await.unwrap();
        let expenses = service.list(Some(CategoryKind::Expense)).await.unwrap();
        assert_eq!(names(&expenses), vec!["casa", "mercado"]);
        assert_eq!(service.list(None).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn create_trims_emoji_and_drops_blank() {
        let service = service();
        let cart =
            service.create("mercado", CategoryKind::Expense, Some(" 🛒 ".into())).await.unwrap();
        let blank =
            service.create("casa", CategoryKind::Expense, Some(String::new())).await.unwrap();
        assert_eq!((cart.emoji.as_deref(), blank.emoji), (Some("🛒"), None));
    }

    #[tokio::test]
    async fn require_kind_rejects_wrong_kind_and_archived() {
        let service = service();
        let salary = service.create("salário", CategoryKind::Income, None).await.unwrap();
        let error = service.require_kind(salary.id, CategoryKind::Expense).await.unwrap_err();
        assert!(error.to_string().contains("kind expense"), "{error}");
        service.archive(salary.id).await.unwrap();
        assert!(matches!(
            service.require_kind(salary.id, CategoryKind::Income).await,
            Err(AppError::NotFound { .. })
        ));
    }
}
