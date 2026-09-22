use app::model::{Category, CategoryKind};
use app::services::CreateCategory;
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
    /// Part of the basic cost of living (expense categories only).
    pub essential: bool,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        let Category { id, name, kind, emoji, archived, essential } = category;
        Self { id: id.0, name, kind, emoji, archived, essential }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryBody {
    #[schema(example = "farmácia")]
    pub name: String,
    #[schema(value_type = String, example = "expense")]
    pub kind: CategoryKind,
    pub emoji: Option<String>,
    /// Defaults to false; only expense categories can be essential.
    #[serde(default)]
    pub essential: bool,
}

impl From<CreateCategoryBody> for CreateCategory {
    fn from(body: CreateCategoryBody) -> Self {
        Self { name: body.name, kind: body.kind, emoji: body.emoji, essential: body.essential }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCategoryBody {
    pub essential: bool,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCategoriesQuery {
    /// `expense` or `income`; omit for both.
    #[param(value_type = Option<String>)]
    pub kind: Option<CategoryKind>,
}
