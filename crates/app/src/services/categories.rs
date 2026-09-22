use std::sync::Arc;

use super::text_rules::clean_name;
use crate::model::{Category, CategoryId, CategoryKind, NewCategory};
use crate::ports::{CategoryStore, Clock};
use crate::{AppError, AppResult};

pub const MAX_CATEGORY_NAME_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategory {
    pub name: String,
    pub kind: CategoryKind,
    pub emoji: Option<String>,
    /// Counts in the basic cost of living; expense categories only.
    pub essential: bool,
}

pub struct CategoryService {
    categories: Arc<dyn CategoryStore>,
    clock: Arc<dyn Clock>,
}

impl CategoryService {
    pub fn new(categories: Arc<dyn CategoryStore>, clock: Arc<dyn Clock>) -> Self {
        Self { categories, clock }
    }

    /// Creates a category; names are unique per kind, ignoring case.
    ///
    /// ```ignore
    /// let request = CreateCategory { name: "pets".into(), kind: CategoryKind::Expense,
    ///     emoji: Some("🐶".into()), essential: false };
    /// categories.create(request).await?;
    /// ```
    pub async fn create(&self, request: CreateCategory) -> AppResult<Category> {
        ensure_essential_is_expense(request.kind, request.essential)?;
        let category = NewCategory {
            name: clean_name("category name", &request.name, MAX_CATEGORY_NAME_CHARS)?,
            kind: request.kind,
            emoji: request.emoji.map(|text| text.trim().to_owned()).filter(|text| !text.is_empty()),
            essential: request.essential,
        };
        Ok(self.categories.create_category(category).await?)
    }

    /// Marks an active expense category as part of the basic cost of
    /// living, or clears the mark.
    pub async fn set_essential(&self, id: CategoryId, essential: bool) -> AppResult<Category> {
        self.require_kind(id, CategoryKind::Expense).await?;
        let updated = self.categories.set_category_essential(id, essential).await?;
        updated.ok_or_else(|| AppError::not_found("active category", id))
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

fn ensure_essential_is_expense(kind: CategoryKind, essential: bool) -> AppResult<()> {
    if essential && kind != CategoryKind::Expense {
        return Err(AppError::invalid("essential", kind.as_str(), "an expense category"));
    }
    Ok(())
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

    fn request(name: &str, kind: CategoryKind, emoji: Option<&str>) -> CreateCategory {
        CreateCategory { name: name.into(), kind, emoji: emoji.map(Into::into), essential: false }
    }

    fn names(categories: &[Category]) -> Vec<&str> {
        categories.iter().map(|category| category.name.as_str()).collect()
    }

    #[tokio::test]
    async fn list_filters_by_kind_and_sorts_by_name() {
        let service = service();
        service.create(request("mercado", CategoryKind::Expense, None)).await.unwrap();
        service.create(request("casa", CategoryKind::Expense, None)).await.unwrap();
        service.create(request("salário", CategoryKind::Income, None)).await.unwrap();
        let expenses = service.list(Some(CategoryKind::Expense)).await.unwrap();
        assert_eq!(names(&expenses), vec!["casa", "mercado"]);
        assert_eq!(service.list(None).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn create_trims_emoji_and_drops_blank() {
        let service = service();
        let cart =
            service.create(request("mercado", CategoryKind::Expense, Some(" 🛒 "))).await.unwrap();
        let blank = service.create(request("casa", CategoryKind::Expense, Some(""))).await.unwrap();
        assert_eq!((cart.emoji.as_deref(), blank.emoji), (Some("🛒"), None));
    }

    #[tokio::test]
    async fn require_kind_rejects_wrong_kind_and_archived() {
        let service = service();
        let salary = service.create(request("salário", CategoryKind::Income, None)).await.unwrap();
        let error = service.require_kind(salary.id, CategoryKind::Expense).await.unwrap_err();
        assert!(error.to_string().contains("kind expense"), "{error}");
        service.archive(salary.id).await.unwrap();
        assert!(matches!(
            service.require_kind(salary.id, CategoryKind::Income).await,
            Err(AppError::NotFound { .. })
        ));
    }

    #[tokio::test]
    async fn only_expense_categories_are_essential() {
        let service = service();
        let rent = service.create(request("aluguel", CategoryKind::Expense, None)).await.unwrap();
        assert!(service.set_essential(rent.id, true).await.unwrap().essential);
        let salary = service.create(request("salário", CategoryKind::Income, None)).await.unwrap();
        assert!(service.set_essential(salary.id, true).await.is_err());
        let essential_income =
            CreateCategory { essential: true, ..request("extra", CategoryKind::Income, None) };
        let error = service.create(essential_income).await.unwrap_err().to_string();
        assert!(error.contains("income") && error.contains("an expense category"), "{error}");
    }
}
