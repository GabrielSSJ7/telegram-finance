use app::model::{Category, CategoryKind};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    #[schema(example = "mercado")]
    pub name: String,
    #[schema(value_type = String, example = "expense")]
    pub kind: CategoryKind,
    #[schema(example = "🛒")]
    pub emoji: Option<String>,
    pub archived: bool,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        let Category { id, name, kind, emoji, archived } = category;
        Self { id: id.0, name, kind, emoji, archived }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryBody {
    #[schema(example = "farmácia")]
    pub name: String,
    #[schema(value_type = String, example = "expense")]
    pub kind: CategoryKind,
    pub emoji: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCategoriesQuery {
    /// `expense` or `income`; omit for both.
    #[param(value_type = Option<String>)]
    pub kind: Option<CategoryKind>,
}
